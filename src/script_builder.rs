use core::num;
use std::{fs, io::Read, process::{Command, Stdio}, sync::{mpsc, Arc, Mutex}, thread};

use anyhow::{Error, Result};
use regex::Regex;

use crate::{dependent_builder::{DependentObject, ForeignKey}, DBContext};

#[derive(Clone)]
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
    fn fmt_query(query: &str) -> Result<String> {
        let replacements = [
            (" ADD CONSTRAINT", "\nADD CONSTRAINT"),
            (" FOREIGN KEY", "\nFOREIGN KEY"),
            (" REFERENCES", "\nREFERENCES"),
            (" CHECK", "\nCHECK"),
        ];
        let mut result = query.to_string();
        for (pattern, value) in replacements {
            let re = Regex::new(pattern)?;
            result = re.replace_all(&result, value).to_string();
        }
        return Ok(result);


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
    pub fn get_create_script(dep_obj: &DependentObject, db_context: &DBContext) -> Result<String> {
        let _ = std::env::set_var("PGPASSWORD", &db_context.password);
        let pg_dump = Command::new("pg_dump")
            .arg("-h")
            .arg(&db_context.host)
            .arg("-U")
            .arg(&db_context.username)
            .arg("-d")
            .arg(&db_context.db_name)
            .arg("-t")
            .arg(&dep_obj.get_full_name())
            .stdout(Stdio::piped())
            .spawn()
            .expect("SPAWN ERROR");

    let sed_output = Command::new("sed")
        .arg("-n")
        .arg("-e")
        .arg(format!("/^CREATE {}/,/;/p", &dep_obj.get_type_name()))
        .stdin(pg_dump.stdout.unwrap())
        .output()
        .expect("ERROR SPAWNING SED");

        // Read the output of pg_dump
        let final_script = format!(
            "-- VIEW {}\n\n{}",
            &dep_obj.get_full_name(),
            String::from_utf8(sed_output.stdout).expect("Should be able to send bytes to string")
        );
        return Ok(final_script);
    }


    pub fn get_create_fk_script(fk: &ForeignKey, db_context: &DBContext) -> Result<String> {
        let _ = std::env::set_var("PGPASSWORD", &db_context.password);
        let pg_dump = Command::new("pg_dump")
        .arg("-h")
        .arg(&db_context.host)
        .arg("-U")
        .arg(&db_context.username)
        .arg("-d")
        .arg(&db_context.db_name)
        .arg("-t")
        .arg(fk.get_parent_table_name())
        .arg("--section=post-data")
        .stdout(Stdio::piped())
        .spawn()
        .expect("SPAWN ERROR");

        let mut stdo = pg_dump.stdout.unwrap(); 

        // Get a well formatted file
        let ptn = fk.get_parent_table_name();
        let mut buf = String::new();
        stdo.read_to_string(&mut buf)?;
        let text_stream: String = buf.chars().filter(|&c| {c != '\n' && c != '\t'}).collect();
        let collapsed_spaces = Self::collapse_spaces(&text_stream);

        // Regex
        let pattern = format!(
            r"ALTER TABLE.*?{} ADD CONSTRAINT {}.*?;", 
            &ptn, 
            &fk.constraint_name
        );
        let re = Regex::new(&pattern).expect("Parser Error");
        if let Some(captures) = re.captures(&collapsed_spaces) {
            if let Some(res) = captures.get(0) {
                let formatted_query = Self::fmt_query(res.as_str())?;
                return Ok(
                    format!(
                        "-- FK {}\n\n{}",
                        fk.constraint_name,
                        formatted_query
                    )
                );
            }
        }
        return Err(Error::msg("No pattern matches found in FK create script"));
    }


    pub fn get_fk_create_scripts(&mut self, fks: Vec<ForeignKey>) -> Result<()> {
        self.add_buffer_line("\n/* --- FOREIGN KEYS --- */\n\n\n");
        let (sender, receiver ) = mpsc::channel::<Result<(usize, String)>>();
        
        let num_items = fks.len();
        let fks = Arc::new(Mutex::new(fks));
        let mut handles = Vec::new();

        for ix in 0..num_items {
            let fks = Arc::clone(&fks);
            let db_context = self.db_context.clone();
            let sender = sender.clone();
            let th = thread::spawn(move || {
                let items = fks.lock().unwrap();
                let item = items.get(ix).unwrap();
                if let Ok(str) = Self::get_create_fk_script(item, &db_context) {
                    let _ = sender.send(Ok((ix, str)));
                } else {
                    let _ = sender.send(Err(Error::msg("Error processing fk script")));
                }
            });
            handles.push(th);
        }

        for th in handles {
            th.join().unwrap();
        }

        let mut results = Vec::new();
        for _ in 0..num_items {
            match receiver.recv() {
                Ok(res) => {
                    match res {
                        Ok(tuple) => results.push(tuple),
                        Err(_) => println!("ERR")
                    }
                },
                Err(_) => println!("ERR"),
            }
        }
        results.sort();
        for res in results {
            self.add_buffer_line(format!("{}\n",&res.1).as_str());
        }
        

        return Ok(());
    }
    pub fn get_dependent_object_create_scripts(
        &mut self, dependent_objects: Vec<DependentObject>
    ) -> Result<()> {
        self.add_buffer_line("\n/* --- VIEWS --- */\n\n\n");
        // Multithreaded calling of pg_dump to extract create scripts
        let (sender, receiver) = mpsc::channel::<Result<(usize, String)>>();
        let inputs_arc = Arc::new(Mutex::new(dependent_objects));
        let dbc = Arc::new(Mutex::new(self.db_context.clone()));

        let mut handles = Vec::new();

        for (index, _) in inputs_arc.lock().unwrap().iter().enumerate() {
            let sender = sender.clone();
            let items_to_process = Arc::clone(&inputs_arc);
            let builder_instance = Arc::clone(&dbc);


            let th = thread::spawn(move || {
                let items = items_to_process.lock().unwrap();
                let item = items.get(index).unwrap();
                if let Ok(str) = Self::get_create_script(item, &builder_instance.lock().unwrap()) {
                    let _ = sender.send(Ok((index, str)));
                } else {
                    let _ = sender.send(Err(Error::msg("Error ")));
                }
            });
            handles.push(th);
        };
        // Collect results
        for th in handles {
            th.join().expect("Thread error");
        }

        let mut results = Vec::new();
        for _ in 0..inputs_arc.lock().unwrap().len() {
            match receiver.recv() {
                Ok(res) => {
                    match res {
                        Ok(tuple) => results.push(tuple),
                        Err(_) => println!("ERR")
                    }
                },
                Err(_) => println!("ERR"),
            }
        }
        results.sort();
        for res in results {
            self.add_buffer_line(format!("{}\n",&res.1).as_str());
        }
        
        return Ok(());
    }
}