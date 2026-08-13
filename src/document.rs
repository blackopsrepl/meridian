use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::astro::Chart;

pub const CHART_EXTENSION: &str = "meridian";
const DOCUMENT_KIND: &str = "org.meridian.chart";
const DOCUMENT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartDocument {
    kind: String,
    version: u16,
    pub chart: Chart,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("could not open chart file: {0}")]
    Open(#[source] std::io::Error),
    #[error("could not read chart file: {0}")]
    Read(#[source] serde_json::Error),
    #[error("this is not a Meridian chart file")]
    WrongKind,
    #[error("chart file version {0} is not supported")]
    UnsupportedVersion(u16),
    #[error("could not create chart file: {0}")]
    Create(#[source] std::io::Error),
    #[error("could not encode chart file: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("could not finish writing chart file: {0}")]
    Flush(#[source] std::io::Error),
}

impl ChartDocument {
    #[must_use]
    pub fn new(chart: Chart) -> Self {
        Self {
            kind: DOCUMENT_KIND.to_owned(),
            version: DOCUMENT_VERSION,
            chart,
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let file = File::open(path).map_err(DocumentError::Open)?;
        let document: Self =
            serde_json::from_reader(BufReader::new(file)).map_err(DocumentError::Read)?;
        document.validate()?;
        Ok(document)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DocumentError> {
        self.validate()?;
        let file = File::create(path).map_err(DocumentError::Create)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, self).map_err(DocumentError::Encode)?;
        writer.write_all(b"\n").map_err(DocumentError::Flush)?;
        writer.flush().map_err(DocumentError::Flush)
    }

    fn validate(&self) -> Result<(), DocumentError> {
        if self.kind != DOCUMENT_KIND {
            return Err(DocumentError::WrongKind);
        }
        if self.version != DOCUMENT_VERSION {
            return Err(DocumentError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ChartDocument;
    use crate::astro::{
        Calendar, ChartCalculator, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
        SwissEphemerisProvider, TimeZoneSpec, TraditionalHouseSystem,
    };

    #[test]
    fn chart_document_round_trips_as_a_file() -> Result<(), Box<dyn std::error::Error>> {
        let chart = ChartCalculator::new(SwissEphemerisProvider::new("data/ephe")?).calculate(
            ChartRequest {
                title: "File chart".to_owned(),
                purpose: ChartPurpose::Natal,
                local_time: CivilDateTime {
                    year: 2000,
                    month: 1,
                    day: 1,
                    hour: 12,
                    minute: 0,
                    second: 0.0,
                    calendar: Calendar::Gregorian,
                },
                time_zone: TimeZoneSpec::FixedOffset {
                    minutes_east: 0,
                    label: Some("UTC".to_owned()),
                },
                location_name: "Greenwich".to_owned(),
                coordinates: Coordinates {
                    latitude: 51.4779,
                    longitude: 0.0,
                    elevation_m: 46.0,
                },
                house_system: TraditionalHouseSystem::WholeSign,
            },
        )?;
        let document = ChartDocument::new(chart);
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("file-chart.meridian");
        document.save(&path)?;
        assert_eq!(ChartDocument::open(path)?, document);
        Ok(())
    }
}
