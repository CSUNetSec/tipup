use bson::Bson;
use bson::ordered::OrderedDocument;
use mongodb::db::{Database, ThreadedDatabase};

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::{Flag, FlagStatus};

use std::collections::HashMap;
use std::sync::mpsc::Sender;

pub struct StdDevAnalyzer {
    name: String,
    variable_name: Vec<String>,
    variable_values: HashMap<String,HashMap<String,Vec<f64>>>, //<vantage,<url,[values]>>
    tx: Sender<Flag>,
}

impl StdDevAnalyzer {
    pub fn new(name: &str, parameters: &OrderedDocument, proddle_db: &Database, tx: Sender<Flag>) -> Result<StdDevAnalyzer, TipupError> {
        //parse parameters to retrieve variable name
        let variable_name = match parameters.get("variable_name") {
            Some(&Bson::Array(ref param_variable_name)) => {
                let mut variable_name = Vec::new();
                for x in param_variable_name {
                    match x {
                        &Bson::String(ref y) => variable_name.push(y.to_owned()),
                        _ => return Err(TipupError::from("failed to parse variable name as String in StdDevAnalzyer")),
                    }
                }

                variable_name
            },
            _ => return Err(TipupError::from("failed to parse variable name parameter in StdDevAnalyzer")),
        };

        //query proddle_db to get information on variable
        //db.results.aggregate([{$match:{measurement:'http-get',timestamp:{$gte:1487785053}}},{$group:{_id:{hostname:'$hostname',url:'$url'},values:{$push:'$result.application_layer_latency'}}}])
        let mut variable_values = HashMap::new();

        let timestamp_gte = doc!("$gte" => 1234); //TODO get last week
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
            variable_values.entry(hostname).or_insert(HashMap::new()).insert(url, values);
        }

        Ok(
            StdDevAnalyzer {
                name: name.to_owned(),
                variable_name: variable_name,
                variable_values: variable_values,
                tx: tx,
            }
        )
    }
}

impl Analyzer for StdDevAnalyzer {
    fn process_result(&mut self, document: &OrderedDocument) -> Result<(), TipupError> {
        //retrieve variables from document
        let hostname = match document.get("hostname") {
            Some(&Bson::String(ref hostname)) => hostname.to_owned(),
            _ => return Ok(()),
        };

        let url = match document.get("url") {
            Some(&Bson::String(ref url)) => url.to_owned(),
            _ => return Ok(()),
        };

        let value = match get_value(document, &self.variable_name) {
            Some(value) => value,
            None => return Ok(()),
        };

        //get list of values from variable values map
        let url_map = self.variable_values.entry(hostname).or_insert(HashMap::new());
        let values = url_map.entry(url).or_insert(Vec::new());

        //compute standard deviation of variable
        let mut mean = 0.0;
        for v in values.iter() {
            mean += *v;
        }
        mean /= values.len() as f64;

        let mut std_dev = 0.0;
        for v in values.iter() {
            std_dev += (*v - mean).powf(2.0);
        }
        std_dev = std_dev.sqrt();

        //if value is greater than 1.5 standard deviations raise warning
        if value > mean + (1.5 * std_dev) {
            let flag = try!(Flag::new(document, FlagStatus::Warning, &self.name));
            try!(self.tx.send(flag));
        }

        //add value to values vector
        values.push(value);
        if values.len() > 10 {
            let _ = values.remove(0);
        }

        Ok(())
    }
}

fn get_value(document: &OrderedDocument, variable_name: &Vec<String>) -> Option<f64> {
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
