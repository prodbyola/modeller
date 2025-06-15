extern crate proc_macro;

use parser::impl_parse_models;
use proc_macro::TokenStream;

mod parser;

#[proc_macro]
/// Parses Rust struct models into `ModelDefinitions` and
/// other types that can be used by modeller.
pub fn parse_models(stream: TokenStream) -> TokenStream {
    impl_parse_models(stream)
}
