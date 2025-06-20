use crate::backend_type::BackendType;
use crate::column::ColumnType;
use bincode::{Decode, Encode, config};

use darling::FromField;
use quote::ToTokens;
use syn::Field;

fn parse_column_type(col: String) -> Option<ColumnType> {
    Some(col.as_str().into())
}

#[derive(Debug, Default, Encode, Decode)]
pub struct FieldDefinition {
    col_name: String,
    col_type: ColumnType,
    serial: bool, // autoincrement field
    unique: bool,
    primary: bool,
    default_value: Option<String>,
    length: Option<usize>,
}

#[derive(FromField, Default)]
#[darling(attributes(modeller))]
pub struct FieldOptions {
    #[darling(rename = "name")]
    col_name: Option<String>,

    #[darling(rename = "type", map = "parse_column_type")]
    col_type: Option<ColumnType>,
    serial: Option<bool>, // autoincrement field
    unique: Option<bool>,
    primary: Option<bool>,

    #[darling(rename = "default")]
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
            let primary = if self.primary { "PRIMARY KEY" } else { "" };
            let default_value = &self
                .default_value
                .as_ref()
                .map(|v| format!("DEFAULT {}", v.trim()))
                .unwrap_or(String::new());
            format!("{col} {col_type} {unique} {default_value} {primary}")
        }
    }

    pub fn accept_opts(&mut self, opts: FieldOptions) {
        let FieldOptions {
            col_name,
            col_type,
            serial,
            unique,
            primary,
            default_value,
            length,
        } = opts;
        if let Some(name) = col_name {
            self.col_name = name
        }

        if let Some(ct) = col_type {
            self.col_type = ct
        }

        if let Some(s) = serial {
            self.serial = s
        }

        if let Some(u) = unique {
            self.unique = u
        }

        if let Some(p) = primary {
            self.primary = p
        }

        if let Some(d) = default_value {
            self.default_value = Some(d)
        }

        if let Some(l) = length {
            self.length = Some(l)
        }
    }

    pub fn col_name(&self) -> &str {
        &self.col_name
    }
}

impl From<&Field> for FieldDefinition {
    fn from(value: &Field) -> Self {
        let Field { ident, ty, .. } = value;
        let col_name = ident
            .as_ref()
            .map(|v| v.to_token_stream().to_string())
            .unwrap_or("".to_string());

        let col_type = ty.into();
        let serial = false;
        let unique = false;
        let primary = false;
        let default_value = None;
        let length = None;

        FieldDefinition {
            col_name,
            col_type,
            serial,
            unique,
            default_value,
            length,
            primary,
        }
    }
}

impl PartialEq for FieldDefinition {
    fn eq(&self, other: &Self) -> bool {
        let config = config::standard();
        let s = bincode::encode_to_vec(&self, config);
        let o = bincode::encode_to_vec(other, config);

        if let (Ok(s), Ok(o)) = (s, o) {
            s == o
        } else {
            false
        }
    }
}
