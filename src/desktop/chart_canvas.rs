use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Fill, Font, Point, Rectangle, Renderer, Size, Theme};

use crate::astro::{AspectKind, Chart, Planet, PointId, ZodiacSign};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inspection {
    Planet(Planet),
    Sign(ZodiacSign),
    House(u8),
    Aspect(usize),
    Ascendant,
    Midheaven,
}

pub fn view(chart: &Chart, selected: Option<Inspection>) -> Element<'_, Inspection> {
    Canvas::new(ChartCanvas { chart, selected })
        .width(Fill)
        .height(Fill)
        .into()
}

struct ChartCanvas<'a> {
    chart: &'a Chart,
    selected: Option<Inspection>,
}

impl canvas::Program<Inspection> for ChartCanvas<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Inspection>> {
        let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return None;
        };
        let point = cursor.position_in(bounds)?;
        self.hit_test(bounds.size(), point)
            .map(canvas::Action::publish)
            .map(canvas::Action::and_capture)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        self.draw_wheel(&mut frame);
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor
            .position_in(bounds)
            .is_some_and(|point| self.hit_test(bounds.size(), point).is_some())
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl ChartCanvas<'_> {
    fn draw_wheel(&self, frame: &mut Frame) {
        let metrics = Metrics::new(frame.size(), self.chart.houses.midheaven);
        frame.fill(&Path::rectangle(Point::ORIGIN, frame.size()), BACKGROUND);

        for index in 0..12 {
            let sign = ZodiacSign::ALL[index];
            let start = index as f64 * 30.0;
            let end = start + 30.0;
            let midpoint = start + 15.0;
            let selected = self.selected == Some(Inspection::Sign(sign));

            let wedge = annular_sector(&metrics, start, end, metrics.sign_inner, metrics.outer);
            frame.fill(
                &wedge,
                if selected {
                    ACCENT_SOFT
                } else if index % 2 == 0 {
                    RING_A
                } else {
                    RING_B
                },
            );

            let label = metrics.point(midpoint, f32::midpoint(metrics.sign_inner, metrics.outer));
            frame.fill_text(canvas::Text {
                content: sign.glyph().to_owned(),
                position: label,
                size: 22.0.into(),
                color: if selected { ACCENT } else { TEXT },
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                font: Font::DEFAULT,
                ..canvas::Text::default()
            });
        }

        for radius in [metrics.inner, metrics.sign_inner, metrics.outer] {
            frame.stroke(
                &Path::circle(metrics.center, radius),
                Stroke::default().with_width(1.2).with_color(BORDER),
            );
        }

        for degree in (0..360).step_by(5) {
            let major = degree % 30 == 0;
            let outer = metrics.point(f64::from(degree), metrics.outer);
            let inner = metrics.point(
                f64::from(degree),
                metrics.outer - if major { 12.0 } else { 5.0 },
            );
            frame.stroke(
                &Path::line(inner, outer),
                Stroke::default()
                    .with_width(if major { 1.4 } else { 0.7 })
                    .with_color(if major { TEXT_MUTED } else { BORDER }),
            );
        }

        self.draw_houses(frame, &metrics);
        self.draw_aspects(frame, &metrics);
        self.draw_planets(frame, &metrics);
        self.draw_angles(frame, &metrics);
    }

    fn draw_houses(&self, frame: &mut Frame, metrics: &Metrics) {
        if let Some(Inspection::House(house)) = self.selected {
            let index = usize::from(house.saturating_sub(1));
            let start = self.chart.houses.cusps[index];
            let end = self.chart.houses.cusps[(index + 1) % 12];
            frame.fill(
                &annular_sector(
                    metrics,
                    start,
                    start + (end - start).rem_euclid(360.0),
                    metrics.inner * 0.28,
                    metrics.sign_inner,
                ),
                HOUSE_SELECTED,
            );
        }

        for (index, cusp) in self.chart.houses.cusps.iter().enumerate() {
            let selected = self.selected == Some(Inspection::House(index as u8 + 1));
            let previous_house = if index == 0 { 12 } else { index as u8 };
            let boundary_selected =
                selected || self.selected == Some(Inspection::House(previous_house));
            let start = metrics.point(*cusp, metrics.inner * 0.28);
            let end = metrics.point(*cusp, metrics.sign_inner);
            frame.stroke(
                &Path::line(start, end),
                Stroke::default()
                    .with_width(if boundary_selected { 2.5 } else { 1.0 })
                    .with_color(if boundary_selected {
                        ACCENT
                    } else {
                        HOUSE_LINE
                    }),
            );

            let next = self.chart.houses.cusps[(index + 1) % 12];
            let span = (next - cusp).rem_euclid(360.0);
            let label = metrics.point(cusp + span / 2.0, metrics.inner * 0.86);
            frame.fill_text(canvas::Text {
                content: (index + 1).to_string(),
                position: label,
                size: 12.0.into(),
                color: if selected { ACCENT } else { TEXT_MUTED },
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                font: Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }

    fn draw_aspects(&self, frame: &mut Frame, metrics: &Metrics) {
        for (index, aspect) in self.chart.aspects.iter().enumerate() {
            let Some(left_longitude) = point_longitude(self.chart, aspect.left) else {
                continue;
            };
            let Some(right_longitude) = point_longitude(self.chart, aspect.right) else {
                continue;
            };
            let selected = self.selected == Some(Inspection::Aspect(index));
            let related = match self.selected {
                Some(Inspection::Planet(planet)) => {
                    aspect.left == PointId::Planet(planet)
                        || aspect.right == PointId::Planet(planet)
                }
                _ => false,
            };
            let color = if selected || related {
                aspect_color(aspect.kind)
            } else {
                aspect_color(aspect.kind).scale_alpha(0.38)
            };
            frame.stroke(
                &Path::line(
                    metrics.point(left_longitude, metrics.inner * 0.69),
                    metrics.point(right_longitude, metrics.inner * 0.69),
                ),
                Stroke::default()
                    .with_width(if selected {
                        3.0
                    } else if related {
                        2.0
                    } else {
                        1.0
                    })
                    .with_color(color),
            );
        }
    }

    fn draw_planets(&self, frame: &mut Frame, metrics: &Metrics) {
        for position in &self.chart.positions {
            let selected = self.selected == Some(Inspection::Planet(position.planet));
            let center = metrics.point(position.longitude, metrics.planet_radius);
            frame.fill(
                &Path::circle(center, if selected { 16.0 } else { 13.0 }),
                if selected { ACCENT } else { PLANET_BACKGROUND },
            );
            frame.stroke(
                &Path::circle(center, if selected { 16.0 } else { 13.0 }),
                Stroke::default()
                    .with_width(if selected { 2.0 } else { 1.0 })
                    .with_color(if selected {
                        ACCENT_BRIGHT
                    } else {
                        BORDER_STRONG
                    }),
            );
            frame.fill_text(canvas::Text {
                content: position.planet.glyph().to_owned(),
                position: center,
                size: 18.0.into(),
                color: if selected { BACKGROUND } else { TEXT },
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                font: Font::DEFAULT,
                ..canvas::Text::default()
            });
            if position.retrograde {
                frame.fill_text(canvas::Text {
                    content: "R".to_owned(),
                    position: Point::new(center.x + 12.0, center.y - 13.0),
                    size: 9.0.into(),
                    color: DANGER,
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: Font::MONOSPACE,
                    ..canvas::Text::default()
                });
            }
        }
    }

    fn draw_angles(&self, frame: &mut Frame, metrics: &Metrics) {
        for (inspection, longitude, label) in [
            (Inspection::Ascendant, self.chart.houses.ascendant, "ASC"),
            (Inspection::Midheaven, self.chart.houses.midheaven, "MC"),
        ] {
            let selected = self.selected == Some(inspection);
            let inner = metrics.point(longitude, metrics.inner * 0.20);
            let outer = metrics.point(longitude, metrics.outer);
            frame.stroke(
                &Path::line(inner, outer),
                Stroke::default()
                    .with_width(if selected { 3.0 } else { 1.8 })
                    .with_color(if selected { ACCENT_BRIGHT } else { ANGLE }),
            );
            let point = metrics.point(longitude, metrics.inner * 0.10);
            frame.fill_text(canvas::Text {
                content: label.to_owned(),
                position: point,
                size: 11.0.into(),
                color: if selected { ACCENT_BRIGHT } else { ANGLE },
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                font: Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }

    fn hit_test(&self, size: Size, point: Point) -> Option<Inspection> {
        let metrics = Metrics::new(size, self.chart.houses.midheaven);
        for position in &self.chart.positions {
            if distance(
                point,
                metrics.point(position.longitude, metrics.planet_radius),
            ) <= 19.0
            {
                return Some(Inspection::Planet(position.planet));
            }
        }

        for (inspection, longitude) in [
            (Inspection::Ascendant, self.chart.houses.ascendant),
            (Inspection::Midheaven, self.chart.houses.midheaven),
        ] {
            let start = metrics.point(longitude, metrics.inner * 0.15);
            let end = metrics.point(longitude, metrics.outer);
            if point_segment_distance(point, start, end) <= 6.0 {
                return Some(inspection);
            }
        }

        for (index, aspect) in self.chart.aspects.iter().enumerate() {
            let (Some(left), Some(right)) = (
                point_longitude(self.chart, aspect.left),
                point_longitude(self.chart, aspect.right),
            ) else {
                continue;
            };
            if point_segment_distance(
                point,
                metrics.point(left, metrics.inner * 0.69),
                metrics.point(right, metrics.inner * 0.69),
            ) <= 5.0
            {
                return Some(Inspection::Aspect(index));
            }
        }

        let radial = distance(point, metrics.center);
        let longitude = metrics.longitude(point);
        if (metrics.sign_inner..=metrics.outer + 4.0).contains(&radial) {
            return Some(Inspection::Sign(ZodiacSign::from_longitude(longitude)));
        }
        if radial <= metrics.sign_inner && radial >= metrics.inner * 0.25 {
            return Some(Inspection::House(self.chart.houses.house_of(longitude)));
        }
        None
    }
}

struct Metrics {
    center: Point,
    outer: f32,
    sign_inner: f32,
    planet_radius: f32,
    inner: f32,
    top_longitude: f64,
}

impl Metrics {
    fn new(size: Size, top_longitude: f64) -> Self {
        let outer = (size.width.min(size.height) * 0.46).max(80.0);
        Self {
            center: Point::new(size.width / 2.0, size.height / 2.0),
            outer,
            sign_inner: outer * 0.84,
            planet_radius: outer * 0.74,
            inner: outer * 0.69,
            top_longitude,
        }
    }

    fn point(&self, longitude: f64, radius: f32) -> Point {
        let theta = (90.0 - (longitude - self.top_longitude).rem_euclid(360.0)).to_radians();
        Point::new(
            self.center.x + radius * theta.cos() as f32,
            self.center.y - radius * theta.sin() as f32,
        )
    }

    fn longitude(&self, point: Point) -> f64 {
        let dx = f64::from(point.x - self.center.x);
        let dy = f64::from(self.center.y - point.y);
        (90.0 - dy.atan2(dx).to_degrees() + self.top_longitude).rem_euclid(360.0)
    }
}

fn annular_sector(metrics: &Metrics, start: f64, end: f64, inner: f32, outer: f32) -> Path {
    Path::new(|builder| {
        let segments = 20;
        builder.move_to(metrics.point(start, outer));
        for step in 1..=segments {
            let longitude = start + (end - start) * f64::from(step) / f64::from(segments);
            builder.line_to(metrics.point(longitude, outer));
        }
        for step in (0..=segments).rev() {
            let longitude = start + (end - start) * f64::from(step) / f64::from(segments);
            builder.line_to(metrics.point(longitude, inner));
        }
        builder.close();
    })
}

fn point_longitude(chart: &Chart, point: PointId) -> Option<f64> {
    match point {
        PointId::Planet(planet) => chart.planet(planet).map(|position| position.longitude),
        PointId::Ascendant => Some(chart.houses.ascendant),
        PointId::Midheaven => Some(chart.houses.midheaven),
        PointId::LotFortune => chart.lots.first().map(|lot| lot.longitude),
        PointId::LotSpirit => chart.lots.get(1).map(|lot| lot.longitude),
    }
}

fn aspect_color(kind: AspectKind) -> Color {
    match kind {
        AspectKind::Conjunction => Color::from_rgb8(203, 213, 225),
        AspectKind::Sextile => Color::from_rgb8(74, 222, 128),
        AspectKind::Square => Color::from_rgb8(248, 113, 113),
        AspectKind::Trine => Color::from_rgb8(96, 165, 250),
        AspectKind::Opposition => Color::from_rgb8(251, 146, 60),
    }
}

fn distance(left: Point, right: Point) -> f32 {
    (left.x - right.x).hypot(left.y - right.y)
}

fn point_segment_distance(point: Point, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        return distance(point, start);
    }
    let ratio =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    distance(
        point,
        Point::new(start.x + ratio * dx, start.y + ratio * dy),
    )
}

const BACKGROUND: Color = Color::from_rgb(0.047, 0.063, 0.102);
const RING_A: Color = Color::from_rgb(0.075, 0.094, 0.145);
const RING_B: Color = Color::from_rgb(0.063, 0.080, 0.125);
const ACCENT_SOFT: Color = Color::from_rgb(0.16, 0.22, 0.29);
const PLANET_BACKGROUND: Color = Color::from_rgb(0.094, 0.118, 0.176);
const TEXT: Color = Color::from_rgb(0.88, 0.90, 0.94);
const TEXT_MUTED: Color = Color::from_rgb(0.52, 0.58, 0.68);
const BORDER: Color = Color::from_rgb(0.16, 0.20, 0.29);
const BORDER_STRONG: Color = Color::from_rgb(0.29, 0.36, 0.48);
const HOUSE_LINE: Color = Color::from_rgb(0.20, 0.25, 0.35);
const HOUSE_SELECTED: Color = Color::from_rgba(0.20, 0.72, 0.64, 0.14);
const ANGLE: Color = Color::from_rgb(0.96, 0.71, 0.35);
const ACCENT: Color = Color::from_rgb(0.30, 0.82, 0.72);
const ACCENT_BRIGHT: Color = Color::from_rgb(0.55, 0.96, 0.86);
const DANGER: Color = Color::from_rgb(0.98, 0.45, 0.45);
