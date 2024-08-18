use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct LogEntry<T> {
    timestamp: u64,
    level: String,
    message: String,
    data: T,
}

pub struct Logger;

impl Logger {
    pub fn log<T: Serialize>(level: &str, message: &str, data: T) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let log_entry = LogEntry {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
            data,
        };

        let json = serde_json::to_string(&log_entry).expect("Failed to serialize log entry");

        println!("{}", json);
    }
}
