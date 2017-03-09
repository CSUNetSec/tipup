use bson::Bson;
use bson::ordered::OrderedDocument;
use chan::Sender;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::{Flag, FlagStatus};
use result_window::{ResultWindow, VariableWindow};

use std::sync::{Arc, RwLock};

pub struct StdDevAnalyzer {
    name: String,
    variable_name: Vec<String>,
    variable_window: Arc<RwLock<VariableWindow>>,
    flag_tx: Sender<Flag>,
}

impl StdDevAnalyzer {
    pub fn new(name: &str, parameters: &OrderedDocument, result_window: Arc<RwLock<ResultWindow>>, flag_tx: Sender<Flag>, ) -> Result<StdDevAnalyzer, TipupError> {
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

        let variable_window;
        {
            let mut result_window = result_window.write().unwrap();
            variable_window = try!(result_window.register_variable(&variable_name));
        }

        Ok(
            StdDevAnalyzer {
                name: name.to_owned(),
                variable_name: variable_name,
                variable_window: variable_window,
                flag_tx: flag_tx,
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

        let value = match get_value(&self.variable_name, document) {
            Some(value) => value,
            None => return Ok(()),
        };

        {
            //get list of values from result window
            let variable_window = self.variable_window.read().unwrap();
            let values: &Vec<f64> = match variable_window.get_values(&hostname, &url) {
                Some(values) => values,
                None => return Ok(()),
            };

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
                self.flag_tx.send(flag);
            }
        }

        Ok(())
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
