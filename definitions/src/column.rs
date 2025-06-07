use bincode::{Decode, Encode};
use quote::ToTokens;
use syn::Type;

#[derive(Debug, Default, Encode, Decode)]
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
}

impl ColumnType {
    pub fn to_sql(&self, len: &Option<usize>) -> String {
        use ColumnType::*;

        let len_str = len.map(|v| format!("({v})")).unwrap_or(String::new());

        let sql = match self {
            VarChar => format!("{}{len_str}", self.to_str()),
            _ => format!("{}", self.to_str()),
        };

        match self {
            Nullable(_) => sql,
            _ => format!("{sql} NOT NULL"),
        }
    }

    pub fn to_str(&self) -> &'static str {
        use ColumnType::*;

        match self {
            Int8 => "BIT",
            Int16 => "SMALLINT",
            Int32 => "INTEGER",
            Int64 => "BIGINT",
            Text => "TEXT",
            VarChar => "VARCHAR",
            Datetime => "TIMESTAMP",
            Nullable(inner) => inner.to_str(),
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
            "Timestamp" | "Datetime" => Datetime,
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
