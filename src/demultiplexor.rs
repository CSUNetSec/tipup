use analyzer::Analyzer;
use error::TipupError;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Demultiplexor {
    analyzers: Arc<Mutex<HashMap<String, Box<Analyzer>>>>,
}

impl Demultiplexor {
    pub fn new() -> Demultiplexor {
        Demultiplexor {
            analyzers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_analyzer(&mut self, name: String, analyzer: Box<Analyzer>) -> Result<(), TipupError> {
        let mut analyzers = self.analyzers.lock().unwrap();
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
