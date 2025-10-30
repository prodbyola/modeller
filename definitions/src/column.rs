use std::fmt::Display;

use bincode::{Decode, Encode};
use darling::FromMeta;
use quote::ToTokens;
use syn::Type;

use crate::backend_type::BackendType;

#[derive(Debug, Default, Encode, Decode, FromMeta, Clone)]
pub(super) enum ColumnType {
    Int8,
    Int16,
    Int32,
    Int64,
    Text,
    #[default]
    VarChar,
    Datetime,
    Nullable(Box<ColumnType>),
    Bool,
    Float32,
    Float64
}

impl ColumnType {
    pub fn to_sql(&self, len: &Option<usize>, bkt: &BackendType) -> String {
        use ColumnType::*;

        let len_str = len.map(|v| format!("({v})")).unwrap_or_default();

        // derive sql from ColumnType
        let sql = |col_type: &ColumnType| match col_type {
            VarChar => match bkt {
                BackendType::Sqlite => {
                    if len.is_some() {
                        format!("{}{len_str}", col_type.to_str())
                    } else {
                        "TEXT".to_string()
                    }
                }
                _ => format!("{}{len_str}", col_type.to_str()),
            },
            Bool => match bkt {
                BackendType::Sqlite => "BOOLEAN".to_string(),
                _ => col_type.to_str().to_string(),
            },
            Float32 => match bkt {
                BackendType::MySql => "FLOAT".to_string(),
                _ => col_type.to_str().to_string(),
            },
            _ => col_type.to_str().to_string(),
        };

        match self {
            Nullable(inner) => sql(inner),
            _ => format!("{} NOT NULL", sql(self)),
        }
    }

    pub fn to_str(&self) -> &'static str {
        use ColumnType::*;

        match self {
            Int16 | Int8 => "SMALLINT",
            Int32 => "INTEGER",
            Int64 => "BIGINT",
            Text => "TEXT",
            VarChar => "VARCHAR",
            Datetime => "TIMESTAMP",
            Nullable(inner) => inner.to_str(),
            Float64 => "DOUBLE PRECISION",
            Float32 => "REAL",
            Bool => "BOOL",
        }
    }

    pub fn from_type_str(ty: &str) -> Self {
        use ColumnType::*;

        match ty {
            "u64" | "i64" => Int64,
            "u32" | "i32" => Int32,
            "u16" | "i16" => Int16,
            "u8" | "i8" => Int8,
            "String" | "str" => VarChar,
            "Text" => Text,
            "Timestamp" | "DateTime" => Datetime,
            "bool" => Bool,
            "f32" => Float32,
            "f64" => Float64,
            _ => panic!("ColumnDefinition not implemented for {ty}"),
        }
    }
}

impl<'a> From<&'a str> for ColumnType {
    fn from(ty: &'a str) -> Self {
        use ColumnType::*;

        if ty.starts_with("NULLABLE") {
            let split: Vec<&str> = ty.split(" ").collect();
            if let Some(value) = split.get(1) {
                let inner = Box::new(value.trim().into());
                Nullable(inner)
            } else {
                panic!("provide field type for a nullable field")
            }
        } else {
            match ty {
                "BIGINT" => Int64,
                "INTEGER" => Int32,
                "SMALLINT" => Int16,
                "BIT" => Int8,
                "VARCHAR" => VarChar,
                "TEXT" => Text,
                "DATETIME" => Datetime,
                "BOOL" => Bool,
                "REAL" => Float32,
                "DOUBLE PRECISION" => Float64,
                _ => panic!("ColumnDefinition not implemented for {ty}"),
            }
        }
    }
}

impl From<&Type> for ColumnType {
    fn from(ty: &Type) -> Self {
        use ColumnType::*;
        let ty = ty.to_token_stream().to_string();
        let ty = ty.trim();

        if ty.starts_with("Option") {
            let rem_opt = ty.trim_start_matches("Option < ");
            let trimmed = rem_opt.trim_end_matches(" >");
            let inner = Box::new(ColumnType::from_type_str(trimmed));
            Nullable(inner)
        } else {
            ColumnType::from_type_str(ty)
        }
    }
}

impl Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
