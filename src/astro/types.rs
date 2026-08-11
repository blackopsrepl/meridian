use std::fmt;

use serde::{Deserialize, Serialize};

/// The seven visible planets of the classical astrological system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Planet {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
}

impl Planet {
    pub const ALL: [Self; 7] = [
        Self::Sun,
        Self::Moon,
        Self::Mercury,
        Self::Venus,
        Self::Mars,
        Self::Jupiter,
        Self::Saturn,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sun => "Sun",
            Self::Moon => "Moon",
            Self::Mercury => "Mercury",
            Self::Venus => "Venus",
            Self::Mars => "Mars",
            Self::Jupiter => "Jupiter",
            Self::Saturn => "Saturn",
        }
    }

    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Sun => "☉",
            Self::Moon => "☽",
            Self::Mercury => "☿",
            Self::Venus => "♀",
            Self::Mars => "♂",
            Self::Jupiter => "♃",
            Self::Saturn => "♄",
        }
    }

    #[must_use]
    pub const fn is_luminary(self) -> bool {
        matches!(self, Self::Sun | Self::Moon)
    }

    #[must_use]
    pub const fn average_daily_motion(self) -> f64 {
        match self {
            Self::Sun => 0.9856,
            Self::Moon => 13.1764,
            Self::Mercury | Self::Venus => 1.20,
            Self::Mars => 0.524,
            Self::Jupiter => 0.0831,
            Self::Saturn => 0.0335,
        }
    }

    #[must_use]
    pub const fn joy_house(self) -> u8 {
        match self {
            Self::Mercury => 1,
            Self::Moon => 3,
            Self::Venus => 5,
            Self::Mars => 6,
            Self::Sun => 9,
            Self::Jupiter => 11,
            Self::Saturn => 12,
        }
    }
}

impl fmt::Display for Planet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZodiacSign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

impl ZodiacSign {
    pub const ALL: [Self; 12] = [
        Self::Aries,
        Self::Taurus,
        Self::Gemini,
        Self::Cancer,
        Self::Leo,
        Self::Virgo,
        Self::Libra,
        Self::Scorpio,
        Self::Sagittarius,
        Self::Capricorn,
        Self::Aquarius,
        Self::Pisces,
    ];

    #[must_use]
    pub fn from_longitude(longitude: f64) -> Self {
        let index = (normalize_degrees(longitude) / 30.0).floor() as usize;
        Self::ALL[index.min(11)]
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Aries => 0,
            Self::Taurus => 1,
            Self::Gemini => 2,
            Self::Cancer => 3,
            Self::Leo => 4,
            Self::Virgo => 5,
            Self::Libra => 6,
            Self::Scorpio => 7,
            Self::Sagittarius => 8,
            Self::Capricorn => 9,
            Self::Aquarius => 10,
            Self::Pisces => 11,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aries => "Aries",
            Self::Taurus => "Taurus",
            Self::Gemini => "Gemini",
            Self::Cancer => "Cancer",
            Self::Leo => "Leo",
            Self::Virgo => "Virgo",
            Self::Libra => "Libra",
            Self::Scorpio => "Scorpio",
            Self::Sagittarius => "Sagittarius",
            Self::Capricorn => "Capricorn",
            Self::Aquarius => "Aquarius",
            Self::Pisces => "Pisces",
        }
    }

    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Aries => "♈",
            Self::Taurus => "♉",
            Self::Gemini => "♊",
            Self::Cancer => "♋",
            Self::Leo => "♌",
            Self::Virgo => "♍",
            Self::Libra => "♎",
            Self::Scorpio => "♏",
            Self::Sagittarius => "♐",
            Self::Capricorn => "♑",
            Self::Aquarius => "♒",
            Self::Pisces => "♓",
        }
    }

    #[must_use]
    pub const fn ruler(self) -> Planet {
        match self {
            Self::Aries | Self::Scorpio => Planet::Mars,
            Self::Taurus | Self::Libra => Planet::Venus,
            Self::Gemini | Self::Virgo => Planet::Mercury,
            Self::Cancer => Planet::Moon,
            Self::Leo => Planet::Sun,
            Self::Sagittarius | Self::Pisces => Planet::Jupiter,
            Self::Capricorn | Self::Aquarius => Planet::Saturn,
        }
    }

    #[must_use]
    pub const fn element(self) -> Element {
        match self {
            Self::Aries | Self::Leo | Self::Sagittarius => Element::Fire,
            Self::Taurus | Self::Virgo | Self::Capricorn => Element::Earth,
            Self::Gemini | Self::Libra | Self::Aquarius => Element::Air,
            Self::Cancer | Self::Scorpio | Self::Pisces => Element::Water,
        }
    }

    #[must_use]
    pub const fn modality(self) -> Modality {
        match self {
            Self::Aries | Self::Cancer | Self::Libra | Self::Capricorn => Modality::Cardinal,
            Self::Taurus | Self::Leo | Self::Scorpio | Self::Aquarius => Modality::Fixed,
            Self::Gemini | Self::Virgo | Self::Sagittarius | Self::Pisces => Modality::Mutable,
        }
    }

    #[must_use]
    pub const fn opposite(self) -> Self {
        Self::ALL[((self.index() + 6) % 12) as usize]
    }
}

