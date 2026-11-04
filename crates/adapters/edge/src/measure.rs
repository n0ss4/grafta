//! The adapters of the measure and of a file's own JSON: `serde_json`.

use crate::app::data::Data;
use crate::port::{JsonText, Measure};

/// A value measured as the bytes of its JSON, as `serde_json` writes it:
/// written into a count, so nothing is kept.
pub struct Json;

impl Measure for Json {
    fn bytes(&self, data: &Data) -> usize {
        let mut count = Count(0);
        serde_json::to_writer(&mut count, data).expect("data is JSON");
        count.0
    }
}

/// A file's own JSON, read and written by `serde_json`.
pub struct Text;

impl JsonText for Text {
    fn parse(&self, text: &str) -> Result<Data, String> {
        serde_json::from_str::<serde_json::Value>(text).map(|value| super::data::from_json(&value)).map_err(|error| error.to_string())
    }
    fn write(&self, data: &Data) -> String {
        serde_json::to_string(data).expect("data is JSON")
    }
}

/// Bytes written, counted and dropped.
struct Count(usize);

impl std::io::Write for Count {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

