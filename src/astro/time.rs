use std::str::FromStr;

use chrono::{Datelike, LocalResult, NaiveDate, Offset, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use swisseph::CalendarType;
use swisseph::date::{date_conversion, julday, revjul};
use thiserror::Error;

use super::types::{Calendar, CivilDateTime, TimeZoneSpec};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedMoment {
    pub jd_ut: f64,
    pub utc: CivilDateTime,
    pub offset_minutes: i32,
    pub time_zone_label: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimeError {
    #[error("invalid calendar date")]
    InvalidDate,
    #[error("hour, minute, or second is outside its valid range")]
    InvalidClock,
    #[error("unknown IANA time zone: {0}")]
    InvalidTimeZone(String),
    #[error("local time {local} is ambiguous in {zone}; provide fold 0 or 1")]
    Ambiguous { local: String, zone: String },
    #[error("local time {local} does not exist in {zone} because of a clock transition")]
    Nonexistent { local: String, zone: String },
    #[error("IANA time zones require the proleptic Gregorian calendar and years 1 through 9999")]
    UnsupportedIanaDate,
    #[error("fold must be 0 or 1")]
    InvalidFold,
    #[error("fixed UTC offset must be between -24 and +24 hours")]
    InvalidOffset,
}

pub fn resolve_moment(
    local: &CivilDateTime,
    zone: &TimeZoneSpec,
) -> Result<ResolvedMoment, TimeError> {
    validate_clock(local)?;
    validate_date(local)?;
    match zone {
        TimeZoneSpec::Iana { name, fold } => resolve_iana(local, name, *fold),
        TimeZoneSpec::FixedOffset {
            minutes_east,
            label,
        } => resolve_fixed(local, *minutes_east, label.as_deref()),
    }
}

fn resolve_iana(
    local: &CivilDateTime,
    zone_name: &str,
    fold: Option<u8>,
) -> Result<ResolvedMoment, TimeError> {
    if local.calendar != Calendar::Gregorian || !(1..=9999).contains(&local.year) {
        return Err(TimeError::UnsupportedIanaDate);
    }
    if fold.is_some_and(|value| value > 1) {
        return Err(TimeError::InvalidFold);
    }

    let zone =
        Tz::from_str(zone_name).map_err(|_| TimeError::InvalidTimeZone(zone_name.to_owned()))?;
    let date = NaiveDate::from_ymd_opt(local.year, u32::from(local.month), u32::from(local.day))
        .ok_or(TimeError::InvalidDate)?;
    let seconds = local.second.floor() as u32;
    let nanos = ((local.second - f64::from(seconds)) * 1_000_000_000.0).round() as u32;
    let naive = date
        .and_hms_nano_opt(
            u32::from(local.hour),
            u32::from(local.minute),
            seconds,
            nanos,
        )
        .ok_or(TimeError::InvalidClock)?;
    let local_label = naive.format("%Y-%m-%d %H:%M:%S").to_string();

    let zoned = match zone.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(first, second) => match fold {
            Some(0) => first,
            Some(1) => second,
            Some(_) => return Err(TimeError::InvalidFold),
            None => {
                return Err(TimeError::Ambiguous {
                    local: local_label,
                    zone: zone_name.to_owned(),
                });
            }
        },
        LocalResult::None => {
            return Err(TimeError::Nonexistent {
                local: local_label,
                zone: zone_name.to_owned(),
            });
        }
    };

    let offset_minutes = zoned.offset().fix().local_minus_utc() / 60;
    let utc = zoned.with_timezone(&Utc);
    let utc_hour = f64::from(utc.hour())
        + f64::from(utc.minute()) / 60.0
        + (f64::from(utc.second()) + f64::from(utc.nanosecond()) / 1_000_000_000.0) / 3600.0;
    let jd_ut = julday(
        utc.year(),
        utc.month().cast_signed(),
        utc.day().cast_signed(),
        utc_hour,
        CalendarType::Gregorian,
    );

    Ok(ResolvedMoment {
        jd_ut,
        utc: civil_from_julian_day(jd_ut, Calendar::Gregorian),
        offset_minutes,
        time_zone_label: zone_name.to_owned(),
    })
}

fn resolve_fixed(
    local: &CivilDateTime,
    minutes_east: i32,
    label: Option<&str>,
) -> Result<ResolvedMoment, TimeError> {
    if !(-1440..=1440).contains(&minutes_east) {
        return Err(TimeError::InvalidOffset);
    }
    let calendar = to_swisseph_calendar(local.calendar);
    let local_hour = f64::from(local.hour) + f64::from(local.minute) / 60.0 + local.second / 3600.0;
    let jd_local = julday(
        local.year,
        i32::from(local.month),
        i32::from(local.day),
        local_hour,
        calendar,
    );
    let jd_ut = jd_local - f64::from(minutes_east) / 1440.0;
    let zone_label = label.map_or_else(
        || format_offset(minutes_east),
        std::borrow::ToOwned::to_owned,
    );

    Ok(ResolvedMoment {
        jd_ut,
        utc: civil_from_julian_day(jd_ut, local.calendar),
        offset_minutes: minutes_east,
        time_zone_label: zone_label,
    })
}

fn validate_clock(local: &CivilDateTime) -> Result<(), TimeError> {
    if local.hour > 23
        || local.minute > 59
        || !local.second.is_finite()
        || !(0.0..60.0).contains(&local.second)
    {
        return Err(TimeError::InvalidClock);
    }
    Ok(())
}

fn validate_date(local: &CivilDateTime) -> Result<(), TimeError> {
    let hour = f64::from(local.hour) + f64::from(local.minute) / 60.0 + local.second / 3600.0;
    date_conversion(
        local.year,
        i32::from(local.month),
        i32::from(local.day),
        hour,
        to_swisseph_calendar(local.calendar),
    )
    .map(|_| ())
    .map_err(|_| TimeError::InvalidDate)
}

const fn to_swisseph_calendar(calendar: Calendar) -> CalendarType {
    match calendar {
        Calendar::Gregorian => CalendarType::Gregorian,
        Calendar::Julian => CalendarType::Julian,
    }
}

#[must_use]
pub fn civil_from_julian_day(jd: f64, calendar: Calendar) -> CivilDateTime {
    let (year, month, day, fractional_hour) = revjul(jd, to_swisseph_calendar(calendar));
    let hour = fractional_hour.floor();
    let fractional_minute = (fractional_hour - hour) * 60.0;
    let minute = fractional_minute.floor();
    let second = (fractional_minute - minute) * 60.0;
    CivilDateTime {
        year,
        month: month as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second,
        calendar,
    }
}

fn format_offset(minutes_east: i32) -> String {
    let sign = if minutes_east < 0 { '-' } else { '+' };
    let absolute = minutes_east.abs();
    format!("UTC{sign}{:02}:{:02}", absolute / 60, absolute % 60)
}

#[cfg(test)]
mod tests {
    use super::{TimeError, resolve_moment};
    use crate::astro::types::{Calendar, CivilDateTime, TimeZoneSpec};

    fn local_time() -> CivilDateTime {
        CivilDateTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 13,
            minute: 0,
            second: 0.0,
            calendar: Calendar::Gregorian,
        }
    }

    #[test]
    fn fixed_offset_resolves_to_j2000() -> Result<(), TimeError> {
        let resolved = resolve_moment(
            &local_time(),
            &TimeZoneSpec::FixedOffset {
                minutes_east: 60,
                label: None,
            },
        )?;
        assert!((resolved.jd_ut - 2_451_545.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn iana_zone_uses_historical_offset() -> Result<(), TimeError> {
        let resolved = resolve_moment(
            &local_time(),
            &TimeZoneSpec::Iana {
                name: "Europe/Rome".to_owned(),
                fold: None,
            },
        )?;
        assert_eq!(resolved.offset_minutes, 60);
        assert!((resolved.jd_ut - 2_451_545.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn daylight_saving_gap_is_rejected() {
        let missing = CivilDateTime {
            year: 2026,
            month: 3,
            day: 29,
            hour: 2,
            minute: 30,
            second: 0.0,
            calendar: Calendar::Gregorian,
        };
        let result = resolve_moment(
            &missing,
            &TimeZoneSpec::Iana {
                name: "Europe/Rome".to_owned(),
                fold: None,
            },
        );
        assert!(matches!(result, Err(TimeError::Nonexistent { .. })));
    }
}
