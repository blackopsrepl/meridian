use serde::Deserialize;

use crate::astro::{
    Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, OrbPolicy, TimeZoneSpec,
    TraditionalHouseSystem,
};
use crate::locations::CityIndex;

use super::error::WebError;

#[derive(Debug, Deserialize)]
pub struct NewChartForm {
    pub title: String,
    pub purpose: String,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub time: String,
    pub calendar: String,
    #[serde(default)]
    pub city_id: Option<u64>,
    #[serde(default)]
    pub manual_coordinates: Option<String>,
    #[serde(default)]
    pub manual_timezone: Option<String>,
    #[serde(default)]
    pub zone_mode: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub fixed_offset_minutes: i32,
    #[serde(default)]
    pub fold: Option<String>,
    #[serde(default)]
    pub location_name: String,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub elevation_m: Option<f64>,
    pub house_system: String,
    #[serde(default = "default_conjunction")]
    pub orb_conjunction: f64,
    #[serde(default = "default_sextile")]
    pub orb_sextile: f64,
    #[serde(default = "default_square")]
    pub orb_square: f64,
    #[serde(default = "default_trine")]
    pub orb_trine: f64,
    #[serde(default = "default_opposition")]
    pub orb_opposition: f64,
    #[serde(default = "default_luminary_bonus")]
    pub orb_luminary_bonus: f64,
    #[serde(default = "default_angle_orb")]
    pub orb_angle: f64,
    #[serde(default = "default_lot_orb")]
    pub orb_lot: f64,
}

impl NewChartForm {
    pub fn into_calculation(
        self,
        cities: &CityIndex,
    ) -> Result<(ChartRequest, OrbPolicy), WebError> {
        let orb_policy = OrbPolicy {
            conjunction: validate_orb(self.orb_conjunction, "conjunction")?,
            sextile: validate_orb(self.orb_sextile, "sextile")?,
            square: validate_orb(self.orb_square, "square")?,
            trine: validate_orb(self.orb_trine, "trine")?,
            opposition: validate_orb(self.orb_opposition, "opposition")?,
            luminary_bonus: validate_orb(self.orb_luminary_bonus, "luminary bonus")?,
            angle_orb: validate_orb(self.orb_angle, "angle")?,
            lot_orb: validate_orb(self.orb_lot, "lot")?,
        };
        let title = required(self.title, "title")?;
        let (hour, minute, second) = parse_time(&self.time)?;
        let calendar = match self.calendar.as_str() {
            "gregorian" => Calendar::Gregorian,
            "julian" => Calendar::Julian,
            value => return Err(WebError::BadRequest(format!("unknown calendar: {value}"))),
        };
        let purpose = parse_purpose(&self.purpose)?;
        let house_system = parse_house_system(&self.house_system)?;
        let manual_coordinates = checkbox(
            self.manual_coordinates.as_deref(),
            "manual coordinate override",
        )?;
        let manual_timezone =
            checkbox(self.manual_timezone.as_deref(), "manual time-zone override")?;
        let selected_city = self
            .city_id
            .map(|id| {
                cities.get(id).ok_or_else(|| {
                    WebError::BadRequest(
                        "the selected city is not present in the installed atlas".to_owned(),
                    )
                })
            })
            .transpose()?;
        let (location_name, coordinates) = if manual_coordinates {
            (
                required(self.location_name, "location name")?,
                Coordinates {
                    latitude: required_number(self.latitude, "latitude")?,
                    longitude: required_number(self.longitude, "longitude")?,
                    elevation_m: self.elevation_m.unwrap_or(0.0),
                },
            )
        } else {
            let city = selected_city.ok_or_else(city_selection_required)?;
            (
                city.display_name.clone(),
                Coordinates {
                    latitude: city.latitude,
                    longitude: city.longitude,
                    elevation_m: city.elevation_m,
                },
            )
        };
        let fold = parse_fold(self.fold.as_deref())?;
        let time_zone = if manual_timezone {
            match self.zone_mode.as_str() {
                "iana" => TimeZoneSpec::Iana {
                    name: required(self.timezone, "IANA time zone")?,
                    fold,
                },
                "fixed" => TimeZoneSpec::FixedOffset {
                    minutes_east: self.fixed_offset_minutes,
                    label: None,
                },
                value => {
                    return Err(WebError::BadRequest(format!(
                        "unknown time-zone mode: {value}"
                    )));
                }
            }
        } else {
            let city = selected_city.ok_or_else(|| {
                WebError::BadRequest(
                    "select a city or enable the manual time-zone override".to_owned(),
                )
            })?;
            TimeZoneSpec::Iana {
                name: city.timezone.clone(),
                fold,
            }
        };
        Ok((
            ChartRequest {
                title,
                purpose,
                local_time: CivilDateTime {
                    year: self.year,
                    month: self.month,
                    day: self.day,
                    hour,
                    minute,
                    second,
                    calendar,
                },
                time_zone,
                location_name,
                coordinates,
                house_system,
            },
            orb_policy,
        ))
    }
}

