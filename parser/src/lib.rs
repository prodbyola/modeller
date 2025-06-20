extern crate proc_macro;

use darling::{FromMeta, ast::NestedMeta};
use definitions::field::FieldDefinition;
use parser::impl_parse_models;
use proc_macro::TokenStream;
use quote::quote;
use syn::FieldsNamed;

mod parser;

#[proc_macro_derive(Modeller, attributes(modeller, table_name))]
/// Parses Rust struct models into `ModelDefinitions` and
/// other types that can be used by modeller.
pub fn parse_models(stream: TokenStream) -> TokenStream {
    impl_parse_models(stream)
}
