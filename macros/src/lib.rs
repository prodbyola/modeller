extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Field, Fields, Ident, Path, parse_macro_input};

use crate::modeller::Modeller;

mod backend_type;
mod column;
mod field;
mod modeller;

fn impl_define_models(stream: TokenStream) -> TokenStream {
    let modeller = parse_macro_input!(stream as Modeller);
    let models = modeller.models();
    let items = modeller.items();

    let original_structs = items.into_iter().enumerate().map(|(i, item)| {
        let vis = &item.vis;
        let attrs = &item.attrs;
        let attrs: Vec<&Attribute> = attrs
            .into_iter()
            .filter(|attr| should_keep_attr(attr, "table_name"))
            .collect();

        let ident = &item.ident;
        let generics = &item.generics;
        let fields = match &item.fields {
            Fields::Named(named) => {
                let new_fields = named.named.iter().cloned().map(strip_field_attrs);
                quote! {
                    {
                        #(#new_fields),*
                    }
                }
            }
            _ => quote! {},
        };

        let create_sql = models
            .get(i)
            .map(|m| m.create_table_sql(modeller.bt()))
            .unwrap_or(String::new());

        quote! {
            #(#attrs)*
            #vis struct #ident #generics #fields

            impl #ident {
                fn create_sql() -> String {
                    #create_sql.to_string()
                }
            }
        }
    });

    let bt_src = std::fs::read_to_string("macros/src/backend_type.rs")
        .expect("unable to load bt source file.");
    let bt_quote: syn::File = syn::parse_file(&bt_src).expect("unable to parse bt source");
    let struct_idents: Vec<&Ident> = items.iter().map(|item| &item.ident).collect();

    quote! {
        #bt_quote
        #(#original_structs)*
    }
    .into()
}

/// Helper to filter out undesired attributes (e.g., "serde", "deprecated")
fn should_keep_attr(attr: &Attribute, ident_key: &'static str) -> bool {
    let Path { segments, .. } = attr.path();
    if let Some(seg) = segments.first() {
        let ident = seg.ident.to_string();
        return ident != ident_key;
    }

    true
}

fn strip_field_attrs(mut field: Field) -> Field {
    field.attrs = field
        .attrs
        .into_iter()
        .filter(|attr| should_keep_attr(attr, "modeller"))
        .collect();
    field
}

#[proc_macro]
pub fn analyze_models(stream: TokenStream) -> TokenStream {
    impl_define_models(stream)
}
