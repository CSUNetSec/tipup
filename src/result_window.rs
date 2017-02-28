use bson::Bson;
use bson::ordered::OrderedDocument;
use mongodb::db::{Database, ThreadedDatabase};
use time;

use error::TipupError;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ResultWindow {
    variable_windows: Vec<Arc<RwLock<VariableWindow>>>,
}

impl ResultWindow {
    pub fn new() -> ResultWindow {
        ResultWindow {
            variable_windows: Vec::new(),
        }
    }

    pub fn register_variable(&mut self, variable_name: &Vec<String>) -> Result<Arc<RwLock<VariableWindow>>, TipupError> {
        //check if variable_name already exists
        for variable_window in self.variable_windows.iter() {
            {
                let variable_window_clone = variable_window.read().unwrap();
                if variable_window_clone.variable_name_equals(&variable_name) {
                    return Ok(variable_window.clone());
                }
            }
        }

        //create new variable window
        let variable_window = Arc::new(RwLock::new(VariableWindow::new(variable_name.to_owned())));
        self.variable_windows.push(variable_window.clone());
        Ok(variable_window)
    }

    pub fn initialize(&mut self, proddle_db: &Database) -> Result<(), TipupError> {
        for variable_window in self.variable_windows.iter() {
            {
                let mut variable_window = variable_window.write().unwrap();
                try!(variable_window.initialize(proddle_db));
            }
        }

        Ok(())
    }

    pub fn add_result(&mut self, document: OrderedDocument) -> Result<(), TipupError> {
        //parse hostname and url
        let (hostname, url);
        {
            hostname = match document.get("hostname") {
                Some(&Bson::String(ref hostname)) => hostname.to_owned(),
                _ => return Err(TipupError::from("failed to parse hostname from _id document")),
            };

            url = match document.get("url") {
                Some(&Bson::String(ref url)) => url.to_owned(),
                _ => return Err(TipupError::from("failed to pase url from _id document")),
            };
        }

        //add document to variable windows
        for variable_window in self.variable_windows.iter() {
            {
                let mut variable_window = variable_window.write().unwrap();
                try!(variable_window.add_result(&hostname, &url, &document));
            }
        }

        Ok(())
    }
}

pub struct VariableWindow {
    variable_name: Vec<String>,
    values: HashMap<String, HashMap<String, Vec<f64>>>,
}

impl VariableWindow {
    fn new(variable_name: Vec<String>) -> VariableWindow {
        VariableWindow {
            variable_name: variable_name,
            values: HashMap::new(),
        }
    }

    fn initialize(&mut self, proddle_db: &Database) -> Result<(), TipupError> {
        let start_time = time::now_utc().to_timespec().sec - (60 * 60 * 24 * 5);
        let timestamp_gte = doc!("$gte" => start_time);
        let match_doc = doc!("measurement" => "http-get", "timestamp" => timestamp_gte);
        let id_doc = doc!("hostname" => "$hostname", "url" => "$url");
        let values_doc = doc!("$push" => "$result.application_layer_latency");
        let group_doc = doc!("_id" => id_doc, "values" => values_doc);
        let aggregate_doc = vec!(
            doc!("$match" => match_doc),
            doc!("$group" => group_doc),
        );

        for document in try!(proddle_db.collection("results").aggregate(aggregate_doc, None)) {
            //retrieve variable values from bson document
            let document = try!(document);

            let id_document = match document.get("_id") {
                Some(&Bson::Document(ref id_document)) => id_document,
                _ => continue,
            };

            let hostname = match id_document.get("hostname") {
                Some(&Bson::String(ref hostname)) => hostname.to_owned(),
                _ => continue,
            };

            let url = match id_document.get("url") {
                Some(&Bson::String(ref url)) => url.to_owned(),
                _ => continue,
            };

            let values = match document.get("values") {
                Some(&Bson::Array(ref array)) => {
                    let mut values = Vec::new();
                    for value in array {
                        match value {
                            &Bson::FloatingPoint(f) => values.push(f),
                            &Bson::I32(i) => values.push(i as f64),
                            &Bson::I64(i) => values.push(i as f64),
                            _ => continue,
                        }
                    }

                    values
                },
                _ => continue,
            };

            //insert values into variable values map
            self.values.entry(hostname).or_insert(HashMap::new()).insert(url, values);
        }

        Ok(())
    }

    fn add_result(&mut self, hostname: &str, url: &str, document: &OrderedDocument) -> Result<(), TipupError> {
        if let Some(value) = get_value(&self.variable_name, document) {
            let values = self.values.entry(hostname.to_owned()).or_insert(HashMap::new()).entry(url.to_owned()).or_insert(Vec::new());
            values.push(value);
            if values.len() > 10 {
                values.remove(0);
            }
        }

        Ok(())
    }

    pub fn get_values(&self, hostname: &str, url: &str) -> Option<&Vec<f64>> {
        if let Some(url_map) = self.values.get(hostname) {
            if let Some(results) = url_map.get(url) {
                return Some(results);
            }
        }

        None
    }

    fn variable_name_equals(&self, variable_name: &Vec<String>) -> bool {
        //check length
        if self.variable_name.len() != variable_name.len() {
            return false;
        }

        //check name
        for i in 0..self.variable_name.len() {
            if self.variable_name[i] != variable_name[i] {
                return false;
            }
        }

        true
    }
}

fn get_value(variable_name: &Vec<String>, document: &OrderedDocument) -> Option<f64> {
    let mut index_document = document;
    for variable in variable_name {
        match index_document.get(variable) {
            Some(&Bson::Document(ref document)) => index_document = document,
            Some(&Bson::FloatingPoint(f)) => return Some(f),
            Some(&Bson::I32(i)) => return Some(i as f64),
            Some(&Bson::I64(i)) => return Some(i as f64),
            _ => return None,
        }
    }

    None
}
/*pub struct ResultWindow {
    results: HashMap<String, HashMap<String, Vec<OrderedDocument>>>,
}

impl ResultWindow {
    pub fn new() -> ResultWindow {
        ResultWindow {
            results: HashMap::new(),
        }
    }

    pub fn add_result(&mut self, document: OrderedDocument) -> Result<(), TipupError> {
        let (hostname, url);
        {
            hostname = match document.get("hostname") {
                Some(&Bson::String(ref hostname)) => hostname.to_owned(),
                _ => return Err(TipupError::from("failed to parse hostname from _id document")),
            };

            url = match document.get("url") {
                Some(&Bson::String(ref url)) => url.to_owned(),
                _ => return Err(TipupError::from("failed to pase url from _id document")),
            };
        }

        //insert values into variable values map
        let mut url_map = self.results.entry(hostname).or_insert(HashMap::new());
        let mut results_vec = url_map.entry(url).or_insert(Vec::new());
        results_vec.push(document);

        if results_vec.len() > 10 {
            results_vec.remove(0);
        }

        Ok(())
    }

    pub fn get_results(&self, vantage: &str, url: &str) -> Option<&Vec<OrderedDocument>> {
        if let Some(url_map) = self.results.get(vantage) {
            if let Some(results_vec) = url_map.get(url) {
                return Some(results_vec);
            }
        }

        None
    }
}*/
