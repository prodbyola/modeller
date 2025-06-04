use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::implmt::parsed::Modeller;
mod backend_type;
mod column;
mod field;
mod parsed;

pub fn impl_define_models(stream: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(stream as Modeller);
    let sql = parsed.create_tables();

    quote! {
        println!("{}", #sql);
    }
    .into()
}
