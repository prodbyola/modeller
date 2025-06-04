extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::parsed::Modeller;

mod backend_type;
mod column;
mod field;
mod parsed;

fn impl_define_models(stream: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(stream as Modeller);
    let sql = parsed.create_tables();

    quote! {
        println!("{}", #sql);
    }
    .into()
}

#[proc_macro]
pub fn define_models(stream: TokenStream) -> TokenStream {
    impl_define_models(stream)
}
