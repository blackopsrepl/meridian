use crate::astro::{AspectKind, Chart, LotKind, PlanetPosition, PointId, ZodiacSign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelOptions {
    pub size: u16,
    pub show_aspects: bool,
    pub show_lots: bool,
}

impl Default for WheelOptions {
    fn default() -> Self {
        Self {
            size: 760,
            show_aspects: true,
            show_lots: true,
        }
    }
}

#[must_use]
pub fn render_wheel(chart: &Chart, options: WheelOptions) -> String {
    let size = f64::from(options.size);
    let center = size / 2.0;
    let outer = size * 0.455;
    let zodiac_inner = size * 0.382;
    let houses_inner = size * 0.295;
    let aspect_radius = size * 0.225;
    let ascendant = chart.houses.ascendant;
    let mut svg = format!(
        r#"<svg class="chart-wheel" viewBox="0 0 {size:.0} {size:.0}" role="img" aria-labelledby="wheel-title wheel-description" xmlns="http://www.w3.org/2000/svg"><title id="wheel-title">{}</title><desc id="wheel-description">Classical septenary chart wheel with houses, planets, lots, and Ptolemaic aspects.</desc><defs><filter id="planet-shadow" x="-30%" y="-30%" width="160%" height="160%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-opacity=".18"/></filter></defs>"#,
        xml_escape(&chart.request.title)
    );
    svg.push_str(&format!(
        r#"<circle class="wheel-paper" cx="{center:.3}" cy="{center:.3}" r="{outer:.3}"/>"#
    ));
    draw_zodiac(&mut svg, center, outer, zodiac_inner, ascendant);
    draw_degree_ticks(&mut svg, center, outer, zodiac_inner, ascendant);
    draw_houses(
        &mut svg,
        chart,
        center,
        zodiac_inner,
        houses_inner,
        ascendant,
    );
    if options.show_aspects {
        draw_aspects(&mut svg, chart, center, aspect_radius, ascendant);
    }
    draw_planets(
        &mut svg,
        chart,
        center,
        zodiac_inner,
        houses_inner,
        ascendant,
    );
    if options.show_lots {
        draw_lots(&mut svg, chart, center, houses_inner, ascendant);
    }
    draw_center(&mut svg, chart, center, aspect_radius);
    svg.push_str("</svg>");
    svg
}

fn draw_zodiac(svg: &mut String, center: f64, outer: f64, inner: f64, ascendant: f64) {
    const COLORS: [&str; 4] = ["#b74e3a", "#4d8068", "#b68a35", "#416e85"];
    for sign in ZodiacSign::ALL {
        let start = f64::from(sign.index()) * 30.0;
        let end = start + 30.0;
        let path = annular_sector(center, outer, inner, start, end, ascendant);
        let element_index = match sign.element() {
            crate::astro::Element::Fire => 0,
            crate::astro::Element::Earth => 1,
            crate::astro::Element::Air => 2,
            crate::astro::Element::Water => 3,
        };
        svg.push_str(&format!(
            r#"<path class="zodiac-sector" d="{path}" fill="{}"/>"#,
            COLORS[element_index]
        ));
        let (x, y) = polar(center, f64::midpoint(outer, inner), start + 15.0, ascendant);
        svg.push_str(&format!(
            r#"<text class="zodiac-glyph" x="{x:.3}" y="{y:.3}">{}</text>"#,
            sign.glyph()
        ));
    }
    svg.push_str(&format!(
        r#"<circle class="zodiac-boundary" cx="{center:.3}" cy="{center:.3}" r="{outer:.3}"/><circle class="zodiac-boundary" cx="{center:.3}" cy="{center:.3}" r="{inner:.3}"/>"#
    ));
}

fn draw_degree_ticks(svg: &mut String, center: f64, outer: f64, inner: f64, ascendant: f64) {
    for degree in 0..360 {
        let major = degree % 10 == 0;
        let sign_edge = degree % 30 == 0;
        let length = if sign_edge {
            outer - inner
        } else if major {
            10.0
        } else {
            4.0
        };
        let (x1, y1) = polar(center, outer, f64::from(degree), ascendant);
        let (x2, y2) = polar(center, outer - length, f64::from(degree), ascendant);
        svg.push_str(&format!(
            r#"<line class="degree-tick{}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}"/>"#,
            if major { " major" } else { "" }
        ));
    }
}

fn draw_houses(
    svg: &mut String,
    chart: &Chart,
    center: f64,
    outer: f64,
    inner: f64,
    ascendant: f64,
) {
    for (index, cusp) in chart.houses.cusps.iter().enumerate() {
        let (x1, y1) = polar(center, outer, *cusp, ascendant);
        let (x2, y2) = polar(center, inner, *cusp, ascendant);
        let angular = matches!(index, 0 | 3 | 6 | 9);
        svg.push_str(&format!(
            r#"<line class="house-cusp{}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}"/>"#,
            if angular { " angular" } else { "" }
        ));
        let next = chart.houses.cusps[(index + 1) % 12];
        let midpoint = midpoint_forward(*cusp, next);
        let (tx, ty) = polar(center, outer - 18.0, midpoint, ascendant);
        svg.push_str(&format!(
            r#"<text class="house-number" x="{tx:.3}" y="{ty:.3}">{}</text>"#,
            index + 1
        ));
    }
    let (asc_x, asc_y) = polar(center, outer + 15.0, chart.houses.ascendant, ascendant);
    let (mc_x, mc_y) = polar(center, outer + 15.0, chart.houses.midheaven, ascendant);
    svg.push_str(&format!(
        r#"<text class="angle-label" x="{asc_x:.3}" y="{asc_y:.3}">ASC</text><text class="angle-label" x="{mc_x:.3}" y="{mc_y:.3}">MC</text>"#
    ));
}

fn draw_aspects(svg: &mut String, chart: &Chart, center: f64, radius: f64, ascendant: f64) {
    svg.push_str("<g class=\"aspect-field\">");
    for aspect in &chart.aspects {
        let Some(left) = point_longitude(chart, aspect.left) else {
            continue;
        };
        let Some(right) = point_longitude(chart, aspect.right) else {
            continue;
        };
        let (x1, y1) = polar(center, radius, left, ascendant);
        let (x2, y2) = polar(center, radius, right, ascendant);
        let class = match aspect.kind {
            AspectKind::Conjunction => "conjunction",
            AspectKind::Sextile | AspectKind::Trine => "harmonious",
            AspectKind::Square | AspectKind::Opposition => "challenging",
        };
        let opacity = (0.22 + (1.0 - (aspect.orb / 10.0).min(1.0)) * 0.58).clamp(0.2, 0.8);
        svg.push_str(&format!(
            r#"<line class="aspect {class}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}" opacity="{opacity:.3}"/>"#
        ));
    }
    svg.push_str("</g>");
}

fn draw_planets(
    svg: &mut String,
    chart: &Chart,
    center: f64,
    outer: f64,
    inner: f64,
    ascendant: f64,
) {
    let mut planets = chart.positions.iter().collect::<Vec<_>>();
    planets.sort_by(|left, right| left.longitude.total_cmp(&right.longitude));
    let mut previous: Option<f64> = None;
    let mut collision_level = 0_u8;
    for position in planets {
        collision_level =
            if previous.is_some_and(|value| forward_distance(value, position.longitude) < 6.0) {
                (collision_level + 1) % 3
            } else {
                0
            };
        previous = Some(position.longitude);
        draw_planet(
            svg,
            position,
            center,
            outer,
            inner,
            ascendant,
            collision_level,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_planet(
    svg: &mut String,
    position: &PlanetPosition,
    center: f64,
    outer: f64,
    inner: f64,
    ascendant: f64,
    collision_level: u8,
) {
    let anchor_radius = outer - 12.0;
    let label_radius = inner + 28.0 + f64::from(collision_level) * 20.0;
    let (anchor_x, anchor_y) = polar(center, anchor_radius, position.longitude, ascendant);
    let (x, y) = polar(center, label_radius, position.longitude, ascendant);
    svg.push_str(&format!(
        r#"<g class="planet" data-planet="{}"><line class="planet-leader" x1="{anchor_x:.3}" y1="{anchor_y:.3}" x2="{x:.3}" y2="{y:.3}"/><circle class="planet-disc" cx="{x:.3}" cy="{y:.3}" r="15" filter="url(#planet-shadow)"/><text class="planet-glyph" x="{x:.3}" y="{y:.3}">{}</text><text class="planet-degree" x="{x:.3}" y="{:.3}">{:02.0}°{:02.0}′{}</text></g>"#,
        position.planet.name().to_lowercase(),
        position.planet.glyph(),
        y + 25.0,
        position.degree_in_sign.floor(),
        (position.degree_in_sign.fract() * 60.0).floor(),
        if position.retrograde { " ℞" } else { "" }
    ));
}

fn draw_lots(svg: &mut String, chart: &Chart, center: f64, radius: f64, ascendant: f64) {
    for lot in &chart.lots {
        let glyph = match lot.kind {
            LotKind::Fortune => "⊗",
            LotKind::Spirit => "⊙",
            _ => continue,
        };
        let (x, y) = polar(center, radius - 18.0, lot.longitude, ascendant);
        svg.push_str(&format!(
            r#"<g class="lot"><circle cx="{x:.3}" cy="{y:.3}" r="9"/><text x="{x:.3}" y="{y:.3}">{glyph}</text></g>"#
        ));
    }
}

fn draw_center(svg: &mut String, chart: &Chart, center: f64, radius: f64) {
    svg.push_str(&format!(
        r#"<circle class="inner-boundary" cx="{center:.3}" cy="{center:.3}" r="{radius:.3}"/><text class="center-mark" x="{center:.3}" y="{:.3}">M</text><text class="center-sect" x="{center:.3}" y="{:.3}">{:?} chart · {:?} Moon</text>"#,
        center - 5.0,
        center + 22.0,
        chart.sect,
        chart.lunar_phase
    ));
}

fn point_longitude(chart: &Chart, point: PointId) -> Option<f64> {
    match point {
        PointId::Planet(planet) => chart.planet(planet).map(|position| position.longitude),
        PointId::Ascendant => Some(chart.houses.ascendant),
        PointId::Midheaven => Some(chart.houses.midheaven),
        PointId::LotFortune => chart.lot(LotKind::Fortune).map(|lot| lot.longitude),
        PointId::LotSpirit => chart.lot(LotKind::Spirit).map(|lot| lot.longitude),
    }
}

fn polar(center: f64, radius: f64, longitude: f64, ascendant: f64) -> (f64, f64) {
    let angle = (180.0 - (longitude - ascendant)).to_radians();
    (center + radius * angle.cos(), center - radius * angle.sin())
}

fn annular_sector(
    center: f64,
    outer: f64,
    inner: f64,
    start: f64,
    end: f64,
    ascendant: f64,
) -> String {
    let (outer_start_x, outer_start_y) = polar(center, outer, start, ascendant);
    let (outer_end_x, outer_end_y) = polar(center, outer, end, ascendant);
    let (inner_end_x, inner_end_y) = polar(center, inner, end, ascendant);
    let (inner_start_x, inner_start_y) = polar(center, inner, start, ascendant);
    format!(
        "M {outer_start_x:.3} {outer_start_y:.3} A {outer:.3} {outer:.3} 0 0 0 {outer_end_x:.3} {outer_end_y:.3} L {inner_end_x:.3} {inner_end_y:.3} A {inner:.3} {inner:.3} 0 0 1 {inner_start_x:.3} {inner_start_y:.3} Z"
    )
}

fn midpoint_forward(start: f64, end: f64) -> f64 {
    (start + forward_distance(start, end) / 2.0).rem_euclid(360.0)
}

fn forward_distance(start: f64, end: f64) -> f64 {
    (end - start).rem_euclid(360.0)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{WheelOptions, render_wheel};
    use crate::astro::{
        Calendar, ChartCalculator, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
        SwissEphemerisProvider, TimeZoneSpec, TraditionalHouseSystem,
    };

    #[test]
    fn wheel_is_self_contained_and_septenary() -> Result<(), Box<dyn std::error::Error>> {
        let chart = ChartCalculator::new(SwissEphemerisProvider::new("data/ephe")?).calculate(
            ChartRequest {
                title: "<Classical>".to_owned(),
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
                    label: None,
                },
                location_name: "Greenwich".to_owned(),
                coordinates: Coordinates {
                    latitude: 51.4779,
                    longitude: 0.0,
                    elevation_m: 46.0,
                },
                house_system: TraditionalHouseSystem::WholeSign,
            },
        )?;
        let svg = render_wheel(&chart, WheelOptions::default());
        assert_eq!(svg.matches("data-planet=").count(), 7);
        assert!(svg.contains("&lt;Classical&gt;"));
        assert!(!svg.contains("Uranus"));
        Ok(())
    }
}
