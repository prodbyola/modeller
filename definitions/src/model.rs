use crate::{backend_type::BackendType, field::FieldDefinition};
use bincode::{Decode, Encode, config};
use quote::ToTokens;
use syn::{Expr, ItemStruct, Meta};

#[derive(Encode, Decode)]
pub struct ModelDefinition {
    name: String,
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
        let field_sqls = self.field_sqls(bt);

        format!(
            "DROP TABLE IF EXISTS {table_name};\nCREATE TABLE {table_name} (\n\t{}\n);",
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
            let field_sqls = self.field_sqls(bt);
            let create_table = format!(
                "CREATE TABLE {table_name} (\n\t{});\n",
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
}

impl From<&ItemStruct> for ModelDefinition {
    fn from(value: &ItemStruct) -> Self {
        let name = parse_model_name(&value);
        let ItemStruct { fields, .. } = value;
        ModelDefinition {
            name,
            fields: fields.iter().map(FieldDefinition::from).collect(),
        }
    }
}

/// Parse the model name as a valid database table name.
///
/// We first seek if model struct has a #\[table_name = ".."] attribute.
/// Otherwise we parse the struct name as a valid database table name.
fn parse_model_name(model: &ItemStruct) -> String {
    let mut name = None;

    for attr in &model.attrs {
        if let Some(ident) = attr.path().get_ident() {
            if ident == "table_name" {
                if let Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(table_name),
                        ..
                    }) = &meta.value
                    {
                        name = Some(table_name.value())
                    }
                }
            }
        }
    }

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
