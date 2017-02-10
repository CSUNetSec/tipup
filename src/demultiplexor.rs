use mongodb::db::Database;

use analyzer::Analyzer;
use error::TipupError;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Demultiplexor {
    analyzers: Arc<Mutex<HashMap<String, HashMap<String, Box<Analyzer>>>>>,
    db: Database,
}

impl Demultiplexor {
    pub fn new(db: Database) -> Demultiplexor {
        Demultiplexor {
            analyzers: Arc::new(Mutex::new(HashMap::new())),
            db: db,
        }
    }

    pub fn add_analyzer(&mut self, measurement: String, name: String, analyzer: Box<Analyzer>) -> Result<(), TipupError> {
        let mut analyzers = self.analyzers.lock().unwrap();
        let mut analyzers = analyzers.entry(measurement).or_insert(HashMap::new());
        if analyzers.contains_key(&name) {
            return Err(TipupError::from("analyzer name already exists"));
        }

        analyzers.insert(name, analyzer);
        Ok(())
    }

    pub fn fetch(&self) -> Result<(), TipupError> {
        unimplemented!();
    }
}
