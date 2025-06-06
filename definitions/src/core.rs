use quote::{ToTokens, quote};
use syn::{ItemStruct, Token, parse::Parse};

use crate::{backend_type::BackendType, model::ModelDefinition};

pub struct DefinitionStream {
    pub items: Vec<ItemStruct>,
    pub models: Vec<String>,
}

impl DefinitionStream {
    pub fn items(&self) -> &[ItemStruct] {
        &self.items
    }

    pub fn models(&self) -> &[String] {
        &self.models
    }
}

pub struct Definitions {
    pub bt: BackendType,
    pub models: Vec<ModelDefinition>,
}

impl Definitions {
    pub fn get_create_tables_sql(&self) -> Vec<String> {
        self.models
            .iter()
            .map(|m| m.create_table_sql(&self.bt))
            .collect()
    }

    pub fn bt(&self) -> &BackendType {
        &self.bt
    }
}

impl Parse for DefinitionStream {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // set sqlite as default dbd
        // let bt = match env::var("MODELLER_DATABASE_URL") {
        //     Ok(url) => url.as_str().into(),
        //     Err(_) => BackendType::Sqlite,
        // };

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

        Ok(DefinitionStream { items, models })
    }
}

impl ToTokens for DefinitionStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let DefinitionStream { models, .. } = self;

        tokens.extend(quote! {
            DefinitionStream {
                models: vec![#(#models.to_string()),*],
                items: vec![],
            }
        });
    }
}
