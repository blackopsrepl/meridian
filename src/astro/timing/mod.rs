mod hours;
mod techniques;
mod transits;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{ChartError, EphemerisError, Planet};

pub use hours::{PlanetaryHour, PlanetaryHours};
pub use techniques::{TechniqueChart, TechniqueContact, TechniqueKind, TechniquePosition};
pub use transits::TransitEvent;

use super::aspects::OrbPolicy;
use super::chart::{Chart, ChartCalculator};
use super::ephemeris::SwissEphemerisProvider;
use super::types::{Coordinates, TraditionalHouseSystem};

#[derive(Debug, Error)]
pub enum TimingError {
    #[error(transparent)]
    Ephemeris(#[from] EphemerisError),
    #[error(transparent)]
    Chart(#[from] ChartError),
    #[error("invalid time range: {0}")]
    InvalidRange(String),
    #[error("chart is missing {0}")]
    MissingPlanet(Planet),
    #[error("no exact {planet} return was found in the requested range")]
    ReturnNotFound { planet: Planet },
}

#[derive(Debug, Clone)]
pub struct TimingCalculator {
    provider: SwissEphemerisProvider,
    orb_policy: OrbPolicy,
}

impl TimingCalculator {
    #[must_use]
    pub fn new(provider: SwissEphemerisProvider) -> Self {
        Self {
            provider,
            orb_policy: OrbPolicy::default(),
        }
    }

    #[must_use]
    pub fn from_chart_calculator(calculator: &ChartCalculator) -> Self {
        Self::new(calculator.provider().clone())
    }

    #[must_use]
    pub fn with_orb_policy(mut self, policy: OrbPolicy) -> Self {
        self.orb_policy = policy;
        self
    }

    pub fn transits(
        &self,
        natal: &Chart,
        start_jd: f64,
        end_jd: f64,
    ) -> Result<Vec<TransitEvent>, TimingError> {
        transits::search(&self.provider, natal, start_jd, end_jd, &self.orb_policy)
    }

    pub fn return_jd(
        &self,
        natal: &Chart,
        planet: Planet,
        start_jd: f64,
        end_jd: f64,
    ) -> Result<f64, TimingError> {
        let target = natal
            .planet(planet)
            .ok_or(TimingError::MissingPlanet(planet))?
            .longitude;
        transits::find_longitude_crossing(&self.provider, planet, target, start_jd, end_jd)?
            .ok_or(TimingError::ReturnNotFound { planet })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn return_chart(
        &self,
        calculator: &ChartCalculator,
        natal: &Chart,
        planet: Planet,
        start_jd: f64,
        end_jd: f64,
        title: String,
        location_name: String,
        coordinates: Coordinates,
        house_system: TraditionalHouseSystem,
    ) -> Result<Chart, TimingError> {
        let jd_ut = self.return_jd(natal, planet, start_jd, end_jd)?;
        Ok(techniques::chart_at_jd(
            calculator,
            jd_ut,
            title,
            location_name,
            coordinates,
            house_system,
        )?)
    }

    pub fn secondary_progressions(
        &self,
        natal: &Chart,
        target_jd: f64,
    ) -> Result<TechniqueChart, TimingError> {
        techniques::secondary_progressions(&self.provider, natal, target_jd, &self.orb_policy)
    }

    pub fn solar_arc(&self, natal: &Chart, target_jd: f64) -> Result<TechniqueChart, TimingError> {
        techniques::solar_arc(&self.provider, natal, target_jd, &self.orb_policy)
    }

    pub fn harmonic(&self, natal: &Chart, harmonic: u16) -> Result<TechniqueChart, TimingError> {
        techniques::harmonic(natal, harmonic, &self.orb_policy)
    }

    pub fn planetary_hours(
        &self,
        day_start_jd: f64,
        coordinates: Coordinates,
    ) -> Result<PlanetaryHours, TimingError> {
        hours::calculate(&self.provider, day_start_jd, coordinates)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualProfection {
    pub age: u32,
    pub activated_house: u8,
    pub activated_sign: super::ZodiacSign,
    pub lord_of_year: Planet,
}

impl AnnualProfection {
    #[must_use]
    pub fn at_age(natal: &Chart, age: u32) -> Self {
        let natal_ascendant_sign = super::ZodiacSign::from_longitude(natal.houses.ascendant);
        let activated_sign =
            super::ZodiacSign::ALL[((u32::from(natal_ascendant_sign.index()) + age) % 12) as usize];
        Self {
            age,
            activated_house: (age % 12) as u8 + 1,
            activated_sign,
            lord_of_year: activated_sign.ruler(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirdariaPeriod {
    pub sect: super::Sect,
    pub age_years: f64,
    pub major_lord: Planet,
    pub sub_lord: Planet,
    pub major_started_at_age: f64,
    pub major_ends_at_age: f64,
    pub sub_started_at_age: f64,
    pub sub_ends_at_age: f64,
}

impl FirdariaPeriod {
    #[must_use]
    pub fn at_age(sect: super::Sect, age_years: f64) -> Self {
        techniques::firdaria(sect, age_years)
    }
}
