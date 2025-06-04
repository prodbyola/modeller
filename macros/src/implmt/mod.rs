use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::implmt::parsed::ParsedModels;
mod column;
mod field;
mod parsed;

pub fn impl_define_models(stream: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(stream as ParsedModels);

    let mut output = quote! {};
    for model in parsed.models() {
        let name = model.name();
        let fields: Vec<(&str, &str)> = model
            .fields()
            .iter()
            .map(|f| (f.col_name(), f.col_type().to_str()))
            .collect();

        let (n1, f1) = fields[0];
        let (n2, f2) = fields[1];

        output.extend(quote! {
            println!("name: {}", #name);
            println!( "\t{}: {}", #n1, #f1);
            println!( "\t{}: {}", #n2, #f2);
        });
    }

    output.into()
}
