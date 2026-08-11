use crate::astro::{AspectKind, Chart, CompositeChart, Planet, PointId, Synastry, ZodiacSign};

#[must_use]
pub fn render_synastry_wheel(
    first: &Chart,
    second: &Chart,
    synastry: &Synastry,
    size: u16,
) -> String {
    let size = f64::from(size);
    let center = size / 2.0;
    let top_longitude = first.houses.midheaven;
    let mut svg = wheel_start(
        size,
        &format!("{} × {}", first.request.title, second.request.title),
        "Synastry bi-wheel with two classical septenaries and cross-chart Ptolemaic aspects.",
    );
    draw_zodiac(&mut svg, center, size * 0.46, size * 0.39, top_longitude);
    draw_houses(
        &mut svg,
        center,
        size * 0.39,
        size * 0.24,
        top_longitude,
        &first.houses.cusps,
    );

    svg.push_str("<g class=\"synastry-aspects\">");
    for aspect in &synastry.aspects {
        let Some(left) = first.planet(aspect.first) else {
            continue;
        };
        let Some(right) = second.planet(aspect.second) else {
            continue;
        };
        let (x1, y1) = polar(center, size * 0.225, left.longitude, top_longitude);
        let (x2, y2) = polar(center, size * 0.225, right.longitude, top_longitude);
        svg.push_str(&format!(
            r#"<line class="aspect {}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}" opacity="{:.3}"/>"#,
            aspect_class(aspect.kind),
            (0.25 + (1.0 - (aspect.orb / 10.0).min(1.0)) * 0.55).clamp(0.2, 0.8)
        ));
    }
    svg.push_str("</g>");
    draw_planet_ring(
        &mut svg,
        center,
        size * 0.345,
        top_longitude,
        &first.positions,
        "first",
    );
    draw_planet_ring(
        &mut svg,
        center,
        size * 0.285,
        top_longitude,
        &second.positions,
        "second",
    );
    svg.push_str(&format!(
        r#"<circle class="inner-boundary" cx="{center:.3}" cy="{center:.3}" r="{:.3}"/><text class="center-mark" x="{center:.3}" y="{:.3}">⋈</text><text class="center-sect" x="{center:.3}" y="{:.3}">outer: {} · inner: {}</text></svg>"#,
        size * 0.225,
        center - 4.0,
        center + 23.0,
        xml_escape(&first.request.title),
        xml_escape(&second.request.title)
    ));
    svg
}

#[must_use]
pub fn render_composite_wheel(composite: &CompositeChart, size: u16) -> String {
    let size = f64::from(size);
    let center = size / 2.0;
    let top_longitude = composite.houses.midheaven;
    let mut svg = wheel_start(
        size,
        &composite.title,
        "Circular-midpoint composite chart using only the traditional seven planets.",
    );
    draw_zodiac(&mut svg, center, size * 0.46, size * 0.385, top_longitude);
    draw_houses(
        &mut svg,
        center,
        size * 0.385,
        size * 0.255,
        top_longitude,
        &composite.houses.cusps,
    );
    for aspect in &composite.aspects {
        let (PointId::Planet(left), PointId::Planet(right)) = (aspect.left, aspect.right) else {
            continue;
        };
        let Some(left_position) = composite
            .positions
            .iter()
            .find(|value| value.planet == left)
        else {
            continue;
        };
        let Some(right_position) = composite
            .positions
            .iter()
            .find(|value| value.planet == right)
        else {
            continue;
        };
        let (x1, y1) = polar(center, size * 0.205, left_position.longitude, top_longitude);
        let (x2, y2) = polar(
            center,
            size * 0.205,
            right_position.longitude,
            top_longitude,
        );
        svg.push_str(&format!(
            r#"<line class="aspect {}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}" opacity=".62"/>"#,
            aspect_class(aspect.kind)
        ));
    }
    draw_planet_ring(
        &mut svg,
        center,
        size * 0.315,
        top_longitude,
        &composite.positions,
        "composite",
    );
    svg.push_str(&format!(
        r#"<circle class="inner-boundary" cx="{center:.3}" cy="{center:.3}" r="{:.3}"/><text class="center-mark" x="{center:.3}" y="{:.3}">M</text><text class="center-sect" x="{center:.3}" y="{:.3}">circular midpoint composite</text></svg>"#,
        size * 0.205,
        center - 4.0,
        center + 23.0
    ));
    svg
}

