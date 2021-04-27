use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct SolarSystemInfoBotError {
    message: String,
}

impl SolarSystemInfoBotError {
    pub fn new(msg: &str) -> Self {
        SolarSystemInfoBotError {
            message: msg.to_string(),
        }
    }
}

impl fmt::Display for SolarSystemInfoBotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SolarSystemInfoBotError {}
