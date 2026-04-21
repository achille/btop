use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// Asciicast v2 header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastHeader {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
}

impl CastHeader {
    pub fn new(width: u32, height: u32) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .ok();
        Self {
            version: 2,
            width,
            height,
            timestamp,
            title: None,
            env: None,
        }
    }
}

/// A single asciicast v2 event: [time, event_type, data].
#[derive(Debug, Clone)]
pub struct CastEvent {
    pub time: f64,
    pub event_type: String,
    pub data: String,
}

impl Serialize for CastEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.time)?;
        seq.serialize_element(&self.event_type)?;
        seq.serialize_element(&self.data)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for CastEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let v: (f64, String, String) = Deserialize::deserialize(deserializer)?;
        Ok(CastEvent {
            time: v.0,
            event_type: v.1,
            data: v.2,
        })
    }
}

/// Writes asciicast v2 format (NDJSON).
pub struct CastWriter<W: Write> {
    writer: W,
}

impl<W: Write> CastWriter<W> {
    pub fn new(mut writer: W, header: &CastHeader) -> Result<Self> {
        let line = serde_json::to_string(header)?;
        writeln!(writer, "{}", line)?;
        Ok(Self { writer })
    }

    pub fn write_event(&mut self, event: &CastEvent) -> Result<()> {
        let line = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", line)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Reads asciicast v2 format (NDJSON).
pub struct CastReader<R: BufRead> {
    reader: R,
    pub header: CastHeader,
}

impl<R: BufRead> CastReader<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .context("failed to read cast header")?;
        let header: CastHeader =
            serde_json::from_str(first_line.trim()).context("failed to parse cast header")?;
        if header.version != 2 {
            bail!("unsupported asciicast version: {}", header.version);
        }
        Ok(Self { reader, header })
    }
}

impl<R: BufRead> Iterator for CastReader<R> {
    type Item = Result<CastEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return self.next();
                }
                Some(
                    serde_json::from_str::<CastEvent>(trimmed)
                        .context("failed to parse cast event"),
                )
            }
            Err(e) => Some(Err(e.into())),
        }
    }
}

/// Read all events from a cast file.
pub fn read_cast_file(path: &std::path::Path) -> Result<(CastHeader, Vec<CastEvent>)> {
    let file = std::fs::File::open(path).context("failed to open cast file")?;
    let reader = CastReader::new(std::io::BufReader::new(file))?;
    let header = reader.header.clone();
    let events: Result<Vec<_>> = reader.collect();
    Ok((header, events?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip() {
        let header = CastHeader {
            version: 2,
            width: 80,
            height: 24,
            timestamp: Some(1234567890),
            title: Some("test".into()),
            env: None,
        };
        let events = vec![
            CastEvent {
                time: 0.0,
                event_type: "o".into(),
                data: "hello".into(),
            },
            CastEvent {
                time: 1.5,
                event_type: "o".into(),
                data: "world\r\n".into(),
            },
            CastEvent {
                time: 2.0,
                event_type: "o".into(),
                data: "\x1b[31mred\x1b[0m".into(),
            },
        ];

        let mut buf = Vec::new();
        {
            let mut writer = CastWriter::new(&mut buf, &header).unwrap();
            for ev in &events {
                writer.write_event(ev).unwrap();
            }
        }

        let reader = CastReader::new(Cursor::new(&buf)).unwrap();
        assert_eq!(reader.header.width, 80);
        assert_eq!(reader.header.height, 24);
        assert_eq!(reader.header.timestamp, Some(1234567890));

        let read_events: Vec<CastEvent> = reader.map(|e| e.unwrap()).collect();
        assert_eq!(read_events.len(), 3);
        assert_eq!(read_events[0].data, "hello");
        assert!((read_events[1].time - 1.5).abs() < 1e-9);
        assert_eq!(read_events[2].data, "\x1b[31mred\x1b[0m");
    }

    #[test]
    fn rejects_v1() {
        let data = b"{\"version\":1,\"width\":80,\"height\":24}\n";
        let result = CastReader::new(Cursor::new(data));
        assert!(result.is_err());
    }
}
