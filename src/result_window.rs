use bson::Bson;
use bson::ordered::OrderedDocument;

use error::TipupError;

use std::collections::HashMap;

pub struct ResultWindow {
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
}
