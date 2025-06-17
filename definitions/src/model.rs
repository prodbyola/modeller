use crate::{backend_type::BackendType, field::FieldDefinition};
use bincode::{Decode, Encode};
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
        let field_sqls: Vec<String> = self
            .fields()
            .iter()
            .map(|field| field.to_sql(bt).trim().to_string())
            .collect();

        format!(
            "DROP TABLE IF EXISTS {table_name};\nCREATE TABLE {table_name} (\n\t{}\n);",
            field_sqls.join(",\n\t")
        )
    }

    /// Generate `ALTER TABLE` queries if changes are detected
    /// on a model.
    ///
    /// - If existing columns are modified, we generate a query to
    /// remove the column and replace with the updated version
    /// - If new columns were added, we generate a query to add a new
    /// columns
    pub fn sql_alter_table(&self, prev: &Self, bt: &BackendType) -> Option<String> {
        let table_name = self.name();
        let new_fields = self.fields();
        let prev_fields = prev.fields();

        let mut queries = Vec::with_capacity(new_fields.len());

        for nf in new_fields {
            let exists = prev_fields.iter().find(|pf| nf.col_name() == pf.col_name());

            match exists {
                Some(exist) => {
                    if exist != nf {
                        queries.push(format!("ALTER TABLE DROP COLUMN {};", exist.col_name()));
                        queries.push(format!(
                            "ALTER TABLE {table_name} ADD COLUMN {};",
                            nf.to_sql(bt)
                        ));
                    }
                }
                None => {
                    let q = format!("ALTER TABLE {table_name} ADD COLUMN {};", nf.to_sql(bt));
                    queries.push(q);
                }
            };
        }

        if queries.is_empty() {
            None
        } else {
            Some(queries.join("\n"))
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
