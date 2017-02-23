use bson::Bson;
use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Pipe {
    analyzers: Arc<Mutex<HashMap<String, HashMap<String, Box<Analyzer>>>>>,
}

impl Pipe {
    pub fn new() -> Pipe {
        Pipe {
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

    pub fn send_result(&self, document: &OrderedDocument) -> Result<(), TipupError> {
        //get measurement name
        let measurement = match document.get("measurement") {
            Some(&Bson::String(ref measurement)) => measurement,
            _ => return Err(TipupError::from("failed to parse result measurement")),
        };

        //send to analyzers registered to that measurement
        let mut analyzers = self.analyzers.lock().unwrap();
        if analyzers.contains_key(measurement) {
            for analyzer in analyzers.get_mut(measurement).unwrap().values_mut() {
                try!(analyzer.process_result(document));
            }
        }

        Ok(())
    }
}
