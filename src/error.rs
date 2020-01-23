use co2mon::Error as Co2monError;
use config::ConfigError;
use reqwest::Error as ReqwestError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("Error with CO2 sensor: {}", source))]
    Co2mon { source: Co2monError },

    #[snafu(display("Config error: {}", source))]
    Config { source: ConfigError },

    #[snafu(display("Data error: {}", description))]
    Data { description: String },

    #[snafu(display("Reqwest error: {}", source))]
    Reqwest { source: ReqwestError },

    #[snafu(display("User error: {}", description))]
    User { description: String },
}

impl From<Co2monError> for Error {
    fn from(source: Co2monError) -> Self {
        Error::Co2mon { source }
    }
}

impl From<ConfigError> for Error {
    fn from(source: ConfigError) -> Self {
        Error::Config { source }
    }
}

impl From<ReqwestError> for Error {
    fn from(source: ReqwestError) -> Self {
        Error::Reqwest { source }
    }
}
