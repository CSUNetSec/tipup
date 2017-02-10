pub mod error_analyzer;
pub mod bayesian_analyzer; 

pub use analyzer::error_analyzer::ErrorAnalyzer;
pub use analyzer::bayesian_analyzer::BayesianAnalyzer;

use error::TipupError;

pub trait Analyzer {
    fn add_result(&self) -> Result<(), TipupError>;
}
