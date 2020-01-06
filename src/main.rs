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
    loop {
        // Acquire sensor. If we cant, wait for a bit then continue.
        let sensor = match Sensor::open_default() {
            Ok(sensor) => sensor,
            Err(e) => {
                eprintln!("Error acquiring sensor: {}", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        loop {
            match sensor.read() {
                Ok(reading) => {
                    let datum: Datum = reading.into();
                    println!(
                        "{}",
                        ::serde_json::to_string(&datum).expect("Failed to serialize JSON")
                    );
                }
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    }
}
