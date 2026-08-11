use serde::{Deserialize, Serialize};

use super::TimingError;
use crate::astro::aspects::{AspectKind, OrbPolicy};
use crate::astro::chart::Chart;
use crate::astro::ephemeris::SwissEphemerisProvider;
use crate::astro::types::{Planet, PointId, normalize_degrees};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitEvent {
    pub exact_jd_ut: f64,
    pub transiting: Planet,
    pub target: PointId,
    pub aspect: AspectKind,
    pub transiting_longitude: f64,
    pub retrograde: bool,
}

pub(super) fn search(
    provider: &SwissEphemerisProvider,
    natal: &Chart,
    start_jd: f64,
    end_jd: f64,
    policy: &OrbPolicy,
) -> Result<Vec<TransitEvent>, TimingError> {
    validate_range(start_jd, end_jd, 3660.0)?;
    let mut events = Vec::new();
    let mut targets = natal
        .positions
        .iter()
        .map(|position| (PointId::Planet(position.planet), position.longitude))
        .collect::<Vec<_>>();
    targets.extend([
        (PointId::Ascendant, natal.houses.ascendant),
        (PointId::Midheaven, natal.houses.midheaven),
    ]);
    targets.extend(natal.lots.iter().filter_map(|lot| match lot.kind {
        crate::astro::LotKind::Fortune => Some((PointId::LotFortune, lot.longitude)),
        crate::astro::LotKind::Spirit => Some((PointId::LotSpirit, lot.longitude)),
        _ => None,
    }));
    for transiting in Planet::ALL {
        for (target, target_longitude) in &targets {
            for aspect in AspectKind::ALL {
                for orientation in oriented_targets(aspect) {
                    scan_target(
                        provider,
                        transiting,
                        *target,
                        *target_longitude,
                        aspect,
                        *orientation,
                        start_jd,
                        end_jd,
                        policy,
                        &mut events,
                    )?;
                }
            }
        }
    }
    events.sort_by(|left, right| left.exact_jd_ut.total_cmp(&right.exact_jd_ut));
    events.dedup_by(|left, right| {
        left.transiting == right.transiting
            && left.target == right.target
            && left.aspect == right.aspect
            && (left.exact_jd_ut - right.exact_jd_ut).abs() < 1e-5
    });
    Ok(events)
}

#[allow(clippy::too_many_arguments)]
fn scan_target(
    provider: &SwissEphemerisProvider,
    transiting: Planet,
    target: PointId,
    target_longitude: f64,
    aspect: AspectKind,
    oriented_target: f64,
    start_jd: f64,
    end_jd: f64,
    policy: &OrbPolicy,
    output: &mut Vec<TransitEvent>,
) -> Result<(), TimingError> {
    let point = PointId::Planet(transiting);
    let allowed = policy.allowed_orb(aspect, point, target);
    if allowed <= 0.0 {
        return Ok(());
    }
    let step = scan_step(transiting);
    let mut left_jd = start_jd;
    let mut left_value = target_function(
        provider,
        transiting,
        target_longitude,
        oriented_target,
        left_jd,
    )?;
    while left_jd < end_jd {
        let right_jd = (left_jd + step).min(end_jd);
        let right_value = target_function(
            provider,
            transiting,
            target_longitude,
            oriented_target,
            right_jd,
        )?;
        if crosses_zero(left_value, right_value) {
            let exact = bisect(
                provider,
                transiting,
                target_longitude,
                oriented_target,
                left_jd,
                right_jd,
                left_value,
            )?;
            let position = provider.ecliptic_position(exact, transiting)?;
            output.push(TransitEvent {
                exact_jd_ut: exact,
                transiting,
                target,
                aspect,
                transiting_longitude: normalize_degrees(position[0]),
                retrograde: position[3] < 0.0,
            });
        }
        left_jd = right_jd;
        left_value = right_value;
    }
    Ok(())
}

pub(super) fn find_longitude_crossing(
    provider: &SwissEphemerisProvider,
    planet: Planet,
    target_longitude: f64,
    start_jd: f64,
    end_jd: f64,
) -> Result<Option<f64>, TimingError> {
    validate_range(start_jd, end_jd, 4000.0)?;
    let step = scan_step(planet);
    let mut left_jd = start_jd;
    let mut left_value = target_function(provider, planet, target_longitude, 0.0, left_jd)?;
    while left_jd < end_jd {
        let right_jd = (left_jd + step).min(end_jd);
        let right_value = target_function(provider, planet, target_longitude, 0.0, right_jd)?;
        if crosses_zero(left_value, right_value) {
            return bisect(
                provider,
                planet,
                target_longitude,
                0.0,
                left_jd,
                right_jd,
                left_value,
            )
            .map(Some);
        }
        left_jd = right_jd;
        left_value = right_value;
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn bisect(
    provider: &SwissEphemerisProvider,
    planet: Planet,
    natal_longitude: f64,
    oriented_target: f64,
    mut left: f64,
    mut right: f64,
    mut left_value: f64,
) -> Result<f64, TimingError> {
    for _ in 0..48 {
        let middle = f64::midpoint(left, right);
        let middle_value =
            target_function(provider, planet, natal_longitude, oriented_target, middle)?;
        if middle_value.abs() < 1e-9 || (right - left) < 1e-8 {
            return Ok(middle);
        }
        if crosses_zero(left_value, middle_value) {
            right = middle;
        } else {
            left = middle;
            left_value = middle_value;
        }
    }
    Ok(f64::midpoint(left, right))
}

fn target_function(
    provider: &SwissEphemerisProvider,
    planet: Planet,
    natal_longitude: f64,
    oriented_target: f64,
    jd: f64,
) -> Result<f64, TimingError> {
    let longitude = provider.ecliptic_position(jd, planet)?[0];
    Ok(signed_degrees(
        longitude - natal_longitude - oriented_target,
    ))
}

fn crosses_zero(left: f64, right: f64) -> bool {
    (left == 0.0 || right == 0.0 || left.signum() != right.signum()) && (left - right).abs() < 180.0
}

fn signed_degrees(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}

fn oriented_targets(aspect: AspectKind) -> &'static [f64] {
    match aspect {
        AspectKind::Conjunction => &[0.0],
        AspectKind::Sextile => &[-60.0, 60.0],
        AspectKind::Square => &[-90.0, 90.0],
        AspectKind::Trine => &[-120.0, 120.0],
        AspectKind::Opposition => &[180.0],
    }
}

const fn scan_step(planet: Planet) -> f64 {
    match planet {
        Planet::Moon => 0.20,
        Planet::Mercury | Planet::Venus => 0.50,
        Planet::Sun | Planet::Mars | Planet::Jupiter | Planet::Saturn => 1.0,
    }
}

fn validate_range(start: f64, end: f64, maximum: f64) -> Result<(), TimingError> {
    if !start.is_finite() || !end.is_finite() || end <= start {
        return Err(TimingError::InvalidRange(
            "end must be later than start".to_owned(),
        ));
    }
    if end - start > maximum {
        return Err(TimingError::InvalidRange(format!(
            "range may not exceed {maximum:.0} days"
        )));
    }
    Ok(())
}
