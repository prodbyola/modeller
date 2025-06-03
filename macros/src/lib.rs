extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput};

use crate::{
    column::column_types,
    field::{build_definitions, field_type},
};

mod column;
mod field;

#[proc_macro_derive(Model)]
pub fn derive_db_model(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let struct_identifier = input.ident;

    match input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            let field_type = field_type();
            let column_types = column_types();
            let field_list = build_definitions(fields);

            quote! {
                #field_type
                #column_types

                impl #struct_identifier {
                    fn field_definitions() -> Vec<FieldDefinition> {
                        #field_list
                        field_definitions
                    }
                }
            }
        }
        _ => panic!("model can only be derived from structs"),
    }
    .into()
}
