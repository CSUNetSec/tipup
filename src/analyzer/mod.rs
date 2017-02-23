use bson::ordered::OrderedDocument;

pub mod error_analyzer;
pub mod std_dev_analyzer; 

pub use analyzer::error_analyzer::ErrorAnalyzer;
pub use analyzer::std_dev_analyzer::StdDevAnalyzer;

use error::TipupError;

pub trait Analyzer {
    fn process_result(&mut self, document: &OrderedDocument) -> Result<(), TipupError>;
}
