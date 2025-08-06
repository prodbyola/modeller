use crate::{backend_type::BackendType, field::FieldDefinition};
use bincode::{Decode, Encode, config};
use darling::{FromDeriveInput, FromMeta, util::PathList};
use quote::ToTokens;

#[derive(Encode, Decode, Default)]
pub struct ModelDefinition {
    pub name: String,
    pub indexes: Vec<IndexDefinition>,
    pub fields: Vec<FieldDefinition>,
}

impl ModelDefinition {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[FieldDefinition] {
        &self.fields
    }

    /// Generate `CREATE TABLE` sql query for the model.
    pub fn sql_create_table(&self, bt: &BackendType) -> String {
        let table_name = &self.name;
        let field_sqls = self.field_sqls(bt);

        let mut output = format!(
            "DROP TABLE IF EXISTS {table_name};\n\
            CREATE TABLE {table_name} (\n\t{}\n);\n",
            field_sqls.join(",\n\t")
        );

        for index in &self.indexes {
            output.push_str(&format!("{}\n", index.to_sql(table_name)));
        }

        output
    }

    pub fn field_sqls(&self, bt: &BackendType) -> Vec<String> {
        let mut sqls = Vec::new();
        let mut fks = Vec::new();

        for field in &self.fields {
            let col_sql = field.to_sql(bt).trim().to_string();
            sqls.push(col_sql);

            if let Some(fk) = field.foreign_key() {
                fks.push(fk.to_sql(field.col_name()));
            }
        }

        if !fks.is_empty() {
            sqls.append(&mut fks);
        }

        sqls
    }

    /// Generate `ALTER TABLE` queries if changes are detected
    /// on a model.
    ///
    /// - If existing columns are modified, we generate a query to remove the column and replace with the updated version
    /// - If new columns were added, we generate a query to add a new columns
    pub fn sql_alter_table(&self, prev: &Self, bt: &BackendType) -> Option<String> {
        if self == prev {
            return None;
        }

        let table_name = self.name();
        let new_fields = self.fields();
        let prev_fields = prev.fields();

        let mut existing_cols = Vec::new();
        let mut new_cols = Vec::new();

        // determine existing and new columns
        for nf in new_fields {
            let exists = prev_fields.iter().find(|pf| nf.col_name() == pf.col_name());

            match exists {
                Some(exist) => existing_cols.push(exist.col_name()),
                None => new_cols.push(format!(
                    "ALTER TABLE {table_name} ADD COLUMN {};",
                    nf.to_sql(bt)
                )),
            };
        }

        let query_len = existing_cols.len() + new_cols.len() + self.fields.len();
        let mut queries = Vec::with_capacity(query_len);

        // if we changed existing column, attempt to recreate the
        // table and refill with existing data
        if !existing_cols.is_empty() {
            let col_names = existing_cols.join(", ");
            let old_table_name = format!("{table_name}_old");

            // backup table
            let mut query = format!(
                "DROP TABLE IF EXISTS {old_table_name};\nALTER TABLE {table_name} RENAME TO {old_table_name};\n",
            );

            // recreate table
            let field_sqls = self.field_sqls(bt);
            let create_table = format!(
                "CREATE TABLE {table_name} (\n\t{}\n);\n",
                field_sqls.join(",\n\t")
            );
            query.push_str(&create_table);

            // copy data from old to new table
            let move_data = format!(
                "INSERT INTO {table_name} ({col_names}) \nSELECT {col_names} FROM {old_table_name};\n"
            );

            query.push_str(&move_data);
            query.push_str(&format!("DROP TABLE {old_table_name};"));

            queries.push(query);
        }

        // for new columns, generate add column query
        if !new_cols.is_empty() {
            let query = new_cols.join("\n");
            queries.push(query);
        }

        for index in &self.indexes {
            queries.push(index.to_sql(table_name));
        }

        if queries.is_empty() {
            None
        } else {
            Some(queries.join("\n\n"))
        }
    }
}

impl PartialEq for ModelDefinition {
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

#[derive(Default, FromDeriveInput, Debug)]
#[darling(attributes(modeller), supports(struct_named))]
pub struct ModelArgs {
    #[darling(rename = "table_name")]
    pub name: Option<String>,

    #[darling(multiple, default, rename = "index")]
    pub indexes: Vec<IndexArgs>,
}

#[derive(Encode, Decode)]
pub struct IndexDefinition {
    pub fields: Vec<String>,
    pub unique: bool,
    pub name: String,
}

impl IndexDefinition {
    fn to_sql(&self, table_name: &str) -> String {
        let name = &self.name;
        let unique = if self.unique { "UNIQUE" } else { "" };

        format!(
            "CREATE {unique} INDEX IF NOT EXISTS {name} ON {table_name} ({});",
            self.fields.join(", ")
        )
    }
}

#[derive(Default, FromMeta, Debug)]
pub struct IndexArgs {
    pub fields: PathList,
    pub unique: Option<bool>,
    pub name: Option<String>,
}

impl From<&IndexArgs> for IndexDefinition {
    fn from(value: &IndexArgs) -> Self {
        let IndexArgs {
            fields,
            unique,
            name,
        } = value;

        let fields: Vec<String> = fields
            .iter()
            .map(|name| name.to_token_stream().to_string())
            .collect();

        let name = name.clone().unwrap_or(format!("idx_{}", fields.join("_")));

        IndexDefinition {
            fields,
            unique: unique.unwrap_or_default(),
            name,
        }
    }
}
