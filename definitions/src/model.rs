use crate::{backend_type::BackendType, field::FieldDefinition};
use bincode::{Decode, Encode, config};
use darling::{FromDeriveInput, util::PathList};

#[derive(Default, FromDeriveInput, Debug)]
#[darling(attributes(modeller), supports(struct_named))]
pub struct ModelArgs {
    #[darling(rename = "table_name")]
    pub name: Option<String>,
    pub unique_together: Option<PathList>,
}

#[derive(Encode, Decode)]
pub struct ModelDefinition {
    pub name: String,
    pub unique_together: Option<Vec<String>>,
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
        let mut field_sqls = self.field_sqls(bt);

        if let Some(ut_sql) = self.unique_together_sql(bt) {
            field_sqls.push(ut_sql);
        }

        format!(
            "DROP TABLE IF EXISTS {table_name};\n\
            CREATE TABLE {table_name} (\n\t{}\n);
            ",
            field_sqls.join(",\n\t")
        )
    }

    pub fn field_sqls(&self, bt: &BackendType) -> Vec<String> {
        self.fields()
            .iter()
            .map(|field| field.to_sql(bt).trim().to_string())
            .collect()
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

        let mut queries = Vec::new();
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
            let mut field_sqls = self.field_sqls(bt);
            if let Some(ut_sql) = self.unique_together_sql(bt) {
                field_sqls.push(ut_sql);
            }

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

        if queries.is_empty() {
            None
        } else {
            Some(queries.join("\n\n"))
        }
    }

    fn unique_together_sql(&self, bt: &BackendType) -> Option<String> {
        use BackendType::*;

        if let Some(cols) = &self.unique_together {
            // let cols: Vec<&str> = ut.split(",").map(|c| c.trim()).collect();
            if !cols.is_empty() {
                let name = cols.join("_");
                let list = cols.join(", ");
                let sql = match bt {
                    MySql => format!("UNIQUE KEY {name} ({list})"),
                    Postgres => format!("CONSTRAINT {name} UNIQUE ({list})"),
                    Sqlite => format!("UNIQUE ({list})"),
                };

                return Some(sql);
            }
        }

        None
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
