use serde::{Deserialize, Serialize};
use swisseph::date::day_of_week;

use super::TimingError;
use crate::astro::ephemeris::SwissEphemerisProvider;
use crate::astro::types::{Coordinates, Planet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryHour {
    pub number: u8,
    pub ruler: Planet,
    pub starts_jd_ut: f64,
    pub ends_jd_ut: f64,
    pub is_daylight: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryHours {
    pub sunrise_jd_ut: f64,
    pub sunset_jd_ut: f64,
    pub next_sunrise_jd_ut: f64,
    pub day_ruler: Planet,
    pub hours: Vec<PlanetaryHour>,
}

pub(super) fn calculate(
    provider: &SwissEphemerisProvider,
    search_start_jd: f64,
    coordinates: Coordinates,
) -> Result<PlanetaryHours, TimingError> {
    if !search_start_jd.is_finite() {
        return Err(TimingError::InvalidRange(
            "planetary-hour start must be finite".to_owned(),
        ));
    }
    let (sunrise, sunset) = provider.sunrise_sunset(search_start_jd, coordinates)?;
    let (next_sunrise, _) = provider.sunrise_sunset(sunset + 1.0 / 1440.0, coordinates)?;
    let local_mean_sunrise = sunrise + coordinates.longitude / 360.0;
    let day_ruler = weekday_ruler(day_of_week(local_mean_sunrise));
    let start_index = chaldean_index(day_ruler);
    let day_length = (sunset - sunrise) / 12.0;
    let night_length = (next_sunrise - sunset) / 12.0;
    let mut hours = Vec::with_capacity(24);
    for number in 0..24_u8 {
        let is_daylight = number < 12;
        let (starts_jd_ut, ends_jd_ut) = if is_daylight {
            let start = sunrise + day_length * f64::from(number);
            (start, start + day_length)
        } else {
            let night_number = number - 12;
            let start = sunset + night_length * f64::from(night_number);
            (start, start + night_length)
        };
        hours.push(PlanetaryHour {
            number: number + 1,
            ruler: CHALDEAN[(start_index + usize::from(number)) % CHALDEAN.len()],
            starts_jd_ut,
            ends_jd_ut,
            is_daylight,
        });
    }
    Ok(PlanetaryHours {
        sunrise_jd_ut: sunrise,
        sunset_jd_ut: sunset,
        next_sunrise_jd_ut: next_sunrise,
        day_ruler,
        hours,
    })
}

const CHALDEAN: [Planet; 7] = [
    Planet::Saturn,
    Planet::Jupiter,
    Planet::Mars,
    Planet::Sun,
    Planet::Venus,
    Planet::Mercury,
    Planet::Moon,
];

const fn chaldean_index(planet: Planet) -> usize {
    match planet {
        Planet::Saturn => 0,
        Planet::Jupiter => 1,
        Planet::Mars => 2,
        Planet::Sun => 3,
        Planet::Venus => 4,
        Planet::Mercury => 5,
        Planet::Moon => 6,
    }
}

const fn weekday_ruler(weekday: u8) -> Planet {
    match weekday % 7 {
        0 => Planet::Moon,
        1 => Planet::Mars,
        2 => Planet::Mercury,
        3 => Planet::Jupiter,
        4 => Planet::Venus,
        5 => Planet::Saturn,
        _ => Planet::Sun,
    }
}

#[cfg(test)]
mod tests {
    use super::calculate;
    use crate::astro::{Coordinates, SwissEphemerisProvider};

    #[test]
    fn planetary_day_has_twelve_day_and_twelve_night_hours()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let hours = calculate(
            &provider,
            2_451_544.5,
            Coordinates {
                latitude: 51.4779,
                longitude: 0.0,
                elevation_m: 46.0,
            },
        )?;
        assert_eq!(hours.hours.len(), 24);
        assert_eq!(
            hours.hours.iter().filter(|hour| hour.is_daylight).count(),
            12
        );
        assert!(hours.sunrise_jd_ut < hours.sunset_jd_ut);
        assert!(hours.sunset_jd_ut < hours.next_sunrise_jd_ut);
        Ok(())
    }
}
