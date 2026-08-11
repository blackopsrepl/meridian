use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::astro::{Chart, ChartRequest};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartRecord {
    pub id: Uuid,
    pub title: String,
    pub request: ChartRequest,
    pub chart: Chart,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSummary {
    pub id: Uuid,
    pub title: String,
    pub purpose: String,
    pub location_name: String,
    pub local_date: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database lock was poisoned")]
    Poisoned,
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored timestamp is invalid: {0}")]
    Timestamp(#[from] chrono::ParseError),
    #[error("invalid chart identifier: {0}")]
    InvalidId(String),
    #[error("database path has no usable parent directory")]
    InvalidPath,
    #[error("could not create database directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();
        if path != Path::new(":memory:") {
            let parent = path.parent().ok_or(StoreError::InvalidPath)?;
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(StoreError::CreateDirectory)?;
            }
        }
        let connection = Connection::open(path)?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn insert_chart(&self, chart: &Chart) -> Result<ChartRecord, StoreError> {
        let now = Utc::now();
        let record = ChartRecord {
            id: Uuid::now_v7(),
            title: chart.request.title.clone(),
            request: chart.request.clone(),
            chart: chart.clone(),
            created_at: now,
            updated_at: now,
        };
        let request_json = serde_json::to_string(&record.request)?;
        let chart_json = serde_json::to_string(&record.chart)?;
        self.lock()?.execute(
            "INSERT INTO charts (
                id, title, purpose, location_name, local_date,
                request_json, chart_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id.to_string(),
                &record.title,
                format!("{:?}", record.request.purpose).to_lowercase(),
                &record.request.location_name,
                format_local_date(&record.request),
                request_json,
                chart_json,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(record)
    }

    pub fn get_chart(&self, id: Uuid) -> Result<Option<ChartRecord>, StoreError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT title, request_json, chart_json, created_at, updated_at
             FROM charts WHERE id = ?1",
        )?;
        let raw = statement
            .query_row([id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .optional()?;
        raw.map(
            |(title, request_json, chart_json, created_at, updated_at)| {
                Ok(ChartRecord {
                    id,
                    title,
                    request: serde_json::from_str(&request_json)?,
                    chart: serde_json::from_str(&chart_json)?,
                    created_at: parse_timestamp(&created_at)?,
                    updated_at: parse_timestamp(&updated_at)?,
                })
            },
        )
        .transpose()
    }

    pub fn list_charts(&self, limit: usize) -> Result<Vec<ChartSummary>, StoreError> {
        let limit = i64::try_from(limit.min(500)).unwrap_or(500);
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, title, purpose, location_name, local_date, created_at
             FROM charts ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            let (id, title, purpose, location_name, local_date, created_at) = row?;
            summaries.push(ChartSummary {
                id: Uuid::parse_str(&id).map_err(|_| StoreError::InvalidId(id))?,
                title,
                purpose,
                location_name,
                local_date,
                created_at: parse_timestamp(&created_at)?,
            });
        }
        Ok(summaries)
    }

    pub fn delete_chart(&self, id: Uuid) -> Result<bool, StoreError> {
        let changed = self
            .lock()?
            .execute("DELETE FROM charts WHERE id = ?1", [id.to_string()])?;
        Ok(changed > 0)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let connection = self.lock()?;
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             CREATE TABLE IF NOT EXISTS charts (
               id TEXT PRIMARY KEY NOT NULL,
               title TEXT NOT NULL,
               purpose TEXT NOT NULL,
               location_name TEXT NOT NULL,
               local_date TEXT NOT NULL,
               request_json TEXT NOT NULL,
               chart_json TEXT NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS charts_created_at_idx
               ON charts(created_at DESC);
             PRAGMA user_version = 1;",
        )?;
        Ok(())
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::Poisoned)
    }
}

fn format_local_date(request: &ChartRequest) -> String {
    let local = &request.local_time;
    format!(
        "{:+05}-{:02}-{:02} {:02}:{:02}",
        local.year, local.month, local.day, local.hour, local.minute
    )
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(StoreError::Timestamp)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{Store, StoreError};
    use crate::astro::{
        Calendar, ChartCalculator, ChartError, ChartPurpose, ChartRequest, CivilDateTime,
        Coordinates, SwissEphemerisProvider, TimeZoneSpec, TraditionalHouseSystem,
    };

    fn chart() -> Result<crate::astro::Chart, ChartError> {
        ChartCalculator::new(SwissEphemerisProvider::new("data/ephe")?).calculate(ChartRequest {
            title: "Archive test".to_owned(),
            purpose: ChartPurpose::Natal,
            local_time: CivilDateTime {
                year: 1984,
                month: 6,
                day: 15,
                hour: 10,
                minute: 30,
                second: 0.0,
                calendar: Calendar::Gregorian,
            },
            time_zone: TimeZoneSpec::Iana {
                name: "Europe/Rome".to_owned(),
                fold: None,
            },
            location_name: "Bergamo".to_owned(),
            coordinates: Coordinates {
                latitude: 45.6983,
                longitude: 9.6773,
                elevation_m: 249.0,
            },
            house_system: TraditionalHouseSystem::WholeSign,
        })
    }

    #[test]
    fn chart_round_trips_through_sqlite() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let store = Store::open(directory.path().join("meridian.sqlite3"))?;
        let original = chart()?;
        let inserted = store.insert_chart(&original)?;
        let loaded = store
            .get_chart(inserted.id)?
            .ok_or(StoreError::InvalidId(inserted.id.to_string()))?;
        assert_eq!(loaded.chart, original);
        assert_eq!(store.list_charts(20)?.len(), 1);
        assert!(store.delete_chart(inserted.id)?);
        assert!(store.get_chart(inserted.id)?.is_none());
        Ok(())
    }
}
