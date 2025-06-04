use quote::ToTokens;
use syn::{ItemStruct, Token, parse::Parse};

use crate::implmt::{backend_type::BackendType, field::FieldDefinition};

enum TableOperation {
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
        let ItemStruct { ident, fields, .. } = value;
        ModelDefinition {
            name: ident.to_token_stream().to_string(),
            fields: fields.iter().map(FieldDefinition::from).collect(),
        }
    }
}
