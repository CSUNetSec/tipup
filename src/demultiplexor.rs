use analyzer::Analyzer;
use error::TipupError;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Demultiplexor {
    analyzers: Arc<Mutex<HashMap<String, HashMap<String, Box<Analyzer>>>>>,
}

impl Demultiplexor {
    pub fn new() -> Demultiplexor {
        Demultiplexor {
            analyzers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_analyzer(&mut self, name: String, measurement: String, analyzer: Box<Analyzer>) -> Result<(), TipupError> {
        let mut analyzers = self.analyzers.lock().unwrap();
        let mut analyzers = analyzers.entry(measurement).or_insert(HashMap::new());
        if analyzers.contains_key(&name) {
            return Err(TipupError::from("analyzer name already exists"));
        }

        analyzers.insert(name, analyzer);
        Ok(())
    }

    pub fn send_result(&self) -> Result<(), TipupError> {
        unimplemented!();
    }
}
