use serde::{Deserialize, Serialize};

use super::types::{
    Element, Planet, PlanetPosition, Sect, ZodiacSign, angular_distance, normalize_degrees,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriplicityRole {
    Day,
    Night,
    Participating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EssentialDignity {
    Domicile,
    Exaltation,
    Triplicity { role: TriplicityRole },
    Term,
    Face,
    Detriment,
    Fall,
}

impl EssentialDignity {
    #[must_use]
    pub const fn score(self) -> i16 {
        match self {
            Self::Domicile => 5,
            Self::Exaltation => 4,
            Self::Triplicity {
                role: TriplicityRole::Day | TriplicityRole::Night,
            } => 3,
            Self::Triplicity {
                role: TriplicityRole::Participating,
            }
            | Self::Face => 1,
            Self::Term => 2,
            Self::Detriment => -5,
            Self::Fall => -4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Combustion {
    Cazimi,
    Combust,
    UnderBeams,
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccidentalDignity {
    Angular,
    Succedent,
    Cadent,
    PlanetaryJoy,
    InSect,
    OutOfSect,
    Direct,
    Retrograde,
    Swift,
    Slow,
    Cazimi,
    Combust,
    UnderBeams,
}

impl AccidentalDignity {
    #[must_use]
    pub const fn score(self) -> i16 {
        match self {
            Self::Angular | Self::Cazimi => 5,
            Self::Succedent => 3,
            Self::Cadent | Self::Direct => 1,
            Self::PlanetaryJoy | Self::InSect | Self::Swift => 2,
            Self::OutOfSect | Self::Slow => -2,
            Self::Retrograde => -5,
            Self::Combust => -6,
            Self::UnderBeams => -4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetCondition {
    pub planet: Planet,
    pub essential: Vec<EssentialDignity>,
    pub accidental: Vec<AccidentalDignity>,
    pub combustion: Combustion,
    pub essential_score: i16,
    pub accidental_score: i16,
    pub total_score: i16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Almuten {
    pub winners: Vec<Planet>,
    pub scores: Vec<(Planet, i16)>,
}

pub(crate) fn calculate_conditions(
    positions: &[PlanetPosition],
    sect: Sect,
) -> Vec<PlanetCondition> {
    let sun_longitude = positions
        .iter()
        .find(|position| position.planet == Planet::Sun)
        .map_or(0.0, |position| position.longitude);
    positions
        .iter()
        .map(|position| condition(position, positions, sect, sun_longitude))
        .collect()
}

pub(crate) fn calculate_almuten(sect: Sect, longitudes: &[f64]) -> Almuten {
    let mut scores = Planet::ALL.map(|planet| (planet, 0_i16));
    for longitude in longitudes {
        let sign = ZodiacSign::from_longitude(*longitude);
        let degree = normalize_degrees(*longitude).rem_euclid(30.0);
        for (planet, score) in &mut scores {
            *score += positive_essential_score(*planet, sign, degree, sect);
        }
    }
    let maximum = scores.iter().map(|(_, score)| *score).max().unwrap_or(0);
    let winners = scores
        .iter()
        .filter(|(_, score)| *score == maximum)
        .map(|(planet, _)| *planet)
        .collect();
    Almuten {
        winners,
        scores: scores.to_vec(),
    }
}

fn condition(
    position: &PlanetPosition,
    positions: &[PlanetPosition],
    sect: Sect,
    sun_longitude: f64,
) -> PlanetCondition {
    let essential = essential_dignities(
        position.planet,
        position.sign,
        position.degree_in_sign,
        sect,
    );
    let combustion = combustion(position.planet, position.longitude, sun_longitude);
    let mut accidental = Vec::with_capacity(6);
    accidental.push(house_dignity(position.house));
    if position.house == position.planet.joy_house() {
        accidental.push(AccidentalDignity::PlanetaryJoy);
    }
    accidental.push(if is_in_sect(position.planet, sect, positions) {
        AccidentalDignity::InSect
    } else {
        AccidentalDignity::OutOfSect
    });
    accidental.push(if position.retrograde {
        AccidentalDignity::Retrograde
    } else {
        AccidentalDignity::Direct
    });
    accidental.push(
        if position.speed_longitude.abs() >= position.planet.average_daily_motion() {
            AccidentalDignity::Swift
        } else {
            AccidentalDignity::Slow
        },
    );
    match combustion {
        Combustion::Cazimi => accidental.push(AccidentalDignity::Cazimi),
        Combustion::Combust => accidental.push(AccidentalDignity::Combust),
        Combustion::UnderBeams => accidental.push(AccidentalDignity::UnderBeams),
        Combustion::Free => {}
    }
    let essential_score = essential.iter().map(|dignity| dignity.score()).sum();
    let accidental_score = accidental.iter().map(|dignity| dignity.score()).sum();
    PlanetCondition {
        planet: position.planet,
        essential,
        accidental,
        combustion,
        essential_score,
        accidental_score,
        total_score: essential_score + accidental_score,
    }
}

fn essential_dignities(
    planet: Planet,
    sign: ZodiacSign,
    degree: f64,
    sect: Sect,
) -> Vec<EssentialDignity> {
    let mut dignities = Vec::with_capacity(5);
    if sign.ruler() == planet {
        dignities.push(EssentialDignity::Domicile);
    }
    if exaltation_ruler(sign) == Some(planet) {
        dignities.push(EssentialDignity::Exaltation);
    }
    let (day, night, participating) = triplicity_rulers(sign.element());
    let role = if planet == day {
        Some(TriplicityRole::Day)
    } else if planet == night {
        Some(TriplicityRole::Night)
    } else if planet == participating {
        Some(TriplicityRole::Participating)
    } else {
        None
    };
    if let Some(role) = role {
        let is_active = matches!(
            (sect, role),
            (Sect::Day, TriplicityRole::Day)
                | (Sect::Night, TriplicityRole::Night)
                | (_, TriplicityRole::Participating)
        );
        if is_active {
            dignities.push(EssentialDignity::Triplicity { role });
        }
    }
    if term_ruler(sign, degree) == planet {
        dignities.push(EssentialDignity::Term);
    }
    if face_ruler(sign, degree) == planet {
        dignities.push(EssentialDignity::Face);
    }
    if sign.opposite().ruler() == planet {
        dignities.push(EssentialDignity::Detriment);
    }
    if exaltation_ruler(sign.opposite()) == Some(planet) {
        dignities.push(EssentialDignity::Fall);
    }
    dignities
}

fn positive_essential_score(planet: Planet, sign: ZodiacSign, degree: f64, sect: Sect) -> i16 {
    essential_dignities(planet, sign, degree, sect)
        .into_iter()
        .map(EssentialDignity::score)
        .filter(|score| *score > 0)
        .sum()
}

const fn exaltation_ruler(sign: ZodiacSign) -> Option<Planet> {
    match sign {
        ZodiacSign::Aries => Some(Planet::Sun),
        ZodiacSign::Taurus => Some(Planet::Moon),
        ZodiacSign::Cancer => Some(Planet::Jupiter),
        ZodiacSign::Virgo => Some(Planet::Mercury),
        ZodiacSign::Libra => Some(Planet::Saturn),
        ZodiacSign::Capricorn => Some(Planet::Mars),
        ZodiacSign::Pisces => Some(Planet::Venus),
        ZodiacSign::Gemini
        | ZodiacSign::Leo
        | ZodiacSign::Scorpio
        | ZodiacSign::Sagittarius
        | ZodiacSign::Aquarius => None,
    }
}

const fn triplicity_rulers(element: Element) -> (Planet, Planet, Planet) {
    match element {
        Element::Fire => (Planet::Sun, Planet::Jupiter, Planet::Saturn),
        Element::Earth => (Planet::Venus, Planet::Moon, Planet::Mars),
        Element::Air => (Planet::Saturn, Planet::Mercury, Planet::Jupiter),
        Element::Water => (Planet::Venus, Planet::Mars, Planet::Moon),
    }
}

fn term_ruler(sign: ZodiacSign, degree: f64) -> Planet {
    let terms = match sign {
        ZodiacSign::Aries => [
            (6, Planet::Jupiter),
            (14, Planet::Venus),
            (21, Planet::Mercury),
            (26, Planet::Mars),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Taurus => [
            (8, Planet::Venus),
            (14, Planet::Mercury),
            (22, Planet::Jupiter),
            (27, Planet::Saturn),
            (30, Planet::Mars),
        ],
        ZodiacSign::Gemini => [
            (6, Planet::Mercury),
            (12, Planet::Jupiter),
            (17, Planet::Venus),
            (24, Planet::Mars),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Cancer => [
            (7, Planet::Mars),
            (13, Planet::Venus),
            (19, Planet::Mercury),
            (26, Planet::Jupiter),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Leo => [
            (6, Planet::Jupiter),
            (11, Planet::Venus),
            (18, Planet::Saturn),
            (24, Planet::Mercury),
            (30, Planet::Mars),
        ],
        ZodiacSign::Virgo => [
            (7, Planet::Mercury),
            (17, Planet::Venus),
            (21, Planet::Jupiter),
            (28, Planet::Mars),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Libra => [
            (6, Planet::Saturn),
            (14, Planet::Mercury),
            (21, Planet::Jupiter),
            (28, Planet::Venus),
            (30, Planet::Mars),
        ],
        ZodiacSign::Scorpio => [
            (7, Planet::Mars),
            (11, Planet::Venus),
            (19, Planet::Mercury),
            (24, Planet::Jupiter),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Sagittarius => [
            (12, Planet::Jupiter),
            (17, Planet::Venus),
            (21, Planet::Mercury),
            (26, Planet::Saturn),
            (30, Planet::Mars),
        ],
        ZodiacSign::Capricorn => [
            (7, Planet::Mercury),
            (14, Planet::Jupiter),
            (22, Planet::Venus),
            (26, Planet::Saturn),
            (30, Planet::Mars),
        ],
        ZodiacSign::Aquarius => [
            (7, Planet::Mercury),
            (13, Planet::Venus),
            (20, Planet::Jupiter),
            (25, Planet::Mars),
            (30, Planet::Saturn),
        ],
        ZodiacSign::Pisces => [
            (12, Planet::Venus),
            (16, Planet::Jupiter),
            (19, Planet::Mercury),
            (28, Planet::Mars),
            (30, Planet::Saturn),
        ],
    };
    terms
        .into_iter()
        .find(|(end, _)| degree < f64::from(*end))
        .map_or(Planet::Saturn, |(_, planet)| planet)
}

fn face_ruler(sign: ZodiacSign, degree: f64) -> Planet {
    const ORDER: [Planet; 7] = [
        Planet::Mars,
        Planet::Sun,
        Planet::Venus,
        Planet::Mercury,
        Planet::Moon,
        Planet::Saturn,
        Planet::Jupiter,
    ];
    let decan = (degree.clamp(0.0, 29.999_999) / 10.0).floor() as usize;
    ORDER[(usize::from(sign.index()) * 3 + decan) % ORDER.len()]
}

const fn house_dignity(house: u8) -> AccidentalDignity {
    match house {
        1 | 4 | 7 | 10 => AccidentalDignity::Angular,
        2 | 5 | 8 | 11 => AccidentalDignity::Succedent,
        _ => AccidentalDignity::Cadent,
    }
}

fn is_in_sect(planet: Planet, sect: Sect, positions: &[PlanetPosition]) -> bool {
    match planet {
        Planet::Sun | Planet::Jupiter | Planet::Saturn => sect == Sect::Day,
        Planet::Moon | Planet::Venus | Planet::Mars => sect == Sect::Night,
        Planet::Mercury => {
            let sun = positions
                .iter()
                .find(|position| position.planet == Planet::Sun)
                .map_or(0.0, |position| position.longitude);
            let mercury = positions
                .iter()
                .find(|position| position.planet == Planet::Mercury)
                .map_or(0.0, |position| position.longitude);
            let oriental = normalize_degrees(mercury - sun) > 180.0;
            matches!((sect, oriental), (Sect::Day, true) | (Sect::Night, false))
        }
    }
}

fn combustion(planet: Planet, longitude: f64, sun_longitude: f64) -> Combustion {
    if planet == Planet::Sun {
        return Combustion::Free;
    }
    let distance = angular_distance(longitude, sun_longitude);
    if distance <= 17.0 / 60.0 {
        Combustion::Cazimi
    } else if distance <= 8.5 {
        Combustion::Combust
    } else if distance <= 17.0 {
        Combustion::UnderBeams
    } else {
        Combustion::Free
    }
}

#[cfg(test)]
mod tests {
    use super::{EssentialDignity, face_ruler, term_ruler};
    use crate::astro::types::{Planet, Sect, ZodiacSign};

    #[test]
    fn egyptian_terms_cover_sign_without_gaps() {
        assert_eq!(term_ruler(ZodiacSign::Aries, 0.0), Planet::Jupiter);
        assert_eq!(term_ruler(ZodiacSign::Aries, 29.999), Planet::Saturn);
        assert_eq!(term_ruler(ZodiacSign::Pisces, 18.0), Planet::Mercury);
    }

    #[test]
    fn chaldean_faces_start_with_mars() {
        assert_eq!(face_ruler(ZodiacSign::Aries, 0.0), Planet::Mars);
        assert_eq!(face_ruler(ZodiacSign::Aries, 10.0), Planet::Sun);
        assert_eq!(face_ruler(ZodiacSign::Taurus, 0.0), Planet::Mercury);
    }

    #[test]
    fn triplicity_score_distinguishes_participant() {
        assert_eq!(
            EssentialDignity::Triplicity {
                role: super::TriplicityRole::Day,
            }
            .score(),
            3
        );
        assert_eq!(
            EssentialDignity::Triplicity {
                role: super::TriplicityRole::Participating,
            }
            .score(),
            1
        );
        let _ = Sect::Day;
    }
}
