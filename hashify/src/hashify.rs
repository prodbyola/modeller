use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, parse_macro_input};

pub fn impl_hashify(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);
    let item_ident = input.ident;

    match input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            let field_ids = fields
                .iter()
                .map(|f| f.ident.as_ref())
                .flatten()
                .collect::<Vec<_>>();

            quote! {
                type HashType = std::collections::HashMap<String, String>;

                impl From<#item_ident> for HashType {
                    fn from(value: #item_ident) -> HashType {
                        let mut output = HashType::new();
                        #(
                            output.insert(stringify!(#field_ids).to_string(), value.#field_ids.to_string());
                        )*

                        output
                    }
                }
            }
            .into()
        }
        _ => panic!("only struct data types are supported"),
    }
}
