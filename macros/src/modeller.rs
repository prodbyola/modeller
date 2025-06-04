use std::env;

use quote::ToTokens;
use syn::{Expr, ItemStruct, Meta, Token, parse::Parse};

use crate::{backend_type::BackendType, field::FieldDefinition};

pub struct ModelDefinition {
    name: String,
    fields: Vec<FieldDefinition>,
}

impl ModelDefinition {
    pub fn fields(&self) -> &[FieldDefinition] {
        &self.fields
    }

    pub fn create_table_sql(&self, bt: &BackendType) -> String {
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
}

pub struct Modeller {
    bt: BackendType,
    items: Vec<ItemStruct>,
}

impl Modeller {
    fn get_create_tables_sql(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|m| {
                let model = ModelDefinition::from(m);
                model.create_table_sql(&self.bt)
            })
            .collect()
    }

    pub fn models(&self) -> Vec<ModelDefinition> {
        self.items.iter().map(|item| item.into()).collect()
    }

    pub fn bt(&self) -> &BackendType {
        &self.bt
    }

    pub fn items(&self) -> &[ItemStruct] {
        &self.items
    }
}

impl Parse for Modeller {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // set sqlite as default dbd
        let bt = match env::var("MODELLER_DATABASE_URL") {
            Ok(url) => url.as_str().into(),
            Err(_) => BackendType::Sqlite,
        };

        // load model definitions
        let mut models = Vec::new();
        while !input.is_empty() {
            models.push(input.parse()?);
            input.parse::<Token![,]>()?;
        }

        Ok(Modeller { items: models, bt })
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
