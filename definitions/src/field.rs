use crate::backend_type::BackendType;
use crate::column::ColumnType;
use bincode::{Decode, Encode, config};

use darling::{FromField, FromMeta};
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
    foreign_key: Option<FkOptions>,
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

    foreign_key: Option<FkOptions>,
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
                .unwrap_or_default();

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
            foreign_key,
        } = opts;
        if let Some(name) = col_name {
            self.col_name = name
        }

        if let Some(ct) = col_type {
            self.col_type = ct
        }

        self.serial = serial.unwrap_or_default();
        self.unique = unique.unwrap_or_default();
        self.primary = primary.unwrap_or_default();
        self.default_value = default_value;
        self.length = length;
        self.foreign_key = foreign_key;
    }

    pub fn col_name(&self) -> &str {
        &self.col_name
    }

    pub fn foreign_key(&self) -> &Option<FkOptions> {
        &self.foreign_key
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

        FieldDefinition {
            col_name,
            col_type,
            ..Default::default()
        }
    }
}

impl PartialEq for FieldDefinition {
    fn eq(&self, other: &Self) -> bool {
        let config = config::standard();
        let s = bincode::encode_to_vec(self, config);
        let o = bincode::encode_to_vec(other, config);

        if let (Ok(s), Ok(o)) = (s, o) {
            s == o
        } else {
            false
        }
    }
}

#[derive(Debug, Default, Encode, Decode, FromMeta)]
pub struct FkOptions {
    references: String,
    on_delete: FkOnDelete,
}

impl FkOptions {
    pub fn references(&self) -> &str {
        &self.references
    }

    pub fn on_delete(&self) -> &FkOnDelete {
        &self.on_delete
    }
}

impl FkOptions {
    pub fn to_sql(&self, col_name: &str) -> String {
        format!(
            "FOREIGN KEY ({}) REFERENCES {} ON DELETE {}",
            col_name,
            self.references,
            self.on_delete.to_sql()
        )
    }
}

#[derive(Debug, Default, Encode, Decode, PartialEq, FromMeta)]
pub enum FkOnDelete {
    Cascade,
    Nullify,
    Default,
    Restrict,

    #[default]
    NoAction,
}

impl FkOnDelete {
    pub fn to_sql(&self) -> String {
        use FkOnDelete::*;

        let sql = match self {
            Cascade => "CASCADE",
            Nullify => "SET NULL",
            Default => "SET DEFAULT",
            Restrict => "RESTRICT",
            NoAction => "NO ACTION",
        };

        sql.to_string()
    }
}
