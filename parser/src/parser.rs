use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataStruct, DeriveInput, Ident, parse_macro_input};

use darling::{FromDeriveInput, FromField};
use definitions::{
    bincode::{self, config},
    field::{FieldDefinition, FieldOptions},
    model::{ModelArgs, ModelDefinition},
};

pub fn impl_parse_model(stream: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(stream);

    // let input_clone = input.clone();
    let ident = input.ident.clone();

    let mut name = parse_table_name(&ident);
    let mut unique_together = None;

    if let Ok(args) = ModelArgs::from_derive_input(&input) {
        name = args.name.unwrap_or(name);
        unique_together = args.unique_together.map(|list| {
            list.iter()
                .map(|p| p.to_token_stream().to_string())
                .collect()
        });
    }

    match input.data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let fields = fields
                .iter()
                .map(|f| {
                    let mut field: FieldDefinition = f.into();

                    if let Ok(opts) = FieldOptions::from_field(f) {
                        field.accept_opts(opts);
                    }

                    field
                })
                .collect::<Vec<_>>();

            let model = ModelDefinition {
                name,
                unique_together,
                fields,
            };

            let config = config::standard();
            let raw = bincode::encode_to_vec(model, config).unwrap_or_default();

            let bincode_enabled = std::env::var("BINCODE_FEATURE_ENABLED").is_ok();
            let bincode_features = if bincode_enabled {
                quote! {
                    fn get_definition() -> OpResult<ModelDefinition> {
                        let stream = Self::get_stream();
                        let config = config::standard();
                        let (model, _): (ModelDefinition, _) = bincode::decode_from_slice(&stream, config)?;

                        Ok(model)
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                impl #ident {
                    fn get_stream() -> Vec<u8> {
                        vec![#(#raw),*]
                    }

                    pub async fn write_stream(config: &Config) -> Result<(), Error> {
                        let mut stream = Self::get_stream();
                        ModellerExec::write_stream(&mut stream, config).await?;

                        Ok(())
                    }

                    #bincode_features
                }
            }
            .into()
        }
        _ => panic!("only struct models are supported"),
    }
}

fn parse_table_name(ident: &Ident) -> String {
    let struct_name = ident.to_token_stream().to_string();
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
}
