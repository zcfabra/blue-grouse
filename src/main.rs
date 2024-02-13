use std::fmt::{self, Display};

use colored::Colorize;
use sqlx::prelude::FromRow;
use sqlx_postgres::PgPool;

const DB_URL: &str = "postgres://postgres:postgres@127.0.0.1:5432/pgrs";

const GET_FOREIGN_KEYS: &str = "
SELECT
    tc.table_schema AS dependent_table_schema, 
    tc.constraint_name, 
    tc.table_name AS dependent_table_name, 
    kcu.column_name AS dependent_column_name, 
    ccu.table_schema AS foreign_table_schema,
    ccu.table_name AS foreign_table_name,
    ccu.column_name AS foreign_column_name 
FROM information_schema.table_constraints AS tc 
JOIN information_schema.key_column_usage AS kcu
    ON tc.constraint_name = kcu.constraint_name
    AND tc.table_schema = kcu.table_schema
JOIN information_schema.constraint_column_usage AS ccu
    ON ccu.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY'
    AND ccu.table_schema=$1
    AND ccu.table_name=$2;
";

const GET_DEPENDENT_OBJECTS: &str = "
SELECT dependent_ns.nspname as dependent_schema
, dependent_view.relname as dependent_view 
, source_ns.nspname as source_schema
, source_table.relname as source_table
, ARRAY_AGG(pg_attribute.attname) as column_names
FROM pg_depend 
JOIN pg_rewrite ON pg_depend.objid = pg_rewrite.oid 
JOIN pg_class as dependent_view ON pg_rewrite.ev_class = dependent_view.oid 
JOIN pg_class as source_table ON pg_depend.refobjid = source_table.oid 
JOIN pg_attribute ON pg_depend.refobjid = pg_attribute.attrelid 
    AND pg_depend.refobjsubid = pg_attribute.attnum 
JOIN pg_namespace dependent_ns ON dependent_ns.oid = dependent_view.relnamespace
JOIN pg_namespace source_ns ON source_ns.oid = source_table.relnamespace
WHERE 
source_ns.nspname = $1
AND source_table.relname = $2
GROUP BY
	dependent_ns.nspname,
	dependent_view.relname,
	source_ns.nspname,
	source_table.relname
ORDER BY 1,2;
";

struct Builder {
    pool: PgPool,
    table_name: String,
    schema_name: String
}

#[derive(FromRow, Debug)]
struct DependentObject {
    dependent_schema: String,
    dependent_view: String,
    source_schema: String,
    source_table: String,
    column_names: Vec<String>
}

impl std::fmt::Display for DependentObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            format!("{}.{}",self.dependent_schema, self.dependent_view).bold().red(),
            format!("{}.{}",self.source_schema, self.source_table).bold().red(),
            cols_str.bold().blue()
        );
        return Ok(());
    }

}

#[derive(FromRow, Debug)]
struct ForeignKey {
    constraint_name: String,
    dependent_table_schema: String,
    dependent_table_name: String,
    dependent_column_name: String,
    foreign_table_schema: String,
    foreign_table_name: String,
    foreign_column_name: String
}

impl std::fmt::Display for ForeignKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        println!(
            "{} {} ({}.{}) --> {} ({}.{})",
            format!("{} {}:", "FK", self.constraint_name).bold().red(),
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

impl Builder {
    async fn get_dependent_objects(&self) -> Result<Vec<DependentObject>, sqlx::Error>{
        let rows: Vec<DependentObject>= sqlx::query_as(GET_DEPENDENT_OBJECTS)
            .bind(&self.schema_name)
            .bind(&self.table_name)
            .fetch_all(&self.pool)
            .await?;

        return Ok(rows);
    }

    async fn get_foreign_keys(&self) -> Result<Vec<ForeignKey>, sqlx::Error> {
        let rows: Vec<ForeignKey> = sqlx::query_as(GET_FOREIGN_KEYS)
            .bind(&self.schema_name)
            .bind(&self.table_name)
            .fetch_all(&self.pool)
            .await?;
        return Ok(rows);
    }
        
} 

#[tokio::main]
async fn main() {
    let pool = PgPool::connect(DB_URL).await.expect("Failed to connect to DB");

    let b = Builder{
        pool:pool,
        table_name: String::from("room"),
        schema_name: String::from("location")
    };

    match b.get_foreign_keys().await {
        Ok(keys) => {
            for key in keys {
                println!("{}", key);
            }
        },
        Err(err) => {
            println!("{:?}", err);
        }
        
    };

    match b.get_dependent_objects().await {
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
