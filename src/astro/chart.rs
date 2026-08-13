use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::aspects::{AspectHit, AspectPoint, OrbPolicy, find_aspects};
use super::dignity::{Almuten, PlanetCondition, calculate_almuten, calculate_conditions};
use super::ephemeris::{DATA_REVISION, ENGINE_VERSION, EphemerisError, SwissEphemerisProvider};
use super::lots::{Lot, LotError, LotKind, calculate_lots};
use super::time::{ResolvedMoment, TimeError, resolve_moment};
use super::types::{
    ChartRequest, HouseCusps, Planet, PlanetPosition, PointId, Sect, ZodiacSign, normalize_degrees,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LunarPhase {
    New,
    Crescent,
    FirstQuarter,
    Gibbous,
    Full,
    Disseminating,
    LastQuarter,
    Balsamic,
}

impl LunarPhase {
    #[must_use]
    pub fn from_elongation(elongation: f64) -> Self {
        match normalize_degrees(elongation) {
            value if !(22.5..337.5).contains(&value) => Self::New,
            value if value < 67.5 => Self::Crescent,
            value if value < 112.5 => Self::FirstQuarter,
            value if value < 157.5 => Self::Gibbous,
            value if value < 202.5 => Self::Full,
            value if value < 247.5 => Self::Disseminating,
            value if value < 292.5 => Self::LastQuarter,
            _ => Self::Balsamic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartMetadata {
    pub application_version: String,
    pub engine_version: String,
    pub data_revision: String,
    pub ephemeris_source: String,
    pub zodiac: String,
    pub planet_set: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    pub request: ChartRequest,
    pub moment: ResolvedMoment,
    pub positions: Vec<PlanetPosition>,
    pub houses: HouseCusps,
    pub sect: Sect,
    pub solar_altitude: f64,
    pub lunar_phase: LunarPhase,
    pub lunar_elongation: f64,
    pub aspects: Vec<AspectHit>,
    pub orb_policy: OrbPolicy,
    pub lots: Vec<Lot>,
    pub conditions: Vec<PlanetCondition>,
    pub almuten: Almuten,
    pub chart_ruler: Planet,
    pub metadata: ChartMetadata,
}

impl Chart {
    #[must_use]
    pub fn planet(&self, planet: Planet) -> Option<&PlanetPosition> {
        self.positions
            .iter()
            .find(|position| position.planet == planet)
    }

    #[must_use]
    pub fn lot(&self, kind: LotKind) -> Option<&Lot> {
        self.lots.iter().find(|lot| lot.kind == kind)
    }

    #[must_use]
    pub fn point_longitude(&self, point: PointId) -> Option<f64> {
        match point {
            PointId::Planet(planet) => self.planet(planet).map(|position| position.longitude),
            PointId::Ascendant => Some(self.houses.ascendant),
            PointId::Midheaven => Some(self.houses.midheaven),
            PointId::LotFortune => self.lot(LotKind::Fortune).map(|lot| lot.longitude),
            PointId::LotSpirit => self.lot(LotKind::Spirit).map(|lot| lot.longitude),
        }
    }
}

#[derive(Debug, Error)]
pub enum ChartError {
    #[error("invalid coordinates: {0}")]
    Coordinates(&'static str),
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error(transparent)]
    Ephemeris(#[from] EphemerisError),
    #[error("classical doctrine could not be derived: {0}")]
    Doctrine(String),
}

#[derive(Debug, Clone)]
pub struct ChartCalculator {
    provider: Arc<SwissEphemerisProvider>,
    orb_policy: OrbPolicy,
}

impl ChartCalculator {
    #[must_use]
    pub fn new(provider: SwissEphemerisProvider) -> Self {
        Self {
            provider: Arc::new(provider),
            orb_policy: OrbPolicy::default(),
        }
    }

    #[must_use]
    pub fn with_orb_policy(mut self, orb_policy: OrbPolicy) -> Self {
        self.orb_policy = orb_policy;
        self
    }

    #[must_use]
    pub fn provider(&self) -> &SwissEphemerisProvider {
        &self.provider
    }

    pub fn calculate(&self, request: ChartRequest) -> Result<Chart, ChartError> {
        let coordinates = request
            .coordinates
            .validate()
            .map_err(ChartError::Coordinates)?;
        let moment = resolve_moment(&request.local_time, &request.time_zone)?;
        let (positions, houses) =
            self.provider
                .positions_and_houses(moment.jd_ut, coordinates, request.house_system)?;
        let sun = require_planet(&positions, Planet::Sun)?;
        let moon = require_planet(&positions, Planet::Moon)?;
        let solar_altitude = altitude(
            sun.right_ascension,
            sun.declination,
            houses.armc,
            coordinates.latitude,
        );
        let sect = if solar_altitude >= 0.0 {
            Sect::Day
        } else {
            Sect::Night
        };
        let lots = calculate_lots(houses.ascendant, sect, &positions, &houses)
            .map_err(doctrine_from_lot)?;
        let aspects = calculate_chart_aspects(&positions, &houses, &lots, &self.orb_policy);
        let conditions = calculate_conditions(&positions, sect);
        let fortune = lots
            .iter()
            .find(|lot| lot.kind == LotKind::Fortune)
            .ok_or_else(|| ChartError::Doctrine("Lot of Fortune is missing".to_owned()))?;
        let almuten = calculate_almuten(
            sect,
            &[
                sun.longitude,
                moon.longitude,
                houses.ascendant,
                houses.midheaven,
                fortune.longitude,
            ],
        );
        let lunar_elongation = normalize_degrees(moon.longitude - sun.longitude);
        let chart_ruler = ZodiacSign::from_longitude(houses.ascendant).ruler();

        Ok(Chart {
            request,
            moment,
            positions,
            houses,
            sect,
            solar_altitude,
            lunar_phase: LunarPhase::from_elongation(lunar_elongation),
            lunar_elongation,
            aspects,
            orb_policy: self.orb_policy.clone(),
            lots,
            conditions,
            almuten,
            chart_ruler,
            metadata: ChartMetadata {
                application_version: env!("CARGO_PKG_VERSION").to_owned(),
                engine_version: ENGINE_VERSION.to_owned(),
                data_revision: DATA_REVISION.to_owned(),
                ephemeris_source: "Swiss Ephemeris DE441 coefficient files".to_owned(),
                zodiac: "Tropical, true equinox of date".to_owned(),
                planet_set: "Classical septenary".to_owned(),
            },
        })
    }

    pub fn calculate_with_orb_policy(
        &self,
        request: ChartRequest,
        orb_policy: OrbPolicy,
    ) -> Result<Chart, ChartError> {
        let mut configured = self.clone();
        configured.orb_policy = orb_policy;
        configured.calculate(request)
    }
}

fn altitude(right_ascension: f64, declination: f64, armc: f64, latitude: f64) -> f64 {
    let hour_angle = normalize_degrees(armc - right_ascension).to_radians();
    let declination = declination.to_radians();
    let latitude = latitude.to_radians();
    (latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour_angle.cos())
        .clamp(-1.0, 1.0)
        .asin()
        .to_degrees()
}

fn require_planet(
    positions: &[PlanetPosition],
    planet: Planet,
) -> Result<&PlanetPosition, ChartError> {
    positions
        .iter()
        .find(|position| position.planet == planet)
        .ok_or_else(|| ChartError::Doctrine(format!("{planet} is missing")))
}

fn doctrine_from_lot(error: LotError) -> ChartError {
    ChartError::Doctrine(error.to_string())
}

fn calculate_chart_aspects(
    positions: &[PlanetPosition],
    houses: &HouseCusps,
    lots: &[Lot],
    policy: &OrbPolicy,
) -> Vec<AspectHit> {
    let mut points = positions
        .iter()
        .map(|position| AspectPoint {
            id: PointId::Planet(position.planet),
            longitude: position.longitude,
            speed: Some(position.speed_longitude),
        })
        .collect::<Vec<_>>();
    points.extend([
        AspectPoint {
            id: PointId::Ascendant,
            longitude: houses.ascendant,
            speed: None,
        },
        AspectPoint {
            id: PointId::Midheaven,
            longitude: houses.midheaven,
            speed: None,
        },
    ]);
    for lot in lots {
        let id = match lot.kind {
            LotKind::Fortune => Some(PointId::LotFortune),
            LotKind::Spirit => Some(PointId::LotSpirit),
            LotKind::Eros
            | LotKind::Necessity
            | LotKind::Courage
            | LotKind::Victory
            | LotKind::Nemesis => None,
        };
        if let Some(id) = id {
            points.push(AspectPoint {
                id,
                longitude: lot.longitude,
                speed: None,
            });
        }
    }
    find_aspects(&points, policy)
}

#[cfg(test)]
mod tests {
    use super::{ChartCalculator, ChartError, LunarPhase, altitude};
    use crate::astro::ephemeris::SwissEphemerisProvider;
    use crate::astro::types::{
        Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, TimeZoneSpec,
        TraditionalHouseSystem,
    };

    #[test]
    fn calculates_a_complete_septenary_chart() -> Result<(), ChartError> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let calculator = ChartCalculator::new(provider);
        let chart = calculator.calculate(ChartRequest {
            title: "J2000".to_owned(),
            purpose: ChartPurpose::Event,
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
        })?;
        assert_eq!(chart.positions.len(), 7);
        assert_eq!(chart.lots.len(), 7);
        assert!(!chart.aspects.is_empty());
        assert!(matches!(chart.lunar_phase, LunarPhase::Balsamic));
        assert!(chart.solar_altitude > 0.0);
        assert_eq!(chart.metadata.planet_set, "Classical septenary");
        Ok(())
    }

    #[test]
    fn altitude_uses_equatorial_horizon_geometry() {
        assert!((altitude(0.0, 0.0, 0.0, 0.0) - 90.0).abs() < 1e-10);
        assert!((altitude(0.0, 0.0, 180.0, 0.0) + 90.0).abs() < 1e-10);
    }
}
