use bson::Bson;
use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::Flag;

use std::sync::mpsc::Sender;

pub struct BayesianAnalyzer {
    name: String,
    tx: Sender<Flag>,
}

impl BayesianAnalyzer {
    pub fn new(name: &str, _: &Vec<Bson>, tx: Sender<Flag>) -> Result<BayesianAnalyzer, TipupError> {
        Ok(
            BayesianAnalyzer {
                name: name.to_owned(),
                tx: tx,
            }
        )
    }
}

impl Analyzer for BayesianAnalyzer {
    fn process_result(&self, document: &OrderedDocument) -> Result<(), TipupError> {
        unimplemented!();
    }
}