fn city_selection_required() -> WebError {
    WebError::BadRequest(
        "choose a city from the atlas or enable the manual coordinate override".to_owned(),
    )
}

fn required_number(value: Option<f64>, name: &str) -> Result<f64, WebError> {
    value.ok_or_else(|| WebError::BadRequest(format!("{name} is required")))
}

fn checkbox(value: Option<&str>, name: &str) -> Result<bool, WebError> {
    match value {
        None => Ok(false),
        Some("1" | "on" | "true") => Ok(true),
        Some(_) => Err(WebError::BadRequest(format!("invalid {name}"))),
    }
}

fn validate_orb(value: f64, name: &str) -> Result<f64, WebError> {
    if value.is_finite() && (0.0..=30.0).contains(&value) {
        Ok(value)
    } else {
        Err(WebError::BadRequest(format!(
            "{name} orb must be between 0 and 30 degrees"
        )))
    }
}

const fn default_conjunction() -> f64 {
    8.0
}
const fn default_sextile() -> f64 {
    5.0
}
const fn default_square() -> f64 {
    7.0
}
const fn default_trine() -> f64 {
    7.0
}
const fn default_opposition() -> f64 {
    8.0
}
const fn default_luminary_bonus() -> f64 {
    2.0
}
const fn default_angle_orb() -> f64 {
    5.0
}
const fn default_lot_orb() -> f64 {
    3.0
}

fn parse_fold(value: Option<&str>) -> Result<Option<u8>, WebError> {
    match value {
        None | Some("") => Ok(None),
        Some("0") => Ok(Some(0)),
        Some("1") => Ok(Some(1)),
        Some(_) => Err(WebError::BadRequest("fold must be 0 or 1".to_owned())),
    }
}

fn required(value: String, field: &str) -> Result<String, WebError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(WebError::BadRequest(format!("{field} is required")))
    } else if trimmed.chars().count() > 160 {
        Err(WebError::BadRequest(format!("{field} is too long")))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn parse_time(value: &str) -> Result<(u8, u8, f64), WebError> {
    let parts = value.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err(WebError::BadRequest(
            "time must use HH:MM or HH:MM:SS".to_owned(),
        ));
    }
    let hour = parse_component(parts[0], "hour")?;
    let minute = parse_component(parts[1], "minute")?;
    let second = if parts.len() == 3 {
        parts[2]
            .parse::<f64>()
            .map_err(|_| WebError::BadRequest("invalid second".to_owned()))?
    } else {
        0.0
    };
    Ok((hour, minute, second))
}

fn parse_component(value: &str, name: &str) -> Result<u8, WebError> {
    value
        .parse::<u8>()
        .map_err(|_| WebError::BadRequest(format!("invalid {name}")))
}

fn parse_purpose(value: &str) -> Result<ChartPurpose, WebError> {
    match value {
        "natal" => Ok(ChartPurpose::Natal),
        "event" => Ok(ChartPurpose::Event),
        "horary" => Ok(ChartPurpose::Horary),
        "electional" => Ok(ChartPurpose::Electional),
        "mundane" => Ok(ChartPurpose::Mundane),
        _ => Err(WebError::BadRequest(format!(
            "unknown chart purpose: {value}"
        ))),
    }
}

fn parse_house_system(value: &str) -> Result<TraditionalHouseSystem, WebError> {
    match value {
        "whole_sign" => Ok(TraditionalHouseSystem::WholeSign),
        "equal" => Ok(TraditionalHouseSystem::Equal),
        "porphyry" => Ok(TraditionalHouseSystem::Porphyry),
        "alcabitius" => Ok(TraditionalHouseSystem::Alcabitius),
        "placidus" => Ok(TraditionalHouseSystem::Placidus),
        "regiomontanus" => Ok(TraditionalHouseSystem::Regiomontanus),
        "campanus" => Ok(TraditionalHouseSystem::Campanus),
        "morinus" => Ok(TraditionalHouseSystem::Morinus),
        _ => Err(WebError::BadRequest(format!(
            "unknown house system: {value}"
        ))),
    }
}
