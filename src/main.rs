use std::{env, error::{self, Error}, fmt, fs::File, io::{self, BufRead, Read, Write}, process::{Command, Stdio}, str::FromStr};
use serde_json::Value;
use anyhow::{Result, anyhow};


use colored::Colorize;
use sqlx::{prelude::FromRow, ConnectOptions};
use sqlx_postgres::{PgConnectOptions, PgPool};
use sqlx_core::Url;

mod dependent_builder;
use dependent_builder::DependentBuilder;

mod queries;

mod script_builder;
use script_builder::ScriptBuilder;



// enum ObjectType {
//     VIEW,
//     TABLE,
//     INDEX,
//     FOREIGNKEY 
// }


pub struct DBContext {
    host: String,
    username: String,
    db_name: String,
    password: String
}


impl DBContext{
    fn from_url(url: &Url) -> Result<Self> {
        return Ok(
            DBContext{
                username: url.username().to_string(),
                password: url.password().unwrap().to_string(),
                host: url.host().expect("No host provided").to_string(),
                db_name: url.path().strip_prefix("/").unwrap().to_string()
            }
        )
    }

}






fn get_db_url_from_config() -> Result<String> {
    let mut file = File::open("./config.json")?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;

    let json: Value = serde_json::from_str(&contents)?;

    if let Some(db_url) = json.get("DB_URL") {
        let str_url = db_url.as_str().unwrap();
        return Ok(str_url.to_string());
    }
    return Err(anyhow!("Hi"));
}

#[tokio::main]
async fn main() {
    let db_url  =  get_db_url_from_config().expect("Couldn't parse URL");
    let url: Url = db_url.parse().expect("Could not parse connection string into URL");
    let options = PgConnectOptions::from_url(&url).expect("Error Parsing Connection Options");

    let dbc = DBContext::from_url(&url).expect("Error building context from DB URL");

    let pool = PgPool::connect_with(options)
        .await
        .expect("Failed to connect to DB");

    let builder = DependentBuilder {
        pool: pool,
        table_name: String::from("room"),
        schema_name: String::from("location")
    };

    let script_generator = ScriptBuilder {
        db_context: &dbc
    };

    if let Ok(dep_objs) = builder.get_dependent_objects().await {
        for dep_obj in dep_objs {
            match script_generator.get_create_script(
                dep_obj.get_full_name(),
                dep_obj.get_type_name().to_string()
            ) {

                Ok(script) => println!("{}", script),
                Err(_) => println!("Failed to print script for dependent script")
            }
        }
    };
     
    if let Ok(fks) = builder.get_foreign_keys().await {
        for fk in fks {
            println!("-- FK {}", fk.constraint_name);
            match script_generator.get_create_fk_script(fk) {
                Ok(script) => println!("{}", script),
                Err(_)=>print!("Failed to print script for FK")
            }

        }
    } else {
        print!("OUCH");
    }


}
