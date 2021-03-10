use std::convert::TryFrom;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use co2mon::{OpenOptions as SensorOptions, Reading, Sensor};
use env_logger::Env;
use log::{debug, error, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Url;
use serde_derive::{Deserialize, Serialize};

mod error;

use error::{Error, Result};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    interval: u64,
    influxdb_url: String,
    influxdb_token: String,
    influxdb_bucket: String,
    influxdb_org: String,
}

impl Settings {
    pub fn load() -> Result<Self> {
        debug!("Loading settings.");

        let mut settings = config::Config::default();
        settings.set_default("interval", 5)?;
        settings.merge(config::File::with_name("coot.yml"))?;
        // settings.merge(config::Environment::with_prefix("COOT"))?;

        // Print out our settings (as a HashMap)
        let settings = settings.try_into::<Self>()?;
        Ok(settings)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Datum {
    temperature: f32,
    co2: u16,
    timestamp: u64,
}

impl TryFrom<Reading> for Datum {
    type Error = Error;

    fn try_from(reading: Reading) -> Result<Self> {
        let co2 = reading.co2();
        if co2 < 300 {
            return Err(Error::Data {
                description: format!("CO2 out of sane range, was {}", co2),
            });
        };

        Ok(Datum {
            temperature: reading.temperature(),
            co2,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        })
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

    fn read(&mut self) -> Result<Datum> {
        // If we don't have an acquired sensor, get it
        if self.sensor.is_none() {
            self.sensor = Some(self.sensor_config.open()?);
        };

        let sensor = self
            .sensor
            .as_ref()
            .expect("Sensor must be Some as we just set it.");

        // Then take a reading. If the reading fails, release the sensor so
        // we try and reaquire it next time.
        let reading = sensor.read().map_err(|e| {
            self.sensor = None;
            e
        })?;
        Datum::try_from(reading)
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

struct InfluxDBClient<'a> {
    client: Client,
    headers: HeaderMap,
    query: Vec<(&'static str, &'a str)>,
    url_write: Url,
}

impl<'a> InfluxDBClient<'a> {
    fn new(client: Client, url: Url, token: &str, bucket: &'a str, org: &'a str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Token {}", &token)
                .parse()
                .map_err(|_| Error::User {
                    description: "Invalid InfluxDB token for Authorization header.".to_owned(),
                })?,
        );
        let query: Vec<(&str, &str)> = vec![("org", org), ("bucket", bucket), ("precision", "s")];

        let url_write = url.join("api/v2/write").expect("invalid write path");

        Ok(Self {
            client,
            headers,
            query,
            url_write,
        })
    }

    fn write_datum(&self, datum: &Datum) -> Result<()> {
        let request_builder = self
            .client
            .post(self.url_write.clone())
            .headers(self.headers.clone())
            .query(&self.query);

        let line_serialized = format!(
            "co2mon c={},t={} {}",
            datum.co2, datum.temperature, datum.timestamp
        );
        // Then, send the data to InfluxDB
        let response = request_builder.body(line_serialized).send()?;
        // If the api errors, log it and continue
        if let Err(e) = response.error_for_status() {
            error!("{}", e);
        }
        Ok(())
    }
}

fn run() -> Result<()> {
    let settings = Settings::load()?;

    let client = Client::new();
    let url = Url::parse(&settings.influxdb_url).map_err(|_| Error::User {
        description: format!("Invalid InfluxDB base url {}", &settings.influxdb_url),
    })?;
    let influxdb_client = InfluxDBClient::new(
        client,
        url,
        &settings.influxdb_token,
        &settings.influxdb_bucket,
        &settings.influxdb_org,
    )?;

    debug!("Taking readings...");
    let mut data = Data::default();
    loop {
        match data.read() {
            Ok(datum) => {
                // First log out in json to stdout
                // TODO convert this to a write trait
                println!(
                    "{}",
                    ::serde_json::to_string(&datum).expect("Failed to serialize JSON")
                );
                influxdb_client.write_datum(&datum)?;
            }
            // There was an error taking the reading.
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

#[cfg(test)]
mod test {
    use super::*;
    use mockito::mock;

    fn mock_influxdb_client() -> Result<InfluxDBClient<'static>> {
        let url = &mockito::server_url();
        let client = Client::new();
        InfluxDBClient::new(
            client,
            url.parse().expect("invalid mockito url"),
            "supersecrettoken",
            "test-bucket",
            "test-org",
        )
    }

    #[test]
    fn test_write_success() -> Result<()> {
        let client = mock_influxdb_client()?;

        let mock_hello = mock(
            "POST",
            "/api/v2/write?org=test-org&bucket=test-bucket&precision=s",
        )
        .match_header("authorization", "Token supersecrettoken")
        .match_body("co2mon c=400,t=21.3 123456789")
        .with_status(200)
        .create();

        client.write_datum(&Datum {
            temperature: 21.3,
            co2: 400,
            timestamp: 123456789,
        })?;

        mock_hello.assert();
        Ok(())
    }

    /// On failure to write to the InfluxDB api, we should continue without error
    #[test]
    fn test_write_fail_continues() -> Result<()> {
        let client = mock_influxdb_client()?;

        let mock_hello = mock(
            "POST",
            "/api/v2/write?org=test-org&bucket=test-bucket&precision=s",
        )
        .with_status(500)
        .create();
        client.write_datum(&Datum::default())?;

        mock_hello.assert();
        Ok(())
    }
}
