// SPDX-License-Identifier: AGPL-3.0-or-later

//! Local `GeoNames` city search and canonical chart-location resolution.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct City {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub country_code: String,
    pub country_name: String,
    pub admin1_name: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation_m: f64,
    pub timezone: String,
    pub population: u64,
}

#[derive(Debug, Clone)]
struct CityRecord {
    city: City,
    canonical_key: String,
    ascii_key: String,
    aliases_key: String,
    search_key: String,
}

#[derive(Debug)]
struct CityIndexData {
    records: Vec<CityRecord>,
    by_id: HashMap<u64, usize>,
}

#[derive(Debug, Clone)]
pub struct CityIndex {
    data: Arc<CityIndexData>,
}

#[derive(Debug, Error)]
pub enum LocationError {
    #[error("required GeoNames file is missing: {0}")]
    MissingFile(PathBuf),
    #[error("could not read GeoNames file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid GeoNames record in {path} at line {line}: {reason}")]
    InvalidRecord {
        path: PathBuf,
        line: usize,
        reason: String,
    },
    #[error("the GeoNames city catalog is empty")]
    Empty,
}

impl CityIndex {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self, LocationError> {
        let directory = directory.as_ref();
        let countries = load_name_map(&directory.join("countryInfo.txt"), NameMapKind::Country)?;
        let admin1 = load_name_map(&directory.join("admin1CodesASCII.txt"), NameMapKind::Admin1)?;
        let cities_path = directory.join("cities500.txt");
        let file = open_required(&cities_path)?;
        let mut records = Vec::new();
        let mut by_id = HashMap::new();

        for (offset, line) in BufReader::new(file).lines().enumerate() {
            let line_number = offset + 1;
            let line = line.map_err(|source| LocationError::Io {
                path: cities_path.clone(),
                source,
            })?;
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() < 19 {
                return Err(invalid_record(
                    &cities_path,
                    line_number,
                    format!("expected 19 tab-separated fields, found {}", fields.len()),
                ));
            }
            let record = parse_city(&fields, &countries, &admin1)
                .map_err(|reason| invalid_record(&cities_path, line_number, reason))?;
            let index = records.len();
            by_id.insert(record.city.id, index);
            records.push(record);
        }
        if records.is_empty() {
            return Err(LocationError::Empty);
        }
        Ok(Self {
            data: Arc::new(CityIndexData { records, by_id }),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.records.is_empty()
    }

    #[must_use]
    pub fn get(&self, id: u64) -> Option<&City> {
        self.data
            .by_id
            .get(&id)
            .and_then(|index| self.data.records.get(*index))
            .map(|record| &record.city)
    }

    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<City> {
        let query = normalize(query);
        if query.chars().count() < 2 || limit == 0 {
            return Vec::new();
        }
        let tokens = query.split_whitespace().collect::<Vec<_>>();
        let mut matches = self
            .data
            .records
            .iter()
            .filter(|record| tokens.iter().all(|token| record.search_key.contains(token)))
            .map(|record| (match_rank(record, &query), record))
            .collect::<Vec<_>>();
        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| right.city.population.cmp(&left.city.population))
                .then_with(|| left.city.display_name.cmp(&right.city.display_name))
        });
        matches
            .into_iter()
            .take(limit.min(20))
            .map(|(_, record)| record.city.clone())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        let records = vec![
            city_record(
                City {
                    id: 3_182_164,
                    name: "Bergamo".to_owned(),
                    display_name: "Bergamo, Lombardy, Italy".to_owned(),
                    country_code: "IT".to_owned(),
                    country_name: "Italy".to_owned(),
                    admin1_name: Some("Lombardy".to_owned()),
                    latitude: 45.696,
                    longitude: 9.667,
                    elevation_m: 249.0,
                    timezone: "Europe/Rome".to_owned(),
                    population: 121_200,
                },
                "Bergamo",
                "Bergamo",
                "Berghem",
            ),
            city_record(
                City {
                    id: 3_171_174,
                    name: "Milano".to_owned(),
                    display_name: "Milano, Lombardy, Italy".to_owned(),
                    country_code: "IT".to_owned(),
                    country_name: "Italy".to_owned(),
                    admin1_name: Some("Lombardy".to_owned()),
                    latitude: 45.464,
                    longitude: 9.19,
                    elevation_m: 122.0,
                    timezone: "Europe/Rome".to_owned(),
                    population: 1_371_498,
                },
                "Milano",
                "Milano",
                "Milan,Mailand",
            ),
        ];
        let by_id = records
            .iter()
            .enumerate()
            .map(|(index, record)| (record.city.id, index))
            .collect();
        Self {
            data: Arc::new(CityIndexData { records, by_id }),
        }
    }
}

