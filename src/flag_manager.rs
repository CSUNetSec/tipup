use error::TipupError;

#[derive(Debug)]
pub struct Flag {
    timestamp: i64,
    hostname: String,
    ip_address: String,
    domain: String,
    url: String,
    level: Level,
    analyzer: String,
}

#[derive(Debug)]
pub enum Level {
    SEVERE,
    WARNING,
}

pub struct FlagManager {
}

impl FlagManager {
    pub fn new() -> FlagManager {
        FlagManager {
        }
    }

    pub fn process_flag(&mut self, flag: &Flag) -> Result<(), TipupError> {
        println!("TODO PROCESS FLAG: {:?}", flag);

        Ok(())
    }
}
