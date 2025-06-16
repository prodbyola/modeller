use proc_macro::TokenStream;

use crate::hashify::impl_hashify;

mod hashify;

#[proc_macro_derive(IntoHashMap)]
pub fn hashify_struct(stream: TokenStream) -> TokenStream {
    impl_hashify(stream)
}
