use error::TipupError;

#[derive(Debug)]
pub struct Flag {
    timestamp: i64,
    hostname: String,
    ip_address: String,
    domain: String,
    url: String,
    level: u8,
    analyzer: String,
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
