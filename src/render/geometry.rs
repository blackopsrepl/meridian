pub(crate) fn wheel_point(
    center_x: f64,
    center_y: f64,
    radius: f64,
    longitude: f64,
    top_longitude: f64,
) -> (f64, f64) {
    let angle = (90.0 + (longitude - top_longitude)).to_radians();
    (
        center_x + radius * angle.cos(),
        center_y - radius * angle.sin(),
    )
}

pub(crate) fn wheel_longitude(
    center_x: f64,
    center_y: f64,
    x: f64,
    y: f64,
    top_longitude: f64,
) -> f64 {
    let angle = (center_y - y).atan2(x - center_x).to_degrees();
    (angle - 90.0 + top_longitude).rem_euclid(360.0)
}

pub(crate) const SVG_STYLE: &str = r"
.chart-wheel { background: #0c101a; color: #e0e6f0; font-family: system-ui, sans-serif; }
.wheel-paper { fill: #0f141f; stroke: #4a5c7a; stroke-width: 1.5; }
.zodiac-sector { fill-opacity: .38; stroke: #52627a; stroke-width: .65; }
.zodiac-glyph, .house-number, .angle-label, .planet-glyph, .lot text, .center-mark, .center-sect { fill: #e0e6f0; text-anchor: middle; dominant-baseline: central; }
.zodiac-glyph { font-size: 22px; }
.zodiac-boundary, .inner-boundary { fill: none; stroke: #52627a; stroke-width: 1.25; }
.degree-tick { stroke: #344258; stroke-width: .6; }
.degree-tick.major { stroke: #718097; stroke-width: 1; }
.house-cusp { stroke: #46566f; stroke-width: 1; }
.house-cusp.angular { stroke: #f5b65a; stroke-width: 1.8; }
.house-number { fill: #9aa7ba; font: 12px ui-monospace, monospace; }
.angle-label { fill: #f5b65a; font: 700 12px ui-monospace, monospace; }
.aspect { fill: none; stroke-width: 1.35; }
.aspect.conjunction, .aspect-marker.conjunction { stroke: #cbd5e1; }
.aspect.sextile { stroke: #4ade80; }
.aspect.square { stroke: #f87171; }
.aspect.trine { stroke: #60a5fa; }
.aspect.opposition { stroke: #fb923c; }
.aspect-marker { fill: #0f141f; stroke-width: 1.5; }
.planet-leader { stroke: #52627a; stroke-width: .8; }
.planet-disc { fill: #18202f; stroke: #63728a; stroke-width: 1; }
.planet-glyph { font-size: 19px; }
.planet-degree { fill: #aab5c6; font: 10px ui-monospace, monospace; text-anchor: middle; }
.lot circle { fill: #101724; stroke: #b7c2d2; stroke-width: 1; }
.lot text { font-size: 13px; }
.center-mark { fill: #2fbda6; font-size: 20px; font-weight: 700; }
.center-sect { fill: #9aa7ba; font: 11px ui-monospace, monospace; }
.relationship-wheel .planet.first .planet-disc { stroke: #2fbda6; }
.relationship-wheel .planet.second .planet-disc { stroke: #f5b65a; }
";

#[cfg(test)]
mod tests {
    use super::{wheel_longitude, wheel_point};

    #[test]
    fn longitudes_increase_anticlockwise_from_the_top() {
        let top = 42.0;
        let (top_x, top_y) = wheel_point(100.0, 100.0, 50.0, top, top);
        let (left_x, left_y) = wheel_point(100.0, 100.0, 50.0, top + 90.0, top);
        let (right_x, right_y) = wheel_point(100.0, 100.0, 50.0, top - 90.0, top);

        assert!((top_x - 100.0).abs() < 1e-10);
        assert!((top_y - 50.0).abs() < 1e-10);
        assert!((left_x - 50.0).abs() < 1e-10);
        assert!((left_y - 100.0).abs() < 1e-10);
        assert!((right_x - 150.0).abs() < 1e-10);
        assert!((right_y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn wheel_projection_round_trips_longitudes() {
        let top = 279.611;
        for longitude in [0.0, 24.266, 90.0, 180.0, 279.611, 359.999] {
            let (x, y) = wheel_point(400.0, 400.0, 250.0, longitude, top);
            let projected = wheel_longitude(400.0, 400.0, x, y, top);
            let error = (projected - longitude)
                .rem_euclid(360.0)
                .min((longitude - projected).rem_euclid(360.0));
            assert!(error < 1e-10, "{longitude} projected as {projected}");
        }
    }
}