impl fmt::Display for ZodiacSign {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Earth,
    Air,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Cardinal,
    Fixed,
    Mutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sect {
    Day,
    Night,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraditionalHouseSystem {
    #[default]
    WholeSign,
    Equal,
    Porphyry,
    Alcabitius,
    Placidus,
    Regiomontanus,
    Campanus,
    Morinus,
}

impl TraditionalHouseSystem {
    pub const ALL: [Self; 8] = [
        Self::WholeSign,
        Self::Equal,
        Self::Porphyry,
        Self::Alcabitius,
        Self::Placidus,
        Self::Regiomontanus,
        Self::Campanus,
        Self::Morinus,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WholeSign => "Whole sign",
            Self::Equal => "Equal",
            Self::Porphyry => "Porphyry",
            Self::Alcabitius => "Alcabitius",
            Self::Placidus => "Placidus",
            Self::Regiomontanus => "Regiomontanus",
            Self::Campanus => "Campanus",
            Self::Morinus => "Morinus",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Calendar {
    #[default]
    Gregorian,
    Julian,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartPurpose {
    #[default]
    Natal,
    Event,
    Horary,
    Electional,
    Mundane,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub elevation_m: f64,
}

impl Coordinates {
    pub fn validate(self) -> Result<Self, &'static str> {
        if !self.latitude.is_finite() || !(-90.0..=90.0).contains(&self.latitude) {
            return Err("latitude must be between -90 and 90 degrees");
        }
        if !self.longitude.is_finite() || !(-180.0..=180.0).contains(&self.longitude) {
            return Err("longitude must be between -180 and 180 degrees");
        }
        if !self.elevation_m.is_finite() || !(-500.0..=10_000.0).contains(&self.elevation_m) {
            return Err("elevation must be between -500 and 10000 metres");
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CivilDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    #[serde(default)]
    pub second: f64,
    #[serde(default)]
    pub calendar: Calendar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimeZoneSpec {
    Iana {
        name: String,
        #[serde(default)]
        fold: Option<u8>,
    },
    FixedOffset {
        minutes_east: i32,
        #[serde(default)]
        label: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartRequest {
    pub title: String,
    #[serde(default)]
    pub purpose: ChartPurpose,
    pub local_time: CivilDateTime,
    pub time_zone: TimeZoneSpec,
    pub location_name: String,
    pub coordinates: Coordinates,
    #[serde(default)]
    pub house_system: TraditionalHouseSystem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetPosition {
    pub planet: Planet,
    pub longitude: f64,
    pub latitude: f64,
    pub distance_au: f64,
    pub speed_longitude: f64,
    pub right_ascension: f64,
    pub declination: f64,
    pub house: u8,
    pub sign: ZodiacSign,
    pub degree_in_sign: f64,
    pub retrograde: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseCusps {
    pub system: TraditionalHouseSystem,
    pub cusps: [f64; 12],
    pub ascendant: f64,
    pub midheaven: f64,
    pub armc: f64,
    pub vertex: f64,
}

impl HouseCusps {
    #[must_use]
    pub fn house_of(&self, longitude: f64) -> u8 {
        let target = normalize_degrees(longitude);
        for index in 0..12 {
            let start = normalize_degrees(self.cusps[index]);
            let end = normalize_degrees(self.cusps[(index + 1) % 12]);
            let span = normalize_degrees(end - start);
            let offset = normalize_degrees(target - start);
            if offset < span || (span == 0.0 && offset == 0.0) {
                return index as u8 + 1;
            }
        }
        12
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PointId {
    Planet(Planet),
    Ascendant,
    Midheaven,
    LotFortune,
    LotSpirit,
}

impl PointId {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Planet(planet) => planet.name(),
            Self::Ascendant => "Ascendant",
            Self::Midheaven => "Midheaven",
            Self::LotFortune => "Lot of Fortune",
            Self::LotSpirit => "Lot of Spirit",
        }
    }
}

#[must_use]
pub fn normalize_degrees(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

#[must_use]
pub fn angular_distance(left: f64, right: f64) -> f64 {
    let difference = (normalize_degrees(left) - normalize_degrees(right)).abs();
    difference.min(360.0 - difference)
}
