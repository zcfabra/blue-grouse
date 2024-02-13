use std::{env, error::Error, fmt, io::{self, BufRead, Write}, process::{Command, Stdio}};
use url::Url;
use regex::Regex;


use colored::Colorize;
use sqlx::prelude::FromRow;
use sqlx_postgres::PgPool;

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

// enum ObjectType {
//     VIEW,
//     TABLE,
//     INDEX,
//     FOREIGNKEY 
// }
struct ScriptBuilder {
    object_name: String, 
    object_type: String,
}

struct DBContext {
    host: String,
    username: String,
    db_name: String,
    password: String
}


impl ScriptBuilder {
    fn get_create_script(&self, db_context: &DBContext) -> Result<String, ()> {
        let _ = std::env::set_var("PGPASSWORD", &db_context.password);
        let pg_dump = Command::new("pg_dump")
        .arg("-U")
        .arg(&db_context.username)
        .arg("-d")
        .arg(&db_context.db_name)
        .arg("-t")
        .arg(&self.object_name)
        .stdout(Stdio::piped())
        .spawn()
        .expect("SPAWN ERROR");

    let sed_output = Command::new("sed")
        .arg("-n")
        .arg("-e")
        .arg("/^CREATE VIEW/,/;/p")
        .stdin(pg_dump.stdout.unwrap())
        .output()
        .expect("ERROR SPAWNING SED");

    // Read the output of pg_dump
    return Ok(String::from_utf8(sed_output.stdout).expect("Should be able to send bytes to string"));
    // let status = pg_dump.wait().expect("CANTE");
    // if !status.success() {
    //     eprintln!("pg_dump failed with exit code: {}", status);
    //     std::process::exit(1);
    // }
    }

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

fn parse_postgres_url(url: &str) -> Result<DBContext, Box<dyn Error>> {
    let parsed_url = Url::parse(url)?;

    let host = parsed_url.host_str().ok_or("No host found")?.to_string();
    let username = parsed_url.username().to_string();
    let db_name = parsed_url.path().trim_start_matches('/').to_string();

    let password = match parsed_url.password() {
        Some(password) => password.to_string(),
        None => String::new(),
    };

    Ok(DBContext {
        host,
        username,
        db_name,
        password,
    })
}

#[tokio::main]
async fn main() {
    let key = "DB_URL";
    let db_url_opt = env::var_os(key).expect("No DB URL Provided");
    if let Some(db_url) = db_url_opt.to_str() {
        let db_context = parse_postgres_url(db_url).unwrap();
        let pool = PgPool::connect(db_url).await.expect("Failed to connect to DB");
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
                    // let sb = ScriptBuilder{
                    //     object_name: format!("{}.{}", obj.dependent_schema, obj.dependent_view),
                    //     object_type: String::from("VIEW")
                    // };
                    // if let Ok(script) = sb.get_create_script(&db_context) {
                    //     println!("{script}");
                    // } else {
                    //     println!("ERROR GETTING SCRIPT");
                    // }
                }
            },
            Err(err) => {
                println!("{:?}", err);
            }
        }
    }
    
}
