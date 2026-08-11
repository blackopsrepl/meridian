use serde::{Deserialize, Serialize};
use swisseph::CalendarType;
use swisseph::date::revjul;

use super::aspects::{AspectHit, AspectPoint, OrbPolicy, find_aspects, identify_aspect};
use super::chart::{Chart, ChartCalculator, ChartError};
use super::types::{
    Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, HouseCusps, Planet,
    PlanetPosition, PointId, TimeZoneSpec, TraditionalHouseSystem, ZodiacSign, normalize_degrees,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynastryAspect {
    pub first: Planet,
    pub second: Planet,
    pub kind: super::aspects::AspectKind,
    pub orb: f64,
    pub partile: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseOverlay {
    pub planet: Planet,
    pub house: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutualReception {
    pub first: Planet,
    pub second: Planet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Synastry {
    pub first_title: String,
    pub second_title: String,
    pub aspects: Vec<SynastryAspect>,
    pub first_in_second_houses: Vec<HouseOverlay>,
    pub second_in_first_houses: Vec<HouseOverlay>,
    pub mutual_receptions: Vec<MutualReception>,
    pub orb_policy: OrbPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeChart {
    pub title: String,
    pub method: String,
    pub positions: Vec<PlanetPosition>,
    pub houses: HouseCusps,
    pub aspects: Vec<AspectHit>,
    pub orb_policy: OrbPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RelationshipCalculator {
    orb_policy: OrbPolicy,
}

impl RelationshipCalculator {
    #[must_use]
    pub fn with_orb_policy(mut self, policy: OrbPolicy) -> Self {
        self.orb_policy = policy;
        self
    }

    #[must_use]
    pub fn synastry(&self, first: &Chart, second: &Chart) -> Synastry {
        let mut aspects = Vec::new();
        for left in &first.positions {
            for right in &second.positions {
                if let Some((kind, orb)) = identify_aspect(
                    PointId::Planet(left.planet),
                    left.longitude,
                    PointId::Planet(right.planet),
                    right.longitude,
                    &self.orb_policy,
                ) {
                    aspects.push(SynastryAspect {
                        first: left.planet,
                        second: right.planet,
                        kind,
                        orb,
                        partile: orb < 1.0,
                    });
                }
            }
        }
        aspects.sort_by(|left, right| left.orb.total_cmp(&right.orb));
        Synastry {
            first_title: first.request.title.clone(),
            second_title: second.request.title.clone(),
            aspects,
            first_in_second_houses: overlays(&first.positions, &second.houses),
            second_in_first_houses: overlays(&second.positions, &first.houses),
            mutual_receptions: receptions(first, second),
            orb_policy: self.orb_policy.clone(),
        }
    }

    #[must_use]
    pub fn composite(&self, first: &Chart, second: &Chart, title: String) -> CompositeChart {
        let houses = midpoint_houses(&first.houses, &second.houses);
        let positions = Planet::ALL
            .into_iter()
            .filter_map(|planet| {
                let left = first.planet(planet)?;
                let right = second.planet(planet)?;
                Some(midpoint_position(left, right, &houses))
            })
            .collect::<Vec<_>>();
        let points = positions
            .iter()
            .map(|position| AspectPoint {
                id: PointId::Planet(position.planet),
                longitude: position.longitude,
                speed: Some(position.speed_longitude),
            })
            .collect::<Vec<_>>();
        CompositeChart {
            title,
            method: "circular midpoint".to_owned(),
            aspects: find_aspects(&points, &self.orb_policy),
            positions,
            houses,
            orb_policy: self.orb_policy.clone(),
        }
    }

    pub fn davison(
        &self,
        calculator: &ChartCalculator,
        first: &Chart,
        second: &Chart,
        title: String,
        house_system: TraditionalHouseSystem,
    ) -> Result<Chart, ChartError> {
        let jd_ut = f64::midpoint(first.moment.jd_ut, second.moment.jd_ut);
        let coordinates = geographic_midpoint(
            first.request.coordinates,
            second.request.coordinates,
        )
        .ok_or_else(|| {
            ChartError::Doctrine("Davison midpoint is undefined for antipodal places".to_owned())
        })?;
        let (year, month, day, hour_decimal) = revjul(jd_ut, CalendarType::Gregorian);
        let hour = hour_decimal.floor();
        let minute_decimal = (hour_decimal - hour) * 60.0;
        let minute = minute_decimal.floor();
        let second_value = (minute_decimal - minute) * 60.0;
        calculator.calculate(ChartRequest {
            title,
            purpose: ChartPurpose::Event,
            local_time: CivilDateTime {
                year,
                month: month as u8,
                day: day as u8,
                hour: hour as u8,
                minute: minute as u8,
                second: second_value,
                calendar: Calendar::Gregorian,
            },
            time_zone: TimeZoneSpec::FixedOffset {
                minutes_east: 0,
                label: Some("UTC · Davison midpoint".to_owned()),
            },
            location_name: format!(
                "Midpoint of {} and {}",
                first.request.location_name, second.request.location_name
            ),
            coordinates,
            house_system,
        })
    }
}

fn overlays(positions: &[PlanetPosition], houses: &HouseCusps) -> Vec<HouseOverlay> {
    positions
        .iter()
        .map(|position| HouseOverlay {
            planet: position.planet,
            house: houses.house_of(position.longitude),
        })
        .collect()
}

fn receptions(first: &Chart, second: &Chart) -> Vec<MutualReception> {
    let mut found = Vec::new();
    for left in &first.positions {
        for right in &second.positions {
            if left.sign.ruler() == right.planet && right.sign.ruler() == left.planet {
                found.push(MutualReception {
                    first: left.planet,
                    second: right.planet,
                });
            }
        }
    }
    found
}

fn midpoint_houses(first: &HouseCusps, second: &HouseCusps) -> HouseCusps {
    let mut cusps = [0.0; 12];
    for (index, destination) in cusps.iter_mut().enumerate() {
        *destination = circular_midpoint(first.cusps[index], second.cusps[index]);
    }
    HouseCusps {
        system: first.system,
        cusps,
        ascendant: circular_midpoint(first.ascendant, second.ascendant),
        midheaven: circular_midpoint(first.midheaven, second.midheaven),
        armc: circular_midpoint(first.armc, second.armc),
        vertex: circular_midpoint(first.vertex, second.vertex),
    }
}

fn midpoint_position(
    left: &PlanetPosition,
    right: &PlanetPosition,
    houses: &HouseCusps,
) -> PlanetPosition {
    let longitude = circular_midpoint(left.longitude, right.longitude);
    let sign = ZodiacSign::from_longitude(longitude);
    let speed_longitude = f64::midpoint(left.speed_longitude, right.speed_longitude);
    PlanetPosition {
        planet: left.planet,
        longitude,
        latitude: f64::midpoint(left.latitude, right.latitude),
        distance_au: f64::midpoint(left.distance_au, right.distance_au),
        speed_longitude,
        right_ascension: circular_midpoint(left.right_ascension, right.right_ascension),
        declination: f64::midpoint(left.declination, right.declination),
        house: houses.house_of(longitude),
        sign,
        degree_in_sign: longitude.rem_euclid(30.0),
        retrograde: speed_longitude < 0.0,
    }
}

fn circular_midpoint(first: f64, second: f64) -> f64 {
    let signed_arc = (second - first + 180.0).rem_euclid(360.0) - 180.0;
    normalize_degrees(first + signed_arc / 2.0)
}

fn geographic_midpoint(first: Coordinates, second: Coordinates) -> Option<Coordinates> {
    let first_lat = first.latitude.to_radians();
    let first_lon = first.longitude.to_radians();
    let second_lat = second.latitude.to_radians();
    let second_lon = second.longitude.to_radians();
    let x = first_lat.cos() * first_lon.cos() + second_lat.cos() * second_lon.cos();
    let y = first_lat.cos() * first_lon.sin() + second_lat.cos() * second_lon.sin();
    let z = first_lat.sin() + second_lat.sin();
    let magnitude = (x * x + y * y + z * z).sqrt();
    if magnitude < 1e-12 {
        return None;
    }
    Some(Coordinates {
        latitude: z.atan2((x * x + y * y).sqrt()).to_degrees(),
        longitude: y.atan2(x).to_degrees(),
        elevation_m: f64::midpoint(first.elevation_m, second.elevation_m),
    })
}

#[cfg(test)]
mod tests {
    use super::circular_midpoint;

    #[test]
    fn circular_midpoint_crosses_zero_on_short_arc() {
        assert!((circular_midpoint(350.0, 10.0) - 0.0).abs() < f64::EPSILON);
        assert!((circular_midpoint(10.0, 30.0) - 20.0).abs() < f64::EPSILON);
    }
}
