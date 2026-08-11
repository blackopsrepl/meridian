use serde::{Deserialize, Serialize};
use swisseph::EclipseFlags;
use thiserror::Error;

use super::ephemeris::{EphemerisError, SwissEphemerisProvider};
use super::types::{Planet, ZodiacSign, normalize_degrees};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemerisCell {
    pub planet: Planet,
    pub longitude: f64,
    pub sign: ZodiacSign,
    pub degree_in_sign: f64,
    pub speed_longitude: f64,
    pub retrograde: bool,
    pub declination: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemerisRow {
    pub jd_ut: f64,
    pub positions: Vec<EphemerisCell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemerisTable {
    pub start_jd_ut: f64,
    pub step_days: f64,
    pub rows: Vec<EphemerisRow>,
}

impl EphemerisTable {
    pub fn calculate(
        provider: &SwissEphemerisProvider,
        start_jd_ut: f64,
        row_count: usize,
        step_days: f64,
    ) -> Result<Self, EventError> {
        if !start_jd_ut.is_finite()
            || !step_days.is_finite()
            || step_days <= 0.0
            || row_count == 0
            || row_count > 5000
        {
            return Err(EventError::InvalidRange);
        }
        let mut rows = Vec::with_capacity(row_count);
        for index in 0..row_count {
            let jd_ut = start_jd_ut + index as f64 * step_days;
            let mut positions = Vec::with_capacity(7);
            for planet in Planet::ALL {
                let ecliptic = provider.ecliptic_position(jd_ut, planet)?;
                let equatorial = provider.equatorial_position(jd_ut, planet)?;
                let longitude = normalize_degrees(ecliptic[0]);
                positions.push(EphemerisCell {
                    planet,
                    longitude,
                    sign: ZodiacSign::from_longitude(longitude),
                    degree_in_sign: longitude.rem_euclid(30.0),
                    speed_longitude: ecliptic[3],
                    retrograde: ecliptic[3] < 0.0,
                    declination: equatorial[1],
                });
            }
            rows.push(EphemerisRow { jd_ut, positions });
        }
        Ok(Self {
            start_jd_ut,
            step_days,
            rows,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionChange {
    StationsDirect,
    StationsRetrograde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LunationKind {
    NewMoon,
    FullMoon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EclipseKind {
    Total,
    Annular,
    Hybrid,
    Partial,
    Penumbral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkyEventKind {
    Ingress {
        planet: Planet,
        sign: ZodiacSign,
    },
    Station {
        planet: Planet,
        change: MotionChange,
    },
    Lunation {
        phase: LunationKind,
    },
    SolarEclipse {
        eclipse: EclipseKind,
    },
    LunarEclipse {
        eclipse: EclipseKind,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkyEvent {
    pub jd_ut: f64,
    pub longitude: Option<f64>,
    pub event: SkyEventKind,
}

#[derive(Debug, Error)]
pub enum EventError {
    #[error("event range is invalid or exceeds 10 years")]
    InvalidRange,
    #[error(transparent)]
    Ephemeris(#[from] EphemerisError),
}

#[derive(Debug, Clone)]
pub struct SkyEventSearch {
    provider: SwissEphemerisProvider,
}

impl SkyEventSearch {
    #[must_use]
    pub fn new(provider: SwissEphemerisProvider) -> Self {
        Self { provider }
    }

    pub fn search(&self, start_jd: f64, end_jd: f64) -> Result<Vec<SkyEvent>, EventError> {
        if !start_jd.is_finite()
            || !end_jd.is_finite()
            || end_jd <= start_jd
            || end_jd - start_jd > 3660.0
        {
            return Err(EventError::InvalidRange);
        }
        let mut events = Vec::new();
        for planet in Planet::ALL {
            self.planet_events(planet, start_jd, end_jd, &mut events)?;
        }
        self.lunations(start_jd, end_jd, &mut events)?;
        self.eclipses(start_jd, end_jd, &mut events)?;
        events.sort_by(|left, right| left.jd_ut.total_cmp(&right.jd_ut));
        events.dedup_by(|left, right| {
            left.event == right.event && (left.jd_ut - right.jd_ut).abs() < 1e-5
        });
        Ok(events)
    }

    fn planet_events(
        &self,
        planet: Planet,
        start: f64,
        end: f64,
        output: &mut Vec<SkyEvent>,
    ) -> Result<(), EventError> {
        let step = if planet == Planet::Moon { 0.2 } else { 0.5 };
        let mut left_jd = start;
        let mut left = self.provider.ecliptic_position(left_jd, planet)?;
        while left_jd < end {
            let right_jd = (left_jd + step).min(end);
            let right = self.provider.ecliptic_position(right_jd, planet)?;
            let left_sign = ZodiacSign::from_longitude(left[0]);
            let right_sign = ZodiacSign::from_longitude(right[0]);
            if left_sign != right_sign {
                let boundary = ingress_boundary(left[0], left[3] + right[3]);
                let exact = self.bisect_longitude(planet, boundary, left_jd, right_jd)?;
                let raw = self.provider.ecliptic_position(exact, planet)?;
                output.push(SkyEvent {
                    jd_ut: exact,
                    longitude: Some(normalize_degrees(raw[0])),
                    event: SkyEventKind::Ingress {
                        planet,
                        sign: ZodiacSign::from_longitude(raw[0] + raw[3].signum() * 1e-5),
                    },
                });
            }
            if left[3] != 0.0 && right[3] != 0.0 && left[3].signum() != right[3].signum() {
                let exact = self.bisect_speed(planet, left_jd, right_jd)?;
                let raw = self.provider.ecliptic_position(exact, planet)?;
                output.push(SkyEvent {
                    jd_ut: exact,
                    longitude: Some(normalize_degrees(raw[0])),
                    event: SkyEventKind::Station {
                        planet,
                        change: if right[3] > 0.0 {
                            MotionChange::StationsDirect
                        } else {
                            MotionChange::StationsRetrograde
                        },
                    },
                });
            }
            left_jd = right_jd;
            left = right;
        }
        Ok(())
    }

    fn lunations(
        &self,
        start: f64,
        end: f64,
        output: &mut Vec<SkyEvent>,
    ) -> Result<(), EventError> {
        for (target, phase) in [
            (0.0, LunationKind::NewMoon),
            (180.0, LunationKind::FullMoon),
        ] {
            let mut left_jd = start;
            let mut left = self.lunation_value(left_jd, target)?;
            while left_jd < end {
                let right_jd = (left_jd + 0.25).min(end);
                let right = self.lunation_value(right_jd, target)?;
                if crosses_zero(left, right) {
                    let exact = self.bisect_lunation(target, left_jd, right_jd)?;
                    let moon = self.provider.ecliptic_position(exact, Planet::Moon)?;
                    output.push(SkyEvent {
                        jd_ut: exact,
                        longitude: Some(normalize_degrees(moon[0])),
                        event: SkyEventKind::Lunation { phase },
                    });
                }
                left_jd = right_jd;
                left = right;
            }
        }
        Ok(())
    }

    fn eclipses(&self, start: f64, end: f64, output: &mut Vec<SkyEvent>) -> Result<(), EventError> {
        let mut cursor = start;
        loop {
            let eclipse = self.provider.next_solar_eclipse(cursor)?;
            if eclipse.time_maximum > end {
                break;
            }
            output.push(SkyEvent {
                jd_ut: eclipse.time_maximum,
                longitude: None,
                event: SkyEventKind::SolarEclipse {
                    eclipse: classify_eclipse(eclipse.flags),
                },
            });
            cursor = eclipse.time_maximum + 1.0;
        }
        cursor = start;
        loop {
            let eclipse = self.provider.next_lunar_eclipse(cursor)?;
            if eclipse.time_maximum > end {
                break;
            }
            output.push(SkyEvent {
                jd_ut: eclipse.time_maximum,
                longitude: None,
                event: SkyEventKind::LunarEclipse {
                    eclipse: classify_eclipse(eclipse.flags),
                },
            });
            cursor = eclipse.time_maximum + 1.0;
        }
        Ok(())
    }

    fn bisect_longitude(
        &self,
        planet: Planet,
        target: f64,
        left: f64,
        right: f64,
    ) -> Result<f64, EventError> {
        Self::bisect(left, right, |jd| {
            let longitude = self.provider.ecliptic_position(jd, planet)?[0];
            Ok(signed_degrees(longitude - target))
        })
    }

    fn bisect_speed(&self, planet: Planet, left: f64, right: f64) -> Result<f64, EventError> {
        Self::bisect(left, right, |jd| {
            Ok(self.provider.ecliptic_position(jd, planet)?[3])
        })
    }

    fn bisect_lunation(&self, target: f64, left: f64, right: f64) -> Result<f64, EventError> {
        Self::bisect(left, right, |jd| self.lunation_value(jd, target))
    }

    fn bisect<F>(mut left: f64, mut right: f64, value: F) -> Result<f64, EventError>
    where
        F: Fn(f64) -> Result<f64, EventError>,
    {
        let mut left_value = value(left)?;
        for _ in 0..48 {
            let middle = f64::midpoint(left, right);
            let middle_value = value(middle)?;
            if middle_value.abs() < 1e-9 || right - left < 1e-8 {
                return Ok(middle);
            }
            if crosses_zero(left_value, middle_value) {
                right = middle;
            } else {
                left = middle;
                left_value = middle_value;
            }
        }
        Ok(f64::midpoint(left, right))
    }

    fn lunation_value(&self, jd: f64, target: f64) -> Result<f64, EventError> {
        let sun = self.provider.ecliptic_position(jd, Planet::Sun)?[0];
        let moon = self.provider.ecliptic_position(jd, Planet::Moon)?[0];
        Ok(signed_degrees(moon - sun - target))
    }
}

fn ingress_boundary(longitude: f64, combined_speed: f64) -> f64 {
    let normalized = normalize_degrees(longitude);
    let sign_start = (normalized / 30.0).floor() * 30.0;
    if combined_speed >= 0.0 {
        normalize_degrees(sign_start + 30.0)
    } else {
        sign_start
    }
}

fn signed_degrees(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}

fn crosses_zero(left: f64, right: f64) -> bool {
    (left == 0.0 || right == 0.0 || left.signum() != right.signum()) && (left - right).abs() < 180.0
}

fn classify_eclipse(flags: EclipseFlags) -> EclipseKind {
    if flags.contains(EclipseFlags::TOTAL) {
        EclipseKind::Total
    } else if flags.contains(EclipseFlags::ANNULAR) {
        EclipseKind::Annular
    } else if flags.contains(EclipseFlags::HYBRID) {
        EclipseKind::Hybrid
    } else if flags.contains(EclipseFlags::PENUMBRAL) {
        EclipseKind::Penumbral
    } else {
        EclipseKind::Partial
    }
}

#[cfg(test)]
mod tests {
    use super::EphemerisTable;
    use crate::astro::SwissEphemerisProvider;

    #[test]
    fn table_contains_only_seven_planets() -> Result<(), Box<dyn std::error::Error>> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let table = EphemerisTable::calculate(&provider, 2_451_545.0, 2, 1.0)?;
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|row| row.positions.len() == 7));
        Ok(())
    }
}
