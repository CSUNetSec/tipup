use bson::ordered::OrderedDocument;

pub mod error_analyzer;
pub mod bayesian_analyzer; 

pub use analyzer::error_analyzer::ErrorAnalyzer;
pub use analyzer::bayesian_analyzer::BayesianAnalyzer;

use error::TipupError;

pub trait Analyzer {
    fn process_result(&self, document: &OrderedDocument) -> Result<(), TipupError>;
}
