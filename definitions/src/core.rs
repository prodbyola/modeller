use bincode::config;
use quote::{ToTokens, quote};
use syn::{ItemStruct, Token, parse::Parse};

use crate::{backend_type::BackendType, model::ModelDefinition};

pub struct DefinitionStream {
    items: Vec<ItemStruct>,
}

impl DefinitionStream {
    pub fn items(&self) -> &[ItemStruct] {
        &self.items
    }

    pub fn raw(&self) -> Vec<u8> {
        let config = config::standard();
        let defs: Vec<ModelDefinition> = self.items.iter().map(ModelDefinition::from).collect();

        match bincode::encode_to_vec(&defs, config) {
            Ok(raw) => raw,
            Err(_) => vec![],
        }
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

        while !input.is_empty() {
            let item: ItemStruct = input.parse()?;
            items.push(item);

            input.parse::<Token![,]>()?;
        }

        Ok(DefinitionStream { items })
    }
}

impl ToTokens for DefinitionStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let bytes = self.raw();
        tokens.extend(quote! {
            pub fn modeller_definition_streams() -> Vec<u8> {
                vec![#(#bytes),*]
            }
        });
    }
}
