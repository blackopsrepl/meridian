use crate::astro::{LotKind, Planet, PointId, ZodiacSign};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inspection {
    Planet(Planet),
    Sign(ZodiacSign),
    House(u8),
    Aspect(usize),
    Ascendant,
    Midheaven,
    Lot(LotKind),
}

impl Inspection {
    pub(super) fn point(self) -> Option<PointId> {
        match self {
            Self::Planet(planet) => Some(PointId::Planet(planet)),
            Self::Ascendant => Some(PointId::Ascendant),
            Self::Midheaven => Some(PointId::Midheaven),
            Self::Lot(LotKind::Fortune) => Some(PointId::LotFortune),
            Self::Lot(LotKind::Spirit) => Some(PointId::LotSpirit),
            Self::Sign(_) | Self::House(_) | Self::Aspect(_) | Self::Lot(_) => None,
        }
    }
}
