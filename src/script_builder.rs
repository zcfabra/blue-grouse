use std::{io::Read, process::{Command, Stdio}};

use crate::{dependent_builder::{DependentObject, ForeignKey}, DBContext};

pub struct ScriptBuilder<'a> {
    pub db_context: &'a DBContext,
}

impl ScriptBuilder<'_> {
    pub fn get_create_script(&self, obj_name: String, obj_type: String) -> Result<String, ()> {
        let _ = std::env::set_var("PGPASSWORD", &self.db_context.password);
        let pg_dump = Command::new("pg_dump")
        .arg("-U")
        .arg(&self.db_context.username)
        .arg("-d")
        .arg(&self.db_context.db_name)
        .arg("-t")
        .arg(obj_name)
        .stdout(Stdio::piped())
        .spawn()
        .expect("SPAWN ERROR");

    let sed_output = Command::new("sed")
        .arg("-n")
        .arg("-e")
        .arg(format!("/^CREATE {}/,/;/p", obj_type))
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


    pub fn get_create_fk_script(&self, fk: ForeignKey) -> Result<String, ()> {
        let _ = std::env::set_var("PGPASSWORD", &self.db_context.password);
        let pg_dump = Command::new("pg_dump")
        .arg("-U")
        .arg(&self.db_context.username)
        .arg("-d")
        .arg(&self.db_context.db_name)
        .arg("-t")
        .arg(fk.get_parent_table_name())
        .arg("--section=post-data")
        .stdout(Stdio::piped())
        .spawn()
        .expect("SPAWN ERROR");

    let mut s: String = String::new();
    let mut stdo = pg_dump.stdout.unwrap(); 
    // let thign =stdo.read_to_string(&mut s).expect("NO");
    // println!("{:?}", s);
    // println!("{}", fk.constraint_name);
    // println!("{}", fk.dependent_column_name);
    let ptn = fk.get_parent_table_name();
    // println!("{ptn}");
    let arg = format!(
            "/^ALTER TABLE.*{}.*/,/.*ADD CONSTRAINT.*{}.*FOREIGN KEY.*{}.*;$/p",
            ptn,
            fk.constraint_name,
            fk.dependent_column_name
        );
    // println!("{arg}");
    let sed_output = Command::new("sed")
        .arg("-n")
        .arg("-e")
        .arg(arg)
        .stdin(stdo)
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