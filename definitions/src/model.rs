use core::panic;

use crate::{backend_type::BackendType, field::FieldDefinition};
use quote::{ToTokens, quote};
use serde::{Deserialize, Serialize};
use syn::{Expr, ItemStruct, Meta};

#[derive(Serialize, Deserialize)]
pub struct ModelDefinition {
    name: String,
    fields: Vec<FieldDefinition>,
}

impl ModelDefinition {
    pub fn fields(&self) -> &[FieldDefinition] {
        &self.fields
    }

    pub fn create_table_sql(&self, bt: &BackendType) -> String {
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
}

impl ToTokens for ModelDefinition {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match serde_json::to_string(&self) {
            Ok(def) => tokens.extend(quote! {#def}),
            Err(err) => panic!("unable to strigify model definition: {err}"),
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
