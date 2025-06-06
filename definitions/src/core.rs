use quote::{ToTokens, quote};
use syn::{ItemStruct, Token, parse::Parse};

use crate::{backend_type::BackendType, model::ModelDefinition};

pub struct DefinitionStream {
    items: Vec<ItemStruct>,
    // pub models: Vec<String>,
}

impl DefinitionStream {
    pub fn items(&self) -> &[ItemStruct] {
        &self.items
    }

    pub fn models(&self) -> Vec<String> {
        let ms = self
            .items
            .iter()
            .map(|item| {
                let def = ModelDefinition::from(item);
                serde_json::to_string(&def).unwrap_or(String::new())
            })
            .collect();

        ms
    }
}

pub struct Definitions {
    pub bt: BackendType,
    pub models: Vec<ModelDefinition>,
}

impl Definitions {
    pub fn bt(&self) -> &BackendType {
        &self.bt
    }
}

impl Parse for DefinitionStream {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // load model definitions
        let mut items = Vec::new();
        let mut models = Vec::new();

        while !input.is_empty() {
            let item: ItemStruct = input.parse()?;
            let mod_def = ModelDefinition::from(&item);
            let model = serde_json::to_string(&mod_def).unwrap_or(String::new());

            items.push(item);
            models.push(model);

            input.parse::<Token![,]>()?;
        }

        Ok(DefinitionStream { items })
    }
}

impl ToTokens for DefinitionStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let models = self.models();
        tokens.extend(quote! {
            fn def_streams_list() -> Vec<String> {
                vec![#(#models.to_string()),*]
            }
        });
    }
}
