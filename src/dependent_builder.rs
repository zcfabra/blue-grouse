use std::fmt::{self};

use colored::Colorize;
use sqlx::prelude::FromRow;
use sqlx_postgres::PgPool;

use crate::queries::*;


#[derive(Debug)]
enum ObjectType {
    VIEW,
    TRIGGER
}

#[derive(FromRow, Debug)]
pub struct DependentObject {
    pub dependent_schema: String,
    pub dependent_view: String,
    pub source_schema: String,
    pub source_table: String,
    pub column_names: Vec<String>,
    // pub object_type: ObjectType
}

impl DependentObject {
    pub fn get_full_name(&self) -> String {
        // TODO: refactor to field in constructor 
        return format!(
            "{}.{}", 
            &self.dependent_schema, 
            &self.dependent_view
        );
    }
    pub fn get_type_name(&self) -> &str {
        return "VIEW";
        // match &self.object_type {
        //     ObjectType::TRIGGER => "TRIGGER",
        //     ObjectType::VIEW => "VIEW",
        // }
    }
}

impl std::fmt::Display for DependentObject {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cols_str = String::new();
        for (ix, col_name) in self.column_names.iter().enumerate() {
            let fmt_col_name = if ix != 0 {
                format!(", {}", col_name)
            } else {
                col_name.to_string()
            };
            cols_str.push_str(&fmt_col_name)
        }
        println!(
            "{} depends on {} via columns ({})",
            format!("{}.{}",self.dependent_schema, self.dependent_view).bold().bright_magenta(),
            format!("{}.{}",self.source_schema, self.source_table).bold().bright_magenta(),
            cols_str.bold().blue()
        );
        return Ok(());
    }

}

#[derive(FromRow, Debug)]
pub struct ForeignKey {
    pub constraint_name: String,
    pub dependent_table_schema: String,
    pub dependent_table_name: String,
    pub dependent_column_name: String,
    pub foreign_table_schema: String,
    pub foreign_table_name: String,
    pub foreign_column_name: String
}

impl ForeignKey {
    pub fn get_parent_table_name(&self) -> String {
        return format!(
            "{}.{}", &self.dependent_table_schema, &self.dependent_table_name
        )
    }
}
impl std::fmt::Display for ForeignKey {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        println!(
            "{} {} ({}.{}) --> {} ({}.{})",
            format!("{} {}:", "FK", self.constraint_name).bold().bright_magenta(),
            self.dependent_column_name.bold(),
            self.dependent_table_schema,
            self.dependent_table_name,
            self.foreign_column_name.bold(),
            self.foreign_table_schema,
            self.foreign_table_name
        );
        return Ok(());
    }

}
pub struct DependentBuilder {
    pub pool: PgPool,
    pub table_name: String,
    pub schema_name: String
}
impl DependentBuilder {
    pub async fn list_dependent_objects(&self) -> () {
        match self.get_dependent_objects().await {
            Ok(objects)=>{
                for obj in objects {
                    println!("{}", obj);
                }
            },
            Err(err) => {
                println!("{:?}", err);
            }
        }
    }

    pub async fn get_dependent_objects(&self) -> Result<Vec<DependentObject>, sqlx::Error>{
        let rows: Vec<DependentObject>= sqlx::query_as(GET_DEPENDENT_OBJECTS)
            .bind(&self.schema_name)
            .bind(&self.table_name)
            .fetch_all(&self.pool)
            .await?;

        return Ok(rows);
    }

    pub async fn list_foreign_keys(&self) -> () {
        match self.get_foreign_keys().await {
            Ok(keys) => {
                for key in keys {
                    println!("{}", key);
                }
            },
            Err(err) => {
                println!("{:?}", err);
            }
    };

    }

    pub async fn get_foreign_keys(&self) -> Result<Vec<ForeignKey>, sqlx::Error> {
        let rows: Vec<ForeignKey> = sqlx::query_as(GET_FOREIGN_KEYS)
            .bind(&self.schema_name)
            .bind(&self.table_name)
            .fetch_all(&self.pool)
            .await?;
        return Ok(rows);
    }
        
} 