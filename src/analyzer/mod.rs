mod error_analyzer;

use error::TipupError;

pub trait Analyzer {
    fn add_result(&self) -> Result<(), TipupError>;
}
