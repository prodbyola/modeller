use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataStruct, DeriveInput, Ident, parse_macro_input};

use darling::{FromDeriveInput, FromField};
use definitions::{
    bincode::{self, config},
    field::FieldDefinition,
    model::{ModelArgs, ModelDefinition},
};

pub fn impl_parse_models(stream: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(stream);

    // let input_clone = input.clone();
    let ident = input.ident.clone();

    let mut name = parse_table_name(&ident);
    let mut unique_together = None;

    if let Ok(args) = ModelArgs::from_derive_input(&input) {
        name = args.name;
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
                .map(|f| FieldDefinition::from_field(&f).ok())
                .flatten()
                .collect::<Vec<_>>();

            let model = ModelDefinition {
                name,
                unique_together,
                fields,
            };

            let config = config::standard();
            let raw = bincode::encode_to_vec(model, config).unwrap_or(vec![]);
            quote! {
                // #input

                impl #ident {
                    pub async fn write_stream(config: &Config) -> Result<(), Error> {
                        let mut stream = vec![#(#raw),*];
                        ModellerExec::write_stream(&mut stream, config).await?;

                        Ok(())
                    }
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
