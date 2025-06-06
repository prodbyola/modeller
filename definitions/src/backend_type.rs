use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

pub enum BackendType {
    MySql,
    Postgres,
    Sqlite,
}

impl<'a> From<&'a str> for BackendType {
    fn from(value: &'a str) -> BackendType {
        use BackendType::*;

        if value.starts_with("mysql") {
            MySql
        } else if value.starts_with("postgres") {
            Postgres
        } else {
            Sqlite
        }
    }
}

impl ToTokens for BackendType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use BackendType::*;
        let variant = match self {
            Postgres => quote! { BackendType::Postgres },
            MySql => quote! { BackendType::MySql },
            Sqlite => quote! { BackendType::Sqlite },
        };

        tokens.extend(variant);
    }
}
