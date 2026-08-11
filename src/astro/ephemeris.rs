use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use swisseph::types::{EphemerisSource, HouseSystem};
use swisseph::{Body, CalcFlags, EclipseFlags, Ephemeris, EphemerisConfig, RiseSetFlags};
use thiserror::Error;

use super::types::{
    Coordinates, HouseCusps, Planet, PlanetPosition, TraditionalHouseSystem, ZodiacSign,
};

pub const ENGINE_VERSION: &str = "swisseph-rs/0.1.9";
pub const DATA_REVISION: &str = "3fd0f956d73898b91cc4f67cf18b21af656d1342";

#[derive(Clone)]
pub struct SwissEphemerisProvider {
    engine: Arc<Ephemeris>,
    ephemeris_path: PathBuf,
}

impl fmt::Debug for SwissEphemerisProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SwissEphemerisProvider")
            .field("ephemeris_path", &self.ephemeris_path)
            .field("engine_version", &ENGINE_VERSION)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
pub enum EphemerisError {
    #[error("ephemeris directory does not exist: {0}")]
    MissingDirectory(PathBuf),
    #[error("invalid Julian day: {0}")]
    InvalidJulianDay(f64),
    #[error("Swiss Ephemeris calculation failed for {planet}: {source}")]
    Calculation {
        planet: Planet,
        #[source]
        source: swisseph::Error,
    },
    #[error(
        "high-precision ephemeris data is missing for {planet} at JD {julian_day}; install the matching .se1 range in {path}"
    )]
    PrecisionFallback {
        planet: Planet,
        julian_day: f64,
        path: PathBuf,
    },
    #[error("house calculation failed: {0}")]
    Houses(#[source] swisseph::Error),
    #[error("failed to initialize the ephemeris: {0}")]
    Initialization(#[source] swisseph::Error),
}

impl SwissEphemerisProvider {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, EphemerisError> {
        let ephemeris_path = path.as_ref().to_path_buf();
        if !ephemeris_path.is_dir() {
            return Err(EphemerisError::MissingDirectory(ephemeris_path));
        }
        let config = EphemerisConfig {
            ephemeris_source: EphemerisSource::Swiss,
            ephe_path: Some(ephemeris_path.clone()),
            ..EphemerisConfig::default()
        };
        let engine = Ephemeris::new(config).map_err(EphemerisError::Initialization)?;
        Ok(Self {
            engine: Arc::new(engine),
            ephemeris_path,
        })
    }

    #[must_use]
    pub fn ephemeris_path(&self) -> &Path {
        &self.ephemeris_path
    }

    pub fn positions_and_houses(
        &self,
        jd_ut: f64,
        coordinates: Coordinates,
        house_system: TraditionalHouseSystem,
    ) -> Result<(Vec<PlanetPosition>, HouseCusps), EphemerisError> {
        if !jd_ut.is_finite() {
            return Err(EphemerisError::InvalidJulianDay(jd_ut));
        }
        let houses = self.houses(jd_ut, coordinates, house_system)?;
        let positions = Planet::ALL
            .into_iter()
            .map(|planet| self.position(jd_ut, planet, &houses))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((positions, houses))
    }

    pub fn ecliptic_position(
        &self,
        jd_ut: f64,
        planet: Planet,
    ) -> Result<[f64; 6], EphemerisError> {
        let result = self
            .engine
            .calc_ut(
                jd_ut,
                body_for(planet),
                CalcFlags::SWIEPH | CalcFlags::SPEED,
            )
            .map_err(|source| EphemerisError::Calculation { planet, source })?;
        self.require_swiss_backend(planet, jd_ut, result.flags_used)?;
        Ok(result.data)
    }

    pub fn equatorial_position(
        &self,
        jd_ut: f64,
        planet: Planet,
    ) -> Result<[f64; 6], EphemerisError> {
        let result = self
            .engine
            .calc_ut(
                jd_ut,
                body_for(planet),
                CalcFlags::SWIEPH | CalcFlags::SPEED | CalcFlags::EQUATORIAL,
            )
            .map_err(|source| EphemerisError::Calculation { planet, source })?;
        self.require_swiss_backend(planet, jd_ut, result.flags_used)?;
        Ok(result.data)
    }

    pub fn sunrise_sunset(
        &self,
        search_start_jd: f64,
        coordinates: Coordinates,
    ) -> Result<(f64, f64), EphemerisError> {
        self.ecliptic_position(search_start_jd, Planet::Sun)?;
        let geopos = [
            coordinates.longitude,
            coordinates.latitude,
            coordinates.elevation_m,
        ];
        let rise = self
            .engine
            .rise_trans(
                search_start_jd,
                Body::Sun,
                None,
                CalcFlags::SWIEPH,
                RiseSetFlags::RISE,
                geopos,
                0.0,
                15.0,
            )
            .map_err(|source| EphemerisError::Calculation {
                planet: Planet::Sun,
                source,
            })?;
        let set = self
            .engine
            .rise_trans(
                rise.time + 1.0 / 1440.0,
                Body::Sun,
                None,
                CalcFlags::SWIEPH,
                RiseSetFlags::SET,
                geopos,
                0.0,
                15.0,
            )
            .map_err(|source| EphemerisError::Calculation {
                planet: Planet::Sun,
                source,
            })?;
        Ok((rise.time, set.time))
    }

    pub(crate) fn next_solar_eclipse(
        &self,
        search_start_jd: f64,
    ) -> Result<swisseph::SolarEclipseGlobal, EphemerisError> {
        self.ecliptic_position(search_start_jd, Planet::Sun)?;
        self.engine
            .sol_eclipse_when_glob(
                search_start_jd,
                CalcFlags::SWIEPH,
                EclipseFlags::empty(),
                false,
            )
            .map_err(|source| EphemerisError::Calculation {
                planet: Planet::Sun,
                source,
            })
    }

    pub(crate) fn next_lunar_eclipse(
        &self,
        search_start_jd: f64,
    ) -> Result<swisseph::LunarEclipseGlobal, EphemerisError> {
        self.ecliptic_position(search_start_jd, Planet::Moon)?;
        self.engine
            .lun_eclipse_when(
                search_start_jd,
                CalcFlags::SWIEPH,
                EclipseFlags::empty(),
                false,
            )
            .map_err(|source| EphemerisError::Calculation {
                planet: Planet::Moon,
                source,
            })
    }

    fn position(
        &self,
        jd_ut: f64,
        planet: Planet,
        houses: &HouseCusps,
    ) -> Result<PlanetPosition, EphemerisError> {
        let ecliptic = self.ecliptic_position(jd_ut, planet)?;
        let equatorial = self.equatorial_position(jd_ut, planet)?;

        let longitude = ecliptic[0].rem_euclid(360.0);
        let sign = ZodiacSign::from_longitude(longitude);
        Ok(PlanetPosition {
            planet,
            longitude,
            latitude: ecliptic[1],
            distance_au: ecliptic[2],
            speed_longitude: ecliptic[3],
            right_ascension: equatorial[0].rem_euclid(360.0),
            declination: equatorial[1],
            house: houses.house_of(longitude),
            sign,
            degree_in_sign: longitude.rem_euclid(30.0),
            retrograde: ecliptic[3] < 0.0,
        })
    }

    fn houses(
        &self,
        jd_ut: f64,
        coordinates: Coordinates,
        house_system: TraditionalHouseSystem,
    ) -> Result<HouseCusps, EphemerisError> {
        let result = self
            .engine
            .houses(
                jd_ut,
                coordinates.latitude,
                coordinates.longitude,
                swiss_house_system(house_system),
            )
            .map_err(EphemerisError::Houses)?;
        let mut cusps = [0.0; 12];
        for (destination, source) in cusps.iter_mut().zip(result.cusps.iter().skip(1).take(12)) {
            *destination = source.rem_euclid(360.0);
        }
        Ok(HouseCusps {
            system: house_system,
            cusps,
            ascendant: result.ascmc.ascendant.rem_euclid(360.0),
            midheaven: result.ascmc.mc.rem_euclid(360.0),
            armc: result.ascmc.armc.rem_euclid(360.0),
            vertex: result.ascmc.vertex.rem_euclid(360.0),
        })
    }

    fn require_swiss_backend(
        &self,
        planet: Planet,
        jd_ut: f64,
        flags_used: CalcFlags,
    ) -> Result<(), EphemerisError> {
        if flags_used.contains(CalcFlags::SWIEPH) && !flags_used.contains(CalcFlags::MOSEPH) {
            Ok(())
        } else {
            Err(EphemerisError::PrecisionFallback {
                planet,
                julian_day: jd_ut,
                path: self.ephemeris_path.clone(),
            })
        }
    }
}

const fn body_for(planet: Planet) -> Body {
    match planet {
        Planet::Sun => Body::Sun,
        Planet::Moon => Body::Moon,
        Planet::Mercury => Body::Mercury,
        Planet::Venus => Body::Venus,
        Planet::Mars => Body::Mars,
        Planet::Jupiter => Body::Jupiter,
        Planet::Saturn => Body::Saturn,
    }
}

const fn swiss_house_system(system: TraditionalHouseSystem) -> HouseSystem {
    match system {
        TraditionalHouseSystem::WholeSign => HouseSystem::WholeSign,
        TraditionalHouseSystem::Equal => HouseSystem::Equal,
        TraditionalHouseSystem::Porphyry => HouseSystem::Porphyry,
        TraditionalHouseSystem::Alcabitius => HouseSystem::Alcabitius,
        TraditionalHouseSystem::Placidus => HouseSystem::Placidus,
        TraditionalHouseSystem::Regiomontanus => HouseSystem::Regiomontanus,
        TraditionalHouseSystem::Campanus => HouseSystem::Campanus,
        TraditionalHouseSystem::Morinus => HouseSystem::Morinus,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{EphemerisError, SwissEphemerisProvider};
    use crate::astro::types::{Coordinates, Planet, TraditionalHouseSystem};

    #[test]
    fn j2000_uses_swiss_coefficients() -> Result<(), EphemerisError> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let sun = provider.ecliptic_position(2_451_545.0, Planet::Sun)?;
        assert!((sun[0] - 280.3689).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn creates_only_septenary_positions() -> Result<(), EphemerisError> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let (positions, houses) = provider.positions_and_houses(
            2_451_545.0,
            Coordinates {
                latitude: 45.4642,
                longitude: 9.19,
                elevation_m: 120.0,
            },
            TraditionalHouseSystem::WholeSign,
        )?;
        assert_eq!(positions.len(), 7);
        assert_eq!(positions[0].planet, Planet::Sun);
        assert!((0.0..360.0).contains(&houses.ascendant));
        Ok(())
    }

    #[test]
    fn missing_coefficients_never_fall_back_to_moshier() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let result = SwissEphemerisProvider::new(directory.path());
        assert!(matches!(result, Err(EphemerisError::Initialization(_))));
        Ok(())
    }

    #[test]
    fn uncovered_date_rejects_analytical_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        std::fs::copy(
            "data/ephe/sepl_18.se1",
            directory.path().join("sepl_18.se1"),
        )?;
        std::fs::copy(
            "data/ephe/semo_18.se1",
            directory.path().join("semo_18.se1"),
        )?;
        let provider = SwissEphemerisProvider::new(directory.path())?;
        let result = provider.ecliptic_position(2_086_302.5, Planet::Sun);
        assert!(matches!(
            result,
            Err(EphemerisError::PrecisionFallback { .. })
        ));
        Ok(())
    }
}
