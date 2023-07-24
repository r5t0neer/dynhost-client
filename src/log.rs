use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local, NaiveDateTime, Offset, TimeZone, Utc};

pub struct Logger
{
    file: File
}

impl Logger
{
    pub fn new(path: &str) -> Result<Logger, std::io::Error>
    {
        Ok(Logger {
            file: OpenOptions::new().append(true).create(true).open(path)?
        })
    }

    fn current_time(&self) -> String
    {
        let off: i32 = Local.timestamp_nanos(0).offset().fix().local_minus_utc();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let timestamp = timestamp + (off as u64);

        let ndt = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap();
        let dt: DateTime<Utc> = DateTime::from_utc(ndt, Utc);

        let date = dt.format("%Y-%m-%d %H:%M:%S");

        date.to_string()
    }

    pub fn info(&mut self, msg: &str)
    {
        self.file.write_all(format!("[{}][INFO] {}\n", self.current_time(), msg).as_bytes());
    }

    pub fn error(&mut self, msg: &str)
    {
        self.file.write_all(format!("[{}][ERROR] {}\n", self.current_time(), msg).as_bytes());
    }
}