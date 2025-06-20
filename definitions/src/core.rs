// use std::collections::VecDeque;

// use bincode::config;
// use syn::{ItemStruct, Token, parse::Parse};

// use crate::{backend_type::BackendType, model::ModelDefinition};

// pub struct DefinitionStream {
//     items: Vec<ItemStruct>,
// }

// impl DefinitionStream {
//     pub fn items(&self) -> &[ItemStruct] {
//         &self.items
//     }

//     pub fn raws(&self) -> Result<VecDeque<Vec<u8>>, String> {
//         let config = config::standard();
//         let mut raws = VecDeque::new();

//         for item in &self.items {
//             let def = ModelDefinition::from(item);
//             let raw = bincode::encode_to_vec(&def, config).map_err(|err| err.to_string())?;
//             raws.push_back(raw);
//         }

//         Ok(raws)
//     }
// }

// impl Parse for DefinitionStream {
//     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//         // load model definitions
//         let mut items = Vec::new();

//         while !input.is_empty() {
//             let item: ItemStruct = input.parse()?;
//             items.push(item);

//             input.parse::<Token![,]>()?;
//         }

//         Ok(DefinitionStream { items })
//     }
// }

// pub struct Definitions {
//     pub bt: BackendType,
//     pub models: Vec<ModelDefinition>,
// }

// impl Definitions {
//     pub fn bt(&self) -> &BackendType {
//         &self.bt
//     }
// }