fn wheel_start(size: f64, title: &str, description: &str) -> String {
    format!(
        r#"<svg class="chart-wheel relationship-wheel" viewBox="0 0 {size:.0} {size:.0}" role="img" aria-label="{}" xmlns="http://www.w3.org/2000/svg"><title>{}</title><desc>{}</desc><circle class="wheel-paper" cx="{:.3}" cy="{:.3}" r="{:.3}"/>"#,
        xml_escape(title),
        xml_escape(title),
        xml_escape(description),
        size / 2.0,
        size / 2.0,
        size * 0.46
    )
}

fn draw_zodiac(svg: &mut String, center: f64, outer: f64, inner: f64, top_longitude: f64) {
    for sign in ZodiacSign::ALL {
        let start = f64::from(sign.index()) * 30.0;
        let (x1, y1) = polar(center, outer, start, top_longitude);
        let (x2, y2) = polar(center, inner, start, top_longitude);
        let (tx, ty) = polar(
            center,
            f64::midpoint(outer, inner),
            start + 15.0,
            top_longitude,
        );
        svg.push_str(&format!(
            r#"<line class="degree-tick major" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}"/><text class="zodiac-glyph" x="{tx:.3}" y="{ty:.3}">{}</text>"#,
            sign.glyph()
        ));
    }
    svg.push_str(&format!(
        r#"<circle class="zodiac-boundary" cx="{center:.3}" cy="{center:.3}" r="{outer:.3}"/><circle class="zodiac-boundary" cx="{center:.3}" cy="{center:.3}" r="{inner:.3}"/>"#
    ));
}

fn draw_houses(
    svg: &mut String,
    center: f64,
    outer: f64,
    inner: f64,
    top_longitude: f64,
    cusps: &[f64; 12],
) {
    for (index, cusp) in cusps.iter().enumerate() {
        let (x1, y1) = polar(center, outer, *cusp, top_longitude);
        let (x2, y2) = polar(center, inner, *cusp, top_longitude);
        svg.push_str(&format!(
            r#"<line class="house-cusp{}" x1="{x1:.3}" y1="{y1:.3}" x2="{x2:.3}" y2="{y2:.3}"/>"#,
            if matches!(index, 0 | 3 | 6 | 9) {
                " angular"
            } else {
                ""
            }
        ));
    }
    let (mc_x, mc_y) = polar(center, outer + 14.0, top_longitude, top_longitude);
    svg.push_str(&format!(
        r#"<text class="angle-label" x="{mc_x:.3}" y="{mc_y:.3}">MC</text>"#
    ));
}

fn draw_planet_ring(
    svg: &mut String,
    center: f64,
    radius: f64,
    top_longitude: f64,
    positions: &[crate::astro::PlanetPosition],
    class: &str,
) {
    for position in positions {
        let (x, y) = polar(center, radius, position.longitude, top_longitude);
        svg.push_str(&format!(
            r#"<g class="planet {class}" data-planet="{}"><circle class="planet-disc" cx="{x:.3}" cy="{y:.3}" r="14"/><text class="planet-glyph" x="{x:.3}" y="{y:.3}">{}</text></g>"#,
            planet_key(position.planet),
            position.planet.glyph()
        ));
    }
}

const fn aspect_class(kind: AspectKind) -> &'static str {
    match kind {
        AspectKind::Conjunction => "conjunction",
        AspectKind::Sextile | AspectKind::Trine => "harmonious",
        AspectKind::Square | AspectKind::Opposition => "challenging",
    }
}

const fn planet_key(planet: Planet) -> &'static str {
    match planet {
        Planet::Sun => "sun",
        Planet::Moon => "moon",
        Planet::Mercury => "mercury",
        Planet::Venus => "venus",
        Planet::Mars => "mars",
        Planet::Jupiter => "jupiter",
        Planet::Saturn => "saturn",
    }
}

fn polar(center: f64, radius: f64, longitude: f64, top_longitude: f64) -> (f64, f64) {
    let angle = (90.0 - (longitude - top_longitude)).to_radians();
    (center + radius * angle.cos(), center - radius * angle.sin())
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
    use super::polar;

    #[test]
    fn relationship_midheaven_is_oriented_to_twelve_oclock() {
        let (x, y) = polar(100.0, 50.0, 42.25, 42.25);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 50.0).abs() < 1e-10);
    }
}
