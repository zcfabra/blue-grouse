use std::{fs, io::Read, process::{Command, Stdio}};

use anyhow::{Error, Result};
use regex::Regex;

use crate::{dependent_builder::{DependentObject, ForeignKey}, DBContext};

pub struct ScriptBuilder<'a> {
    pub db_context: &'a DBContext,
    pub file_buffer: String
}

impl ScriptBuilder<'_> {
    fn collapse_spaces(input: &str) -> String {
        let mut result = String::new();
        let mut prev_char: Option<char> = None;
        
        for current_char in input.chars() {
            if current_char != ' ' || prev_char != Some(' ') {
                result.push(current_char);
            }
            prev_char = Some(current_char);
        }
        
        return result;
    }
    pub fn display(&self){ 
        println!("{}", self.file_buffer);
    }

    pub fn save_file(&self, file_path: String) {
        match fs::write(&file_path, &self.file_buffer) {
        Ok(_) => println!("Committed File To {}", &file_path),
        Err(e) => eprintln!("Error writing to {}: {}", file_path, e),
    }
    }

    pub fn add_buffer_line(&mut self, content: &str) {
        self.file_buffer.push_str(content);
    }
    pub fn get_delete_script(&self, obj_name: String, obj_type: String) -> String {
        return format!("DROP {obj_type} {obj_name};");
    }
    pub fn get_fk_delete_script(&self, fk: &ForeignKey) -> String {
        return format!(
            "ALTER TABLE {}\nDROP CONSTRAINT {};",
            fk.get_parent_table_name(), fk.constraint_name
        );
    }
    pub fn get_create_script(&self, obj_name: String, obj_type: String) -> Result<String> {
        let _ = std::env::set_var("PGPASSWORD", &self.db_context.password);
        let pg_dump = Command::new("pg_dump")
            .arg("-h")
            .arg(&self.db_context.host)
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


    pub fn get_create_fk_script(&self, fk: ForeignKey) -> Result<String> {
        let _ = std::env::set_var("PGPASSWORD", &self.db_context.password);
        let pg_dump = Command::new("pg_dump")
        .arg("-h")
        .arg(&self.db_context.host)
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

        let mut stdo = pg_dump.stdout.unwrap(); 
        
        let ptn = fk.get_parent_table_name();
        let mut buf = String::new();
        stdo.read_to_string(&mut buf)?;
        let text_stream: String = buf.chars().filter(|&c| {c != '\n' && c != '\t'}).collect();
        let collapsed_spaces = Self::collapse_spaces(&text_stream);
        let result = format!(
            r"ALTER TABLE.*?{}.*?ADD CONSTRAINT {}.*?;", 
            &ptn, 
            &fk.constraint_name
        ).to_string();
        let re = Regex::new(&result).expect("PARSER ERROR");
        if let Some(captures) = re.captures(&collapsed_spaces) {
            if let Some(res) = captures.get(0) {
                return Ok(res.as_str().to_string());
            }
        }
        return Err(Error::msg("No pattern matches found"));
    }


}