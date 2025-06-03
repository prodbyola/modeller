use proc_macro2::TokenStream;
use quote::quote;
use syn::Fields;

pub fn field_type() -> TokenStream {
    quote! {
        #[derive(Debug, Default)]
        struct FieldDefinition {
            col_name: &'static str,
            col_type: ColumnType,
            unique: bool,
            default_value: Option<&'static str>,
            lenth: Option<usize>,
            serial: bool // autoincrement field
        }
    }
}

pub fn build_definitions(fields: Fields) -> TokenStream {
    let len = fields.len();
    let mut stream = quote! {
        let mut field_definitions: Vec<FieldDefinition> = Vec::with_capacity(#len);
    };

    for field in fields {
        if let Some(fi) = field.ident {
            let ft = field.ty;

            stream.extend(quote! {
                let row = FieldDefinition {
                    col_name: stringify!(#fi),
                    col_type: stringify!(#ft).into(),
                    ..Default::default()
                };

                field_definitions.push(row);
            });
        }
    }

    stream
}
