use std::collections::VecDeque;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, DataStruct, DeriveInput, Field, Fields, Ident, Path, parse_macro_input};

use darling::{FromDeriveInput, FromField};
use definitions::{
    bincode::{self, config},
    core::DefinitionStream,
    field::FieldDefinition,
    model::{ModelArgs, ModelDefinition},
};

#[derive(FromDeriveInput, Clone)]
#[darling(
    attributes(modeller),
    supports(struct_named),
    forward_attrs(allow, doc, cfg)
)]
struct ModelData {
    ident: syn::Ident,
    attrs: Vec<syn::Attribute>,
}

pub fn impl_parse_models(stream: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(stream);

    let ident = input.ident;
    let mut name = parse_table_name(&ident);
    let mut unique_together = None;

    if let Ok(args) = ModelArgs::from_derive_input(&input) {
        name = args.name;
        unique_together = args.unique_together.map(|list| {
            list.iter()
                .map(|p| p.to_token_stream().to_string())
                .collect()
        });
    }

    match input.data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let fields = fields
                .iter()
                .map(|f| FieldDefinition::from_field(&f).ok())
                .flatten()
                .collect::<Vec<_>>();

            let model = ModelDefinition {
                name,
                unique_together,
                fields,
            };

            let config = config::standard();
            let raw = bincode::encode_to_vec(model, config).unwrap_or(vec![]);
            quote! {
                #input

                impl #ident {
                    pub async fn write_stream(config: &Config) -> Result<(), Error> {
                        let mut stream = vec![#(#raw),*];
                        Modeller::write_stream(&mut stream, config).await?;

                        Ok(())
                    }
                }
            }
            .into()
        }
        _ => panic!("only struct models are supported"),
    }

    // let items = input.items();
    // let mut raws = input.raws().ok().unwrap_or(VecDeque::new());

    // let original_structs = items.into_iter().map(|item| {
    //     let vis = &item.vis;
    //     let attrs = &item.attrs;
    //     let attrs: Vec<&Attribute> = attrs
    //         .into_iter()
    //         .filter(|attr| should_keep_attr(attr, "table_name"))
    //         .filter(|attr| should_keep_attr(attr, "unique_together"))
    //         .collect();

    //     let ident = &item.ident;
    //     let generics = &item.generics;
    //     let fields = match &item.fields {
    //         Fields::Named(named) => {
    //             let new_fields = named.named.iter().cloned().map(strip_field_attrs);
    //             quote! {
    //                 {
    //                     #(#new_fields),*
    //                 }
    //             }
    //         }
    //         _ => quote! {},
    //     };

    //     let mut output = quote! {
    //         #(#attrs)*
    //         #vis struct #ident #generics #fields
    //     };

    //     if let Some(raw) = raws.pop_front() {
    //         output.extend(quote! {
    //             impl #ident {
    //                 fn get_stream() -> Vec<u8> {
    //                     vec![#(#raw),*]
    //                 }
    //             }
    //         });
    //     }

    //     output
    // });

    // let raws = def_stream.raws().ok().unwrap_or(vec![]);
    // let idents: Vec<Ident> = items.iter().map(|item| item.ident.clone()).collect();

    // quote! {
    //     // #(#original_structs)*

    // }
    // .into()
}

fn parse_table_name(ident: &Ident) -> String {
    let struct_name = ident.to_token_stream().to_string();
    let mut name = String::new();

    for (i, c) in struct_name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                name.push('_');
            }

            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }

    name
}

/// Once sql_maker is done analyzing models and extracting sql,
/// we remove all attributes defined for modeller.
///
///  This helper removes modeller attributes in order to avoid
/// "attr not found" error.
///
/// We pass attribute instance and an `ident_key` str that
/// reprensents the modeller attribute we'd link to remove
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