fn parse_city(
    fields: &[&str],
    countries: &HashMap<String, String>,
    admin1: &HashMap<String, String>,
) -> Result<CityRecord, String> {
    if fields[6] != "P" {
        return Err(format!(
            "expected populated-place feature class, got {}",
            fields[6]
        ));
    }
    let id = parse_field(fields[0], "geoname id")?;
    let name = required_field(fields[1], "name")?;
    let ascii_name = required_field(fields[2], "ASCII name")?;
    let latitude = parse_field(fields[4], "latitude")?;
    let longitude = parse_field(fields[5], "longitude")?;
    let country_code = required_field(fields[8], "country code")?;
    let population = if fields[14].is_empty() {
        0
    } else {
        parse_field(fields[14], "population")?
    };
    let elevation_m = fields[15]
        .parse::<f64>()
        .or_else(|_| fields[16].parse::<f64>())
        .unwrap_or(0.0);
    let timezone = required_field(fields[17], "time zone")?;
    let country_name = countries
        .get(country_code)
        .cloned()
        .unwrap_or_else(|| country_code.to_owned());
    let admin1_key = format!("{country_code}.{}", fields[10]);
    let admin1_name = admin1.get(&admin1_key).cloned();
    let display_name = display_name(name, admin1_name.as_deref(), &country_name);
    Ok(city_record(
        City {
            id,
            name: name.to_owned(),
            display_name,
            country_code: country_code.to_owned(),
            country_name,
            admin1_name,
            latitude,
            longitude,
            elevation_m,
            timezone: timezone.to_owned(),
            population,
        },
        name,
        ascii_name,
        fields[3],
    ))
}

fn city_record(city: City, canonical: &str, ascii: &str, aliases: &str) -> CityRecord {
    let canonical_key = normalize(canonical);
    let ascii_key = normalize(ascii);
    let aliases_key = normalize(aliases);
    let search_key = normalize(&format!(
        "{} {} {} {} {} {}",
        canonical,
        ascii,
        aliases.replace(',', " "),
        city.admin1_name.as_deref().unwrap_or_default(),
        city.country_name,
        city.country_code
    ));
    CityRecord {
        city,
        canonical_key,
        ascii_key,
        aliases_key,
        search_key,
    }
}

fn match_rank(record: &CityRecord, query: &str) -> u8 {
    if record.canonical_key == query || record.ascii_key == query {
        0
    } else if record
        .aliases_key
        .split(',')
        .any(|alias| alias.trim() == query)
    {
        1
    } else if record.canonical_key.starts_with(query) || record.ascii_key.starts_with(query) {
        2
    } else if record
        .search_key
        .split_whitespace()
        .any(|word| word.starts_with(query))
    {
        3
    } else {
        4
    }
}

fn display_name(name: &str, admin1: Option<&str>, country: &str) -> String {
    match admin1.filter(|admin| *admin != name && *admin != country) {
        Some(admin) => format!("{name}, {admin}, {country}"),
        None => format!("{name}, {country}"),
    }
}

#[derive(Debug, Clone, Copy)]
enum NameMapKind {
    Country,
    Admin1,
}

fn load_name_map(path: &Path, kind: NameMapKind) -> Result<HashMap<String, String>, LocationError> {
    let file = open_required(path)?;
    let mut values = HashMap::new();
    for (offset, line) in BufReader::new(file).lines().enumerate() {
        let line_number = offset + 1;
        let line = line.map_err(|source| LocationError::Io {
            path: path.to_owned(),
            source,
        })?;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        let (key_index, name_index) = match kind {
            NameMapKind::Country => (0, 4),
            NameMapKind::Admin1 => (0, 1),
        };
        if fields.len() <= name_index {
            return Err(invalid_record(
                path,
                line_number,
                format!("missing field {name_index}"),
            ));
        }
        values.insert(fields[key_index].to_owned(), fields[name_index].to_owned());
    }
    Ok(values)
}

fn open_required(path: &Path) -> Result<File, LocationError> {
    File::open(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            LocationError::MissingFile(path.to_owned())
        } else {
            LocationError::Io {
                path: path.to_owned(),
                source,
            }
        }
    })
}

fn invalid_record(path: &Path, line: usize, reason: String) -> LocationError {
    LocationError::InvalidRecord {
        path: path.to_owned(),
        line,
        reason,
    }
}

fn required_field<'a>(value: &'a str, name: &str) -> Result<&'a str, String> {
    if value.trim().is_empty() {
        Err(format!("{name} is empty"))
    } else {
        Ok(value)
    }
}

fn parse_field<T>(value: &str, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| format!("invalid {name}"))
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::CityIndex;

    #[test]
    fn alternate_city_name_finds_canonical_record() {
        let index = CityIndex::test_fixture();
        let matches = index.search("Milan Italy", 10);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Milano");
        assert_eq!(matches[0].timezone, "Europe/Rome");
    }

    #[test]
    fn exact_name_outranks_larger_partial_match() {
        let index = CityIndex::test_fixture();
        let matches = index.search("Bergamo", 10);
        assert_eq!(matches[0].id, 3_182_164);
    }
}
