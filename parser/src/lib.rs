extern crate proc_macro;

use parser::impl_parse_model;
use proc_macro::TokenStream;

mod parser;

#[proc_macro_derive(Modeller, attributes(modeller, table_name))]
/// Parses Rust struct model into a `ModelDefinition` and
/// other types that can be used by modeller.
pub fn parse_model(stream: TokenStream) -> TokenStream {
    impl_parse_model(stream)
}
