use serde::{Deserialize, Serialize};
use swisseph::CalendarType;
use swisseph::date::revjul;

use super::{FirdariaPeriod, TimingError};
use crate::astro::aspects::{AspectKind, OrbPolicy, identify_aspect};
use crate::astro::chart::{Chart, ChartCalculator};
use crate::astro::ephemeris::SwissEphemerisProvider;
use crate::astro::types::{
    Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, Planet, PointId,
    TimeZoneSpec, TraditionalHouseSystem, ZodiacSign, normalize_degrees,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TechniqueKind {
    SecondaryProgression,
    SolarArc,
    Harmonic(u16),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniquePosition {
    pub planet: Planet,
    pub longitude: f64,
    pub sign: ZodiacSign,
    pub degree_in_sign: f64,
    pub natal_house: u8,
    pub retrograde: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniqueContact {
    pub moving: Planet,
    pub natal: Planet,
    pub aspect: AspectKind,
    pub orb: f64,
    pub partile: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniqueChart {
    pub title: String,
    pub technique: TechniqueKind,
    pub natal_jd_ut: f64,
    pub target_jd_ut: f64,
    pub symbolic_jd_ut: Option<f64>,
    pub key_degrees: Option<f64>,
    pub positions: Vec<TechniquePosition>,
    pub contacts: Vec<TechniqueContact>,
    pub orb_policy: OrbPolicy,
}

pub(super) fn secondary_progressions(
    provider: &SwissEphemerisProvider,
    natal: &Chart,
    target_jd: f64,
    policy: &OrbPolicy,
) -> Result<TechniqueChart, TimingError> {
    validate_target(natal, target_jd)?;
    let elapsed_years = (target_jd - natal.moment.jd_ut) / 365.2425;
    let symbolic_jd = natal.moment.jd_ut + elapsed_years;
    let mut positions = Vec::with_capacity(7);
    for planet in Planet::ALL {
        let raw = provider.ecliptic_position(symbolic_jd, planet)?;
        let longitude = normalize_degrees(raw[0]);
        positions.push(TechniquePosition {
            planet,
            longitude,
            sign: ZodiacSign::from_longitude(longitude),
            degree_in_sign: longitude.rem_euclid(30.0),
            natal_house: natal.houses.house_of(longitude),
            retrograde: raw[3] < 0.0,
        });
    }
    Ok(TechniqueChart {
        title: format!("{} · secondary progressions", natal.request.title),
        technique: TechniqueKind::SecondaryProgression,
        natal_jd_ut: natal.moment.jd_ut,
        target_jd_ut: target_jd,
        symbolic_jd_ut: Some(symbolic_jd),
        key_degrees: None,
        contacts: contacts(&positions, natal, policy),
        positions,
        orb_policy: policy.clone(),
    })
}

pub(super) fn solar_arc(
    provider: &SwissEphemerisProvider,
    natal: &Chart,
    target_jd: f64,
    policy: &OrbPolicy,
) -> Result<TechniqueChart, TimingError> {
    validate_target(natal, target_jd)?;
    let elapsed_years = (target_jd - natal.moment.jd_ut) / 365.2425;
    let symbolic_jd = natal.moment.jd_ut + elapsed_years;
    let natal_sun = natal
        .planet(Planet::Sun)
        .ok_or(TimingError::MissingPlanet(Planet::Sun))?;
    let progressed_sun = provider.ecliptic_position(symbolic_jd, Planet::Sun)?;
    let arc = normalize_degrees(progressed_sun[0] - natal_sun.longitude);
    let positions = natal
        .positions
        .iter()
        .map(|position| {
            let longitude = normalize_degrees(position.longitude + arc);
            TechniquePosition {
                planet: position.planet,
                longitude,
                sign: ZodiacSign::from_longitude(longitude),
                degree_in_sign: longitude.rem_euclid(30.0),
                natal_house: natal.houses.house_of(longitude),
                retrograde: false,
            }
        })
        .collect::<Vec<_>>();
    Ok(TechniqueChart {
        title: format!("{} · solar arc", natal.request.title),
        technique: TechniqueKind::SolarArc,
        natal_jd_ut: natal.moment.jd_ut,
        target_jd_ut: target_jd,
        symbolic_jd_ut: Some(symbolic_jd),
        key_degrees: Some(arc),
        contacts: contacts(&positions, natal, policy),
        positions,
        orb_policy: policy.clone(),
    })
}

pub(super) fn harmonic(
    natal: &Chart,
    harmonic: u16,
    policy: &OrbPolicy,
) -> Result<TechniqueChart, TimingError> {
    if !(1..=360).contains(&harmonic) {
        return Err(TimingError::InvalidRange(
            "harmonic must be between 1 and 360".to_owned(),
        ));
    }
    let positions = natal
        .positions
        .iter()
        .map(|position| {
            let longitude = normalize_degrees(position.longitude * f64::from(harmonic));
            TechniquePosition {
                planet: position.planet,
                longitude,
                sign: ZodiacSign::from_longitude(longitude),
                degree_in_sign: longitude.rem_euclid(30.0),
                natal_house: natal.houses.house_of(longitude),
                retrograde: position.retrograde,
            }
        })
        .collect::<Vec<_>>();
    Ok(TechniqueChart {
        title: format!("{} · harmonic {harmonic}", natal.request.title),
        technique: TechniqueKind::Harmonic(harmonic),
        natal_jd_ut: natal.moment.jd_ut,
        target_jd_ut: natal.moment.jd_ut,
        symbolic_jd_ut: None,
        key_degrees: Some(f64::from(harmonic)),
        contacts: contacts(&positions, natal, policy),
        positions,
        orb_policy: policy.clone(),
    })
}

fn contacts(
    positions: &[TechniquePosition],
    natal: &Chart,
    policy: &OrbPolicy,
) -> Vec<TechniqueContact> {
    let mut output = Vec::new();
    for moving in positions {
        for radix in &natal.positions {
            if let Some((aspect, orb)) = identify_aspect(
                PointId::Planet(moving.planet),
                moving.longitude,
                PointId::Planet(radix.planet),
                radix.longitude,
                policy,
            ) {
                output.push(TechniqueContact {
                    moving: moving.planet,
                    natal: radix.planet,
                    aspect,
                    orb,
                    partile: orb < 1.0,
                });
            }
        }
    }
    output.sort_by(|left, right| left.orb.total_cmp(&right.orb));
    output
}

fn validate_target(natal: &Chart, target_jd: f64) -> Result<(), TimingError> {
    if !target_jd.is_finite() || target_jd < natal.moment.jd_ut {
        Err(TimingError::InvalidRange(
            "target must be a finite instant on or after the natal chart".to_owned(),
        ))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn chart_at_jd(
    calculator: &ChartCalculator,
    jd_ut: f64,
    title: String,
    location_name: String,
    coordinates: Coordinates,
    house_system: TraditionalHouseSystem,
) -> Result<Chart, crate::astro::ChartError> {
    let (year, month, day, hour_decimal) = revjul(jd_ut, CalendarType::Gregorian);
    let hour = hour_decimal.floor();
    let minute_decimal = (hour_decimal - hour) * 60.0;
    let minute = minute_decimal.floor();
    calculator.calculate(ChartRequest {
        title,
        purpose: ChartPurpose::Event,
        local_time: CivilDateTime {
            year,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
            second: (minute_decimal - minute) * 60.0,
            calendar: Calendar::Gregorian,
        },
        time_zone: TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC · exact ephemeris event".to_owned()),
        },
        location_name,
        coordinates,
        house_system,
    })
}

pub(super) fn firdaria(sect: crate::astro::Sect, age_years: f64) -> FirdariaPeriod {
    const DAY: [(Planet, f64); 7] = [
        (Planet::Sun, 10.0),
        (Planet::Venus, 8.0),
        (Planet::Mercury, 13.0),
        (Planet::Moon, 9.0),
        (Planet::Saturn, 11.0),
        (Planet::Jupiter, 12.0),
        (Planet::Mars, 7.0),
    ];
    const NIGHT: [(Planet, f64); 7] = [
        (Planet::Moon, 9.0),
        (Planet::Saturn, 11.0),
        (Planet::Jupiter, 12.0),
        (Planet::Mars, 7.0),
        (Planet::Sun, 10.0),
        (Planet::Venus, 8.0),
        (Planet::Mercury, 13.0),
    ];
    let sequence = match sect {
        crate::astro::Sect::Day => &DAY,
        crate::astro::Sect::Night => &NIGHT,
    };
    let age = age_years.max(0.0).rem_euclid(70.0);
    let mut start = 0.0;
    let mut major_index = 0;
    for (index, (_, duration)) in sequence.iter().enumerate() {
        if age < start + duration {
            major_index = index;
            break;
        }
        start += duration;
    }
    let (major_lord, duration) = sequence[major_index];
    let sub_duration = duration / 7.0;
    let sub_index = (((age - start) / sub_duration).floor() as usize).min(6);
    let sub_lord = sequence[(major_index + sub_index) % 7].0;
    FirdariaPeriod {
        sect,
        age_years,
        major_lord,
        sub_lord,
        major_started_at_age: start,
        major_ends_at_age: start + duration,
        sub_started_at_age: start + sub_duration * sub_index as f64,
        sub_ends_at_age: start + sub_duration * (sub_index + 1) as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::firdaria;
    use crate::astro::{Planet, Sect};

    #[test]
    fn day_firdaria_begins_with_sun() {
        let period = firdaria(Sect::Day, 0.0);
        assert_eq!(period.major_lord, Planet::Sun);
        assert_eq!(period.sub_lord, Planet::Sun);
    }

    #[test]
    fn node_periods_are_absent_from_the_septenary_cycle() {
        let period = firdaria(Sect::Night, 69.9);
        assert!(Planet::ALL.contains(&period.major_lord));
        assert!(Planet::ALL.contains(&period.sub_lord));
    }
}
