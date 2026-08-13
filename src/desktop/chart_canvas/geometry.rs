use iced::widget::canvas::Path;
use iced::{Color, Point, Size};

use crate::astro::AspectKind;
use crate::render::geometry::{wheel_longitude, wheel_point};

pub(super) struct Metrics {
    pub(super) center: Point,
    pub(super) outer: f32,
    pub(super) sign_inner: f32,
    pub(super) planet_radius: f32,
    pub(super) inner: f32,
    top_longitude: f64,
}

impl Metrics {
    pub(super) fn new(size: Size, top_longitude: f64) -> Self {
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

    pub(super) fn point(&self, longitude: f64, radius: f32) -> Point {
        let (x, y) = wheel_point(
            f64::from(self.center.x),
            f64::from(self.center.y),
            f64::from(radius),
            longitude,
            self.top_longitude,
        );
        Point::new(x as f32, y as f32)
    }

    pub(super) fn longitude(&self, point: Point) -> f64 {
        wheel_longitude(
            f64::from(self.center.x),
            f64::from(self.center.y),
            f64::from(point.x),
            f64::from(point.y),
            self.top_longitude,
        )
    }

    pub(super) fn aspect_radius(&self) -> f32 {
        self.inner * 0.69
    }

    pub(super) fn lot_radius(&self) -> f32 {
        self.inner * 0.86
    }
}

pub(super) fn annular_sector(
    metrics: &Metrics,
    start: f64,
    end: f64,
    inner: f32,
    outer: f32,
) -> Path {
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

pub(super) fn aspect_color(kind: AspectKind) -> Color {
    match kind {
        AspectKind::Conjunction => Color::from_rgb8(203, 213, 225),
        AspectKind::Sextile => Color::from_rgb8(74, 222, 128),
        AspectKind::Square => Color::from_rgb8(248, 113, 113),
        AspectKind::Trine => Color::from_rgb8(96, 165, 250),
        AspectKind::Opposition => Color::from_rgb8(251, 146, 60),
    }
}

pub(super) fn distance(left: Point, right: Point) -> f32 {
    (left.x - right.x).hypot(left.y - right.y)
}

pub(super) fn point_segment_distance(point: Point, start: Point, end: Point) -> f32 {
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

pub(super) const BACKGROUND: Color = Color::from_rgb(0.047, 0.063, 0.102);
pub(super) const RING_A: Color = Color::from_rgb(0.075, 0.094, 0.145);
pub(super) const RING_B: Color = Color::from_rgb(0.063, 0.080, 0.125);
pub(super) const ACCENT_SOFT: Color = Color::from_rgb(0.16, 0.22, 0.29);
pub(super) const PLANET_BACKGROUND: Color = Color::from_rgb(0.094, 0.118, 0.176);
pub(super) const LOT_BACKGROUND: Color = Color::from_rgb(0.063, 0.090, 0.137);
pub(super) const LOT_BORDER: Color = Color::from_rgb(0.50, 0.58, 0.70);
pub(super) const TEXT: Color = Color::from_rgb(0.88, 0.90, 0.94);
pub(super) const TEXT_MUTED: Color = Color::from_rgb(0.52, 0.58, 0.68);
pub(super) const BORDER: Color = Color::from_rgb(0.16, 0.20, 0.29);
pub(super) const BORDER_STRONG: Color = Color::from_rgb(0.29, 0.36, 0.48);
pub(super) const HOUSE_LINE: Color = Color::from_rgb(0.20, 0.25, 0.35);
pub(super) const HOUSE_SELECTED: Color = Color::from_rgba(0.20, 0.72, 0.64, 0.14);
pub(super) const ANGLE: Color = Color::from_rgb(0.96, 0.71, 0.35);
pub(super) const ACCENT: Color = Color::from_rgb(0.30, 0.82, 0.72);
pub(super) const ACCENT_BRIGHT: Color = Color::from_rgb(0.55, 0.96, 0.86);
pub(super) const DANGER: Color = Color::from_rgb(0.98, 0.45, 0.45);
