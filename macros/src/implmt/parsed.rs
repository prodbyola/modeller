use quote::ToTokens;
use syn::{Expr, ItemStruct, Meta, Token, parse::Parse};

use crate::implmt::{backend_type::BackendType, field::FieldDefinition};

pub enum TableOperation {
    Create,
    Alter,
}

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

    pub fn to_sql(&self, bt: &BackendType, op: TableOperation) -> String {
        use TableOperation::*;

        match op {
            Create => {
                let field_sqls: Vec<String> = self
                    .fields()
                    .iter()
                    .map(|field| field.to_sql(bt).trim().to_string())
                    .collect();

                format!(
                    "CREATE TABLE {} (\n\t{}\n);",
                    self.name,
                    field_sqls.join(",\n\t")
                )
            }
            _ => unimplemented!(),
        }
    }
}

pub struct ParsedModels {
    bt: BackendType,
    models: Vec<ModelDefinition>,
}

impl ParsedModels {
    pub fn models(&self) -> &[ModelDefinition] {
        &self.models
    }
    pub fn create_tables(&self) -> String {
        let models = self.models();
        let sql: Vec<String> = models
            .iter()
            .map(|model| model.to_sql(&self.bt, TableOperation::Create))
            .collect();

        format!("{}", sql.join("\n\n"))
    }
}

impl Parse for ParsedModels {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let bt = match std::env::var("DATABASE_URL") {
            Ok(url) => url.as_str().into(),
            Err(_) => BackendType::Sqlite,
        };

        let mut models = Vec::new();
        while !input.is_empty() {
            let s: ItemStruct = input.parse()?;
            models.push(s.into());

            input.parse::<Token![,]>()?;
        }

        Ok(ParsedModels { models, bt })
    }
}

impl From<ItemStruct> for ModelDefinition {
    fn from(value: ItemStruct) -> Self {
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
