use quote::ToTokens;
use syn::{ItemStruct, Token, parse::Parse};

use crate::implmt::field::FieldDefinition;

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
}

pub struct ParsedModels {
    models: Vec<ModelDefinition>,
}

impl ParsedModels {
    pub fn models(self) -> Vec<ModelDefinition> {
        self.models
    }
}

impl Parse for ParsedModels {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut models = Vec::new();
        while !input.is_empty() {
            let s: ItemStruct = input.parse()?;
            models.push(s.into());

            input.parse::<Token![,]>()?;
        }

        Ok(ParsedModels { models })
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
