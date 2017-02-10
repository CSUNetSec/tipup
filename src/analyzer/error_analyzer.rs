use analyzer::Analyzer;
use error::TipupError;

pub struct ErrorAnalyzer {

}

impl Analyzer for ErrorAnalyzer {
    fn add_result(&self) -> Result<(), TipupError> {
        unimplemented!();
    }
}
