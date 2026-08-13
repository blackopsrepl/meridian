use serde::{Deserialize, Serialize};

use super::types::{Planet, PointId, angular_distance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectKind {
    Conjunction,
    Sextile,
    Square,
    Trine,
    Opposition,
}

impl AspectKind {
    pub const ALL: [Self; 5] = [
        Self::Conjunction,
        Self::Sextile,
        Self::Square,
        Self::Trine,
        Self::Opposition,
    ];

    #[must_use]
    pub const fn angle(self) -> f64 {
        match self {
            Self::Conjunction => 0.0,
            Self::Sextile => 60.0,
            Self::Square => 90.0,
            Self::Trine => 120.0,
            Self::Opposition => 180.0,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Conjunction => "Conjunction",
            Self::Sextile => "Sextile",
            Self::Square => "Square",
            Self::Trine => "Trine",
            Self::Opposition => "Opposition",
        }
    }

    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Conjunction => "☌",
            Self::Sextile => "⚹",
            Self::Square => "□",
            Self::Trine => "△",
            Self::Opposition => "☍",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectPhase {
    Applying,
    Separating,
    Exact,
    Static,
}

impl AspectPhase {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Applying => "Applying",
            Self::Separating => "Separating",
            Self::Exact => "Exact",
            Self::Static => "Static",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AspectHit {
    pub left: PointId,
    pub right: PointId,
    pub kind: AspectKind,
    pub orb: f64,
    pub phase: AspectPhase,
    pub partile: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbPolicy {
    pub conjunction: f64,
    pub sextile: f64,
    pub square: f64,
    pub trine: f64,
    pub opposition: f64,
    pub luminary_bonus: f64,
    pub angle_orb: f64,
    pub lot_orb: f64,
}

impl Default for OrbPolicy {
    fn default() -> Self {
        Self {
            conjunction: 8.0,
            sextile: 5.0,
            square: 7.0,
            trine: 7.0,
            opposition: 8.0,
            luminary_bonus: 2.0,
            angle_orb: 5.0,
            lot_orb: 3.0,
        }
    }
}

impl OrbPolicy {
    fn base_orb(&self, kind: AspectKind) -> f64 {
        match kind {
            AspectKind::Conjunction => self.conjunction,
            AspectKind::Sextile => self.sextile,
            AspectKind::Square => self.square,
            AspectKind::Trine => self.trine,
            AspectKind::Opposition => self.opposition,
        }
    }

    pub(crate) fn allowed_orb(&self, kind: AspectKind, left: PointId, right: PointId) -> f64 {
        let mut allowed = self.base_orb(kind);
        if is_luminary(left) || is_luminary(right) {
            allowed += self.luminary_bonus;
        }
        if is_angle(left) || is_angle(right) {
            allowed = allowed.min(self.angle_orb);
        }
        if is_lot(left) || is_lot(right) {
            allowed = allowed.min(self.lot_orb);
        }
        allowed
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AspectPoint {
    pub id: PointId,
    pub longitude: f64,
    pub speed: Option<f64>,
}

pub(crate) fn find_aspects(points: &[AspectPoint], policy: &OrbPolicy) -> Vec<AspectHit> {
    let mut hits = Vec::new();
    for left_index in 0..points.len() {
        for right_index in (left_index + 1)..points.len() {
            let left = points[left_index];
            let right = points[right_index];
            if !matches!(left.id, PointId::Planet(_)) && !matches!(right.id, PointId::Planet(_)) {
                continue;
            }
            if let Some(hit) = closest_aspect(left, right, policy) {
                hits.push(hit);
            }
        }
    }
    hits.sort_by(|left, right| {
        left.orb
            .total_cmp(&right.orb)
            .then_with(|| left.kind.angle().total_cmp(&right.kind.angle()))
    });
    hits
}

fn closest_aspect(left: AspectPoint, right: AspectPoint, policy: &OrbPolicy) -> Option<AspectHit> {
    let best = identify_aspect(left.id, left.longitude, right.id, right.longitude, policy);
    best.map(|(kind, orb)| AspectHit {
        left: left.id,
        right: right.id,
        kind,
        orb,
        phase: phase(left, right, kind, orb),
        partile: orb < 1.0,
    })
}

pub(crate) fn identify_aspect(
    left: PointId,
    left_longitude: f64,
    right: PointId,
    right_longitude: f64,
    policy: &OrbPolicy,
) -> Option<(AspectKind, f64)> {
    let separation = angular_distance(left_longitude, right_longitude);
    let mut best: Option<(AspectKind, f64)> = None;
    for kind in AspectKind::ALL {
        let orb = (separation - kind.angle()).abs();
        if orb <= policy.allowed_orb(kind, left, right)
            && best.is_none_or(|(_, best_orb)| orb < best_orb)
        {
            best = Some((kind, orb));
        }
    }
    best
}

fn phase(left: AspectPoint, right: AspectPoint, kind: AspectKind, current_orb: f64) -> AspectPhase {
    if current_orb <= 1e-6 {
        return AspectPhase::Exact;
    }
    let (Some(left_speed), Some(right_speed)) = (left.speed, right.speed) else {
        return AspectPhase::Static;
    };
    let relative_speed = right_speed - left_speed;
    if relative_speed.abs() <= f64::EPSILON {
        return AspectPhase::Static;
    }

    let error = oriented_aspect_error(right.longitude - left.longitude, kind);
    let orb_trend = error * relative_speed;
    if orb_trend < -1e-12 {
        AspectPhase::Applying
    } else if orb_trend > 1e-12 {
        AspectPhase::Separating
    } else {
        AspectPhase::Static
    }
}

fn oriented_aspect_error(separation: f64, kind: AspectKind) -> f64 {
    let angle = match kind {
        AspectKind::Conjunction => return signed_degrees(separation),
        AspectKind::Opposition => return signed_degrees(separation - 180.0),
        AspectKind::Sextile => 60.0,
        AspectKind::Square => 90.0,
        AspectKind::Trine => 120.0,
    };

    let positive = signed_degrees(separation - angle);
    let negative = signed_degrees(separation + angle);
    if positive.abs() <= negative.abs() {
        positive
    } else {
        negative
    }
}

fn signed_degrees(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}

const fn is_luminary(point: PointId) -> bool {
    matches!(point, PointId::Planet(Planet::Sun | Planet::Moon))
}

const fn is_angle(point: PointId) -> bool {
    matches!(point, PointId::Ascendant | PointId::Midheaven)
}

const fn is_lot(point: PointId) -> bool {
    matches!(point, PointId::LotFortune | PointId::LotSpirit)
}

#[cfg(test)]
mod tests {
    use super::{AspectKind, AspectPhase, AspectPoint, OrbPolicy, find_aspects, identify_aspect};
    use crate::astro::types::{Planet, PointId};

    #[test]
    fn wraparound_conjunction_is_found() {
        let hits = find_aspects(
            &[
                AspectPoint {
                    id: PointId::Planet(Planet::Sun),
                    longitude: 359.0,
                    speed: Some(1.0),
                },
                AspectPoint {
                    id: PointId::Planet(Planet::Moon),
                    longitude: 1.0,
                    speed: Some(13.0),
                },
            ],
            &OrbPolicy::default(),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, AspectKind::Conjunction);
        assert_eq!(hits[0].phase, AspectPhase::Separating);
    }

    #[test]
    fn modern_minor_aspects_are_not_emitted() {
        let hits = find_aspects(
            &[
                AspectPoint {
                    id: PointId::Planet(Planet::Venus),
                    longitude: 0.0,
                    speed: Some(1.0),
                },
                AspectPoint {
                    id: PointId::Planet(Planet::Mars),
                    longitude: 45.0,
                    speed: Some(0.5),
                },
            ],
            &OrbPolicy::default(),
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn all_ptolemaic_aspects_are_identified_across_the_zodiac_boundary() {
        let policy = OrbPolicy::default();
        for kind in AspectKind::ALL {
            let right = (350.0 + kind.angle()).rem_euclid(360.0);
            let identified = identify_aspect(
                PointId::Planet(Planet::Mercury),
                350.0,
                PointId::Planet(Planet::Saturn),
                right,
                &policy,
            );
            assert_eq!(identified, Some((kind, 0.0)), "failed for {kind:?}");
        }
    }

    #[test]
    fn aspect_boundaries_follow_point_specific_orbs() {
        let policy = OrbPolicy::default();
        assert!(
            identify_aspect(
                PointId::Planet(Planet::Sun),
                0.0,
                PointId::Planet(Planet::Saturn),
                189.9,
                &policy,
            )
            .is_some()
        );
        assert!(
            identify_aspect(
                PointId::Planet(Planet::Mercury),
                0.0,
                PointId::Ascendant,
                95.0,
                &policy,
            )
            .is_some()
        );
        assert!(
            identify_aspect(
                PointId::Planet(Planet::Mercury),
                0.0,
                PointId::LotFortune,
                93.0,
                &policy,
            )
            .is_some()
        );
        assert!(
            identify_aspect(
                PointId::Planet(Planet::Mercury),
                0.0,
                PointId::LotFortune,
                93.01,
                &policy,
            )
            .is_none()
        );
    }

    #[test]
    fn phase_does_not_overshoot_an_imminent_exact_aspect() {
        let hits = find_aspects(
            &[
                AspectPoint {
                    id: PointId::Planet(Planet::Saturn),
                    longitude: 0.0,
                    speed: Some(0.0),
                },
                AspectPoint {
                    id: PointId::Planet(Planet::Moon),
                    longitude: 59.95,
                    speed: Some(20.0),
                },
            ],
            &OrbPolicy::default(),
        );
        assert_eq!(hits[0].kind, AspectKind::Sextile);
        assert_eq!(hits[0].phase, AspectPhase::Applying);
    }

    #[test]
    fn phase_handles_both_aspect_directions_and_retrograde_motion() {
        let cases = [
            (59.0, 2.0, AspectPhase::Applying),
            (61.0, 2.0, AspectPhase::Separating),
            (301.0, -2.0, AspectPhase::Applying),
            (299.0, -2.0, AspectPhase::Separating),
        ];
        for (longitude, speed, expected) in cases {
            let hits = find_aspects(
                &[
                    AspectPoint {
                        id: PointId::Planet(Planet::Saturn),
                        longitude: 0.0,
                        speed: Some(0.0),
                    },
                    AspectPoint {
                        id: PointId::Planet(Planet::Mercury),
                        longitude,
                        speed: Some(speed),
                    },
                ],
                &OrbPolicy::default(),
            );
            assert_eq!(hits[0].phase, expected, "longitude {longitude}");
        }
    }

    #[test]
    fn point_to_point_pairs_are_not_reported_as_chart_aspects() {
        let hits = find_aspects(
            &[
                AspectPoint {
                    id: PointId::Ascendant,
                    longitude: 0.0,
                    speed: None,
                },
                AspectPoint {
                    id: PointId::Midheaven,
                    longitude: 90.0,
                    speed: None,
                },
            ],
            &OrbPolicy::default(),
        );
        assert!(hits.is_empty());
    }
}
