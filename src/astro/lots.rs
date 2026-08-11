use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::{HouseCusps, Planet, PlanetPosition, Sect, ZodiacSign, normalize_degrees};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LotKind {
    Fortune,
    Spirit,
    Eros,
    Necessity,
    Courage,
    Victory,
    Nemesis,
}

impl LotKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fortune => "Lot of Fortune",
            Self::Spirit => "Lot of Spirit",
            Self::Eros => "Lot of Eros",
            Self::Necessity => "Lot of Necessity",
            Self::Courage => "Lot of Courage",
            Self::Victory => "Lot of Victory",
            Self::Nemesis => "Lot of Nemesis",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lot {
    pub kind: LotKind,
    pub longitude: f64,
    pub sign: ZodiacSign,
    pub degree_in_sign: f64,
    pub house: u8,
    pub ruler: Planet,
    pub antiscia: f64,
    pub contra_antiscia: f64,
    pub dodecatemoria: f64,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LotError {
    #[error("chart is missing {0}")]
    MissingPlanet(Planet),
}

pub(crate) fn calculate_lots(
    ascendant: f64,
    sect: Sect,
    positions: &[PlanetPosition],
    houses: &HouseCusps,
) -> Result<Vec<Lot>, LotError> {
    let sun = planet_longitude(positions, Planet::Sun)?;
    let moon = planet_longitude(positions, Planet::Moon)?;
    let mercury = planet_longitude(positions, Planet::Mercury)?;
    let venus = planet_longitude(positions, Planet::Venus)?;
    let mars = planet_longitude(positions, Planet::Mars)?;
    let jupiter = planet_longitude(positions, Planet::Jupiter)?;
    let saturn = planet_longitude(positions, Planet::Saturn)?;

    let fortune = sect_formula(sect, ascendant, moon, sun);
    let spirit = sect_formula(sect, ascendant, sun, moon);
    let eros = sect_formula(sect, ascendant, venus, spirit);
    let necessity = sect_formula(sect, ascendant, fortune, mercury);
    let courage = sect_formula(sect, ascendant, fortune, mars);
    let victory = sect_formula(sect, ascendant, jupiter, spirit);
    let nemesis = sect_formula(sect, ascendant, fortune, saturn);

    Ok([
        (LotKind::Fortune, fortune),
        (LotKind::Spirit, spirit),
        (LotKind::Eros, eros),
        (LotKind::Necessity, necessity),
        (LotKind::Courage, courage),
        (LotKind::Victory, victory),
        (LotKind::Nemesis, nemesis),
    ]
    .into_iter()
    .map(|(kind, longitude)| build_lot(kind, longitude, houses))
    .collect())
}

fn planet_longitude(positions: &[PlanetPosition], planet: Planet) -> Result<f64, LotError> {
    positions
        .iter()
        .find(|position| position.planet == planet)
        .map(|position| position.longitude)
        .ok_or(LotError::MissingPlanet(planet))
}

fn sect_formula(sect: Sect, ascendant: f64, day_add: f64, day_subtract: f64) -> f64 {
    match sect {
        Sect::Day => normalize_degrees(ascendant + day_add - day_subtract),
        Sect::Night => normalize_degrees(ascendant + day_subtract - day_add),
    }
}

fn build_lot(kind: LotKind, longitude: f64, houses: &HouseCusps) -> Lot {
    let sign = ZodiacSign::from_longitude(longitude);
    Lot {
        kind,
        longitude,
        sign,
        degree_in_sign: longitude.rem_euclid(30.0),
        house: houses.house_of(longitude),
        ruler: sign.ruler(),
        antiscia: antiscia(longitude),
        contra_antiscia: contra_antiscia(longitude),
        dodecatemoria: dodecatemoria(longitude),
    }
}

#[must_use]
pub fn antiscia(longitude: f64) -> f64 {
    normalize_degrees(180.0 - longitude)
}

#[must_use]
pub fn contra_antiscia(longitude: f64) -> f64 {
    normalize_degrees(antiscia(longitude) + 180.0)
}

#[must_use]
pub fn dodecatemoria(longitude: f64) -> f64 {
    let normalized = normalize_degrees(longitude);
    let sign_start = (normalized / 30.0).floor() * 30.0;
    normalize_degrees(sign_start + normalized.rem_euclid(30.0) * 12.0)
}

#[cfg(test)]
mod tests {
    use super::{antiscia, contra_antiscia, dodecatemoria};

    #[test]
    fn derived_points_follow_classical_mirrors() {
        assert!((antiscia(10.0) - 170.0).abs() < f64::EPSILON);
        assert!((contra_antiscia(10.0) - 350.0).abs() < f64::EPSILON);
        assert!((dodecatemoria(2.5) - 30.0).abs() < f64::EPSILON);
    }
}
