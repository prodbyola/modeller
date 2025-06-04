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

        struct Modeller {
            bt: BackendType,
            db_url: String,
            db_pool: RBatis,
            migrations_dir: String,
        };

        impl Modeller {
            fn new() -> Result<Self, std::env::VarError> {
                let db_url = std::env::var("MODELLER_DATABASE_URL")?;
                let bt = db_url.as_str().into();
                let migrations_dir = std::env::var("MODELLER_MIGRATIONS_DIR")?;
                let db_pool = RBatis::new();

                let s = Self {
                    db_pool,
                    db_url,
                    migrations_dir,
                    bt
                };

                Ok(s)
            }

            fn create_sqls() {
                #(println!("{}", #struct_idents::create_sql());)*
            }

            /// initializes modeller.
            /// - attempts to connect to the database
            /// - create database "migrations" table if it doesn't exist
            /// - create "migrations" directory and metadata file if they don't exist.
            pub async fn init(&self) {
                // perform init
                self.connect().await;
                self.create_migrations_table().await;
                self.create_migrations_folder().await;
            }

            async fn create_migrations_table(&self) {
                let query = r#"
                    CREATE TABLE IF NOT EXISTS mmm_migrations (
                        filename VARCHAR(200) NOT NULL UNIQUE,
                        run_status BOOLEAN DEFAULT false
                    )
                "#;

                if let Err(err) = self.db_pool.exec(&query, vec![]).await {
                    panic!("unable to create migrations table: {err}");
                }
            }

            async fn create_migrations_folder(&self) {
                let dir = Path::new(&self.migrations_dir);
                let dir_exists = dir.exists() && dir.is_dir();

                if !dir_exists {
                    if let Err(err) = tokio::fs::create_dir_all(dir).await {
                        panic!("unable to create migations dir: {err}");
                    }
                }
            }

            async fn connect(&self) {
                use BackendType::*;
                let rb = &self.db_pool;
                let url = &self.db_url;

                match self.bt {
                    Sqlite => {
                        if let Err(err) = rb.link(SqliteDriver {}, url).await {
                            panic!("{err}")
                        }
                    }
                    MySql => {
                        if let Err(err) = rb.link(MysqlDriver {}, url).await {
                            panic!("{err}")
                        }
                    }
                    Postgres => {
                        if let Err(err) = rb.link(PgDriver {}, url).await {
                            panic!("{err}")
                        }
                    }
                }
            }
        }
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
pub fn define_models(stream: TokenStream) -> TokenStream {
    impl_define_models(stream)
}
