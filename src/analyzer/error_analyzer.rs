use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;

pub struct ErrorAnalyzer {
}

impl ErrorAnalyzer {
    pub fn new(_: &OrderedDocument) -> Result<ErrorAnalyzer, TipupError> {
        Ok(
            ErrorAnalyzer {
            }
        )
    }
}

impl Analyzer for ErrorAnalyzer {
    fn add_result(&self) -> Result<(), TipupError> {
        unimplemented!();
    }
}
