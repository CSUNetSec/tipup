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

    pub fn add_analyzer(&mut self, name: String, measurement_class: String, analyzer: Box<Analyzer>) -> Result<(), TipupError> {
        let mut analyzers = self.analyzers.lock().unwrap();
        let mut analyzers = analyzers.entry(measurement_class).or_insert(HashMap::new());
        if analyzers.contains_key(&name) {
            return Err(TipupError::from("analyzer name already exists"));
        }

        analyzers.insert(name, analyzer);
        Ok(())
    }

    pub fn send_measurement(&self, document: &OrderedDocument) -> Result<(), TipupError> {
        //get measurement name
        let measurement_class = match document.get("measurement_class") {
            Some(&Bson::String(ref measurement_class)) => measurement_class,
            _ => return Err(TipupError::from("failed to parse result measurement_class")),
        };

        //send to analyzers registered to that measurement
        let mut analyzers = self.analyzers.lock().unwrap();
        if analyzers.contains_key(measurement_class) {
            for analyzer in analyzers.get_mut(measurement_class).unwrap().values_mut() {
                try!(analyzer.process_measurement(document));
            }
        }

        Ok(())
    }
}
