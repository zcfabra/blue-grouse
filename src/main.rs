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

    let mut script_generator = ScriptBuilder {
        db_context: &dbc,
        file_buffer: String::new()
    };

    let dep_objs = builder
    .get_dependent_objects()
    .await
    .expect("Error getting dependent objects");

    let fks = builder
        .get_foreign_keys()
        .await
        .expect("Error getting foreign keys");

    script_generator.add_buffer_line("-- DELETE DEPENDENTS\n\n");

    for dep_obj in dep_objs.iter() {
        let obj_name = dep_obj.get_full_name();
        let dep_obj_script_header = format!("-- VIEW {}\n", &obj_name); 
        script_generator.add_buffer_line(dep_obj_script_header.as_str());

        let script = script_generator.get_delete_script(
            obj_name, 
            "VIEW".to_string()
        );
        script_generator.add_buffer_line(format!("{}\n\n", script).as_str());
    }

    for fk in fks.iter() {
        let fk_header = format!("-- FK {}\n", fk.constraint_name);
        script_generator.add_buffer_line(&fk_header.as_str());

        let script = script_generator.get_fk_delete_script(fk);
        script_generator.add_buffer_line(&format!("{}\n\n", script));
    }

    for dep_obj in dep_objs.iter() {
        let create_header = format!("-- VIEW {}\n", dep_obj.dependent_view);
        script_generator.add_buffer_line(&create_header);
        match script_generator.get_create_script(
            dep_obj.get_full_name(),
            dep_obj.get_type_name().to_string()
        ) {
            Ok(script) => {
                script_generator.add_buffer_line(&format!("{}\n\n", script));
            },
            Err(_) => println!("Failed to print script for dependent script")
        }
    };
    script_generator.add_buffer_line("\n\n\n\n\n\n-- ADD BACK DEPENDENTS\n\n");
    for fk in fks {
        script_generator.add_buffer_line(&format!("-- FK {}\n", fk.constraint_name));
        match script_generator.get_create_fk_script(fk) {
            Ok(script) => script_generator.file_buffer.push_str(&format!("{}", script)),
            Err(err)=>println!("Failed to print script for FK {}", err)
        }
    } 

    // script_generator.display();
    script_generator.save_file("out.sql".to_string());
    


}
