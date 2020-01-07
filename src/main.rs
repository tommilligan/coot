use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use co2mon::{Error as Co2monError, OpenOptions as SensorOptions, Reading, Sensor};
use config;
use env_logger::Env;
use log::{debug, error, warn};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Url;
use serde_derive::{Deserialize, Serialize};

mod error;

use error::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    interval: u64,
    influxdb_url: String,
    influxdb_token: String,
    influxdb_bucket: String,
    influxdb_org: String,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        debug!("Loading settings");

        let mut settings = config::Config::default();
        settings.set_default("interval", 5)?;
        settings.merge(config::File::with_name("coot.yml"))?;
        // settings.merge(config::Environment::with_prefix("COOT"))?;

        // Print out our settings (as a HashMap)
        let settings = settings.try_into::<Self>()?;
        Ok(settings)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Datum {
    temperature: f32,
    co2: u16,
    timestamp: u64,
}

impl From<Reading> for Datum {
    fn from(reading: Reading) -> Self {
        Datum {
            temperature: reading.temperature(),
            co2: reading.co2(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        }
    }
}

pub struct Data {
    /// Configuration of how to acquire the sensor.
    sensor_config: SensorOptions,

    /// The acquired sensor.
    sensor: Option<Sensor>,
}

impl Data {
    pub fn new(sensor_config: SensorOptions) -> Self {
        Self {
            sensor_config,
            sensor: None,
        }
    }

    fn read(&mut self) -> Result<Datum, Co2monError> {
        // If we don't have an acquired sensor, get it
        if self.sensor.is_none() {
            self.sensor = Some(self.sensor_config.open()?);
        };
        let sensor = self
            .sensor
            .as_ref()
            .expect("Sensor must be Some as we just set it.");

        // Then take a reading
        sensor.read().map(|reading| reading.into())
    }
}

impl Default for Data {
    fn default() -> Self {
        Self {
            sensor_config: SensorOptions::new(),
            sensor: None,
        }
    }
}

fn run() -> Result<(), Error> {
    let settings = Settings::load()?;

    let client = reqwest::blocking::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Token {}", &settings.influxdb_token)
            .parse()
            .map_err(|_| Error::User {
                description: "Invalid InfluxDB token for Authorization header.".to_owned(),
            })?,
    );
    let query: Vec<(&str, &str)> = vec![
        ("org", &settings.influxdb_org),
        ("bucket", &settings.influxdb_bucket),
        ("precision", "s"),
    ];

    let url = Url::parse(&settings.influxdb_url)
        .map_err(|_| Error::User {
            description: format!("Invalid InfluxDB base url {}", &settings.influxdb_url),
        })?
        .join("api/v2/write")
        .unwrap();

    let mut data = Data::default();
    loop {
        match data.read() {
            Ok(datum) => {
                let response = client
                    .post(url.clone())
                    .headers(headers.clone())
                    .query(&query)
                    .body(format!(
                        "{} c={},t={} {}",
                        datum.timestamp, datum.co2, datum.temperature, datum.timestamp
                    ))
                    .send()?;
                debug!("{}", response.text()?);
                println!(
                    "{}",
                    ::serde_json::to_string(&datum).expect("Failed to serialize JSON")
                );
            }
            Err(e) => warn!("{}", e),
        };
        thread::sleep(Duration::from_secs(settings.interval));
    }
}

fn main() {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();
    debug!("Initialised logger.");

    run().unwrap_or_else(|e| error!("{}", e));
}
