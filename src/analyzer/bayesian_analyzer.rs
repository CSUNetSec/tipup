use bson::Bson;
use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::Flag;

use std::sync::mpsc::Sender;

pub struct BayesianAnalyzer {
    tx: Sender<Flag>,
}

impl BayesianAnalyzer {
    pub fn new(_: &Vec<Bson>, tx: Sender<Flag>) -> Result<BayesianAnalyzer, TipupError> {
        Ok(
            BayesianAnalyzer {
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
