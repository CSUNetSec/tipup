use error::TipupError;

pub struct Flag {
}

pub struct FlagManager {

}

impl FlagManager {
    pub fn new() -> FlagManager {
        FlagManager {
        }
    }

    pub fn process_flag(&mut self, flag: &Flag) -> Result<(), TipupError> {
        unimplemented!();
    }
}
