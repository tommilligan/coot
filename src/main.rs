use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use co2mon::{Reading, Result, Sensor};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Datum {
    temperature: f32,
    co2: u16,
    timestamp: u128,
}

impl From<Reading> for Datum {
    fn from(reading: Reading) -> Self {
        Datum {
            temperature: reading.temperature(),
            co2: reading.co2(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis(),
        }
    }
}

fn main() -> Result<()> {
    let sensor = Sensor::open_default()?;
    loop {
        match sensor.read() {
            Ok(reading) => {
                let datum: Datum = reading.into();
                println!(
                    "{}",
                    ::serde_json::to_string(&datum).expect("Failed to serialize JSON")
                );
            }
            Err(e) => eprintln!("{}", e),
        }
        thread::sleep(Duration::from_secs(5));
    }
}
