use quote::ToTokens;
use syn::{Field, Meta};

use crate::backend_type::BackendType;

use crate::column::ColumnType;

#[derive(Debug, Default)]
pub struct FieldDefinition {
    col_name: String,
    col_type: ColumnType,
    serial: bool, // autoincrement field
    unique: bool,
    default_value: Option<String>,
    length: Option<usize>,
}

impl FieldDefinition {
    pub fn to_sql(&self, bt: &BackendType) -> String {
        use BackendType::*;
        let col = &self.col_name;

        if self.serial {
            match bt {
                MySql => format!("{col} INT AUTO_INCREMENT PRIMARY KEY"),
                Postgres => format!("{col} INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY"),
                Sqlite => format!("{col} INTEGER PRIMARY KEY AUTOINCREMENT"),
            }
        } else {
            let col_type = &self.col_type.to_sql(&self.length);
            let unique = if self.unique { "UNIQUE" } else { "" };
            let default_value = &self
                .default_value
                .as_ref()
                .map(|v| format!("DEFAULT {}", v.trim()))
                .unwrap_or(String::new());
            format!("{col} {col_type} {unique} {default_value}")
        }
    }
}

impl From<&Field> for FieldDefinition {
    fn from(value: &Field) -> Self {
        let Field {
            ident, ty, attrs, ..
        } = value;
        let mut col_name = ident
            .as_ref()
            .map(|v| v.to_token_stream().to_string())
            .unwrap_or("".to_string());

        let mut col_type = ty.into();
        let mut serial = false;
        let mut unique = false;
        let mut default_value = None;
        let mut length = None;

        for attr in attrs {
            if let Some(ident) = attr.path().get_ident() {
                if ident == "modeller" {
                    if let Meta::List(meta) = &attr.meta {
                        let value = meta.tokens.to_string();
                        for prop in value.split(",") {
                            let prop = prop.trim();
                            if ["serial", "unique"].contains(&prop) {
                                serial = prop == "serial";
                                unique = prop == "unique";

                                continue;
                            }

                            let prop_split: Vec<&str> = prop.split("=").collect();
                            if let (Some(key), Some(value)) = (prop_split.get(0), prop_split.get(1))
                            {
                                let key = key.trim();
                                if key == "default" {
                                    default_value = Some(value.to_string())
                                } else if key == "length" {
                                    match value.parse::<usize>() {
                                        Ok(len) => length = Some(len),
                                        Err(_) => panic!(
                                            r#"unable to parse attr "length" for field "{col_name}"."#
                                        ),
                                    }
                                } else if key == "name" {
                                    col_name = value.to_string()
                                } else if key == "type" {
                                    col_type = ColumnType::from(*value);
                                }
                            }
                        }
                    }
                }
            }
        }

        FieldDefinition {
            col_name,
            col_type,
            serial,
            unique,
            default_value,
            length,
        }
    }
}
