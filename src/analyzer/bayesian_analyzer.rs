use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;

pub struct BayesianAnalyzer {
}

impl BayesianAnalyzer {
    pub fn new(_: &OrderedDocument) -> Result<BayesianAnalyzer, TipupError> {
        Ok(
            BayesianAnalyzer {
            }
        )
    }
}

impl Analyzer for BayesianAnalyzer {
    fn add_result(&self) -> Result<(), TipupError> {
        unimplemented!();
    }
}
