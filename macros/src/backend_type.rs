use proc_macro2::TokenStream;
use quote::quote;

pub fn backend_type() -> TokenStream {
    quote! {
        enum BackendType {
            MySql,
            Postgres,
            Sqlite
        }

        impl From<&'static str> for BackendType {
            fn from(value: &'static str) -> BackendType {
                use BackendType::*;

                if value.starts_with("mysql") {
                    MySql
                } else if value.starts_with("postgres") {
                    Postgres
                } else  {
                    Sqlite
                }
            }
        }
    }
}
