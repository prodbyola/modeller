use crate::{backend_type::BackendType, field::FieldDefinition};
use bincode::{Decode, Encode, config};
use proc_macro2::Span;
use quote::ToTokens;
use syn::{Expr, Ident, ItemStruct, Meta};

#[derive(Encode, Decode)]
pub struct ModelDefinition {
    name: String,
    unique_together: Option<String>,
    fields: Vec<FieldDefinition>,
}

impl ModelDefinition {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[FieldDefinition] {
        &self.fields
    }

    /// Generate `CREATE TABLE` sql query for the model.
    pub fn sql_create_table(&self, bt: &BackendType) -> String {
        let table_name = &self.name;
        let mut field_sqls = self.field_sqls(bt);

        if let Some(ut_sql) = self.unique_together_sql(bt) {
            field_sqls.push(ut_sql);
        }

        format!(
            "DROP TABLE IF EXISTS {table_name};\n\
            CREATE TABLE {table_name} (\n\t{}\n);
            ",
            field_sqls.join(",\n\t")
        )
    }

    pub fn field_sqls(&self, bt: &BackendType) -> Vec<String> {
        self.fields()
            .iter()
            .map(|field| field.to_sql(bt).trim().to_string())
            .collect()
    }

    /// Generate `ALTER TABLE` queries if changes are detected
    /// on a model.
    ///
    /// - If existing columns are modified, we generate a query to
    /// remove the column and replace with the updated version
    /// - If new columns were added, we generate a query to add a new
    /// columns
    pub fn sql_alter_table(&self, prev: &Self, bt: &BackendType) -> Option<String> {
        if self == prev {
            return None;
        }

        let table_name = self.name();
        let new_fields = self.fields();
        let prev_fields = prev.fields();

        let mut existing_cols = Vec::new();
        let mut new_cols = Vec::new();

        // determine existing and new columns
        for nf in new_fields {
            let exists = prev_fields.iter().find(|pf| nf.col_name() == pf.col_name());

            match exists {
                Some(exist) => existing_cols.push(exist.col_name()),
                None => new_cols.push(format!(
                    "ALTER TABLE {table_name} ADD COLUMN {};",
                    nf.to_sql(bt)
                )),
            };
        }

        let mut queries = Vec::new();
        // if we changed existing column, attempt to recreate the
        // table and refill with existing data
        if !existing_cols.is_empty() {
            let col_names = existing_cols.join(", ");
            let old_table_name = format!("{table_name}_old");

            // backup table
            let mut query = format!(
                "DROP TABLE IF EXISTS {old_table_name};\nALTER TABLE {table_name} RENAME TO {old_table_name};\n",
            );

            // recreate table
            let mut field_sqls = self.field_sqls(bt);
            if let Some(ut_sql) = self.unique_together_sql(bt) {
                field_sqls.push(ut_sql);
            }

            let create_table = format!(
                "CREATE TABLE {table_name} (\n\t{}\n);\n",
                field_sqls.join(",\n\t")
            );
            query.push_str(&create_table);

            // copy data from old to new table
            let move_data = format!(
                "INSERT INTO {table_name} ({col_names}) \nSELECT {col_names} FROM {old_table_name};\n"
            );
            query.push_str(&move_data);
            query.push_str(&format!("DROP TABLE {old_table_name};"));

            queries.push(query);
        }

        // for new columns, generate add column query
        if !new_cols.is_empty() {
            let query = new_cols.join("\n");
            queries.push(query);
        }

        if queries.is_empty() {
            None
        } else {
            Some(queries.join("\n\n"))
        }
    }

    fn unique_together_sql(&self, bt: &BackendType) -> Option<String> {
        use BackendType::*;

        if let Some(ut) = &self.unique_together {
            let cols: Vec<&str> = ut.split(",").map(|c| c.trim()).collect();
            if !cols.is_empty() {
                let name = cols.join("_");
                let sql = match bt {
                    MySql => format!("UNIQUE KEY {name} ({ut})"),
                    Postgres => format!("CONSTRAINT {name} UNIQUE ({ut})"),
                    Sqlite => format!("UNIQUE ({ut})"),
                };

                return Some(sql);
            }
        }

        None
    }
}

impl From<&ItemStruct> for ModelDefinition {
    fn from(value: &ItemStruct) -> Self {
        let name = parse_model_name(&value);
        let unique_together = parse_model_attr(&value, "unique_together");
        let ItemStruct { fields, .. } = value;

        ModelDefinition {
            name,
            unique_together,
            fields: fields.iter().map(FieldDefinition::from).collect(),
        }
    }
}

/// Parse the model name as a valid database table name.
///
/// We first seek if model struct has a #\[table_name = ".."] attribute.
/// Otherwise we parse the struct name as a valid database table name.
fn parse_model_name(model: &ItemStruct) -> String {
    let name = parse_model_attr(model, "table_name");

    name.unwrap_or_else(|| {
        let struct_name = model.ident.to_token_stream().to_string();
        let mut name = String::new();

        for (i, c) in struct_name.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    name.push('_');
                }

                name.push(c.to_ascii_lowercase());
            } else {
                name.push(c);
            }
        }

        name
    })
}

fn parse_model_attr(model: &ItemStruct, attr_name: &str) -> Option<String> {
    let mut attr_value = None;
    let attr_name = Ident::new(attr_name, Span::call_site());

    for attr in &model.attrs {
        if let Some(ident) = attr.path().get_ident() {
            if ident == &attr_name {
                if let Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(value_lit),
                        ..
                    }) = &meta.value
                    {
                        attr_value = Some(value_lit.value())
                    }
                }
            }
        }
    }

    attr_value
}

impl PartialEq for ModelDefinition {
    fn eq(&self, other: &Self) -> bool {
        let config = config::standard();
        let s = bincode::encode_to_vec(&self, config);
        let o = bincode::encode_to_vec(other, config);

        if let (Ok(s), Ok(o)) = (s, o) {
            s == o
        } else {
            false
        }
    }
}
