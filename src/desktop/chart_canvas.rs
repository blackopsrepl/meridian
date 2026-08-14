mod geometry;
mod inspection;

use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Font, Point, Rectangle, Renderer, Size, Theme};

use crate::astro::{AspectKind, Chart, LotKind, MercuryTendency, Planet, PointId, ZodiacSign};

use geometry::{
    ACCENT, ACCENT_BRIGHT, ACCENT_SOFT, ANGLE, BACKGROUND, BENEFIC, BORDER, BORDER_STRONG, DANGER,
    HOUSE_LINE, HOUSE_SELECTED, LOT_BACKGROUND, LOT_BORDER, MALEFIC, MIXED, Metrics,
    PLANET_BACKGROUND, RING_A, RING_B, TEXT, TEXT_MUTED, annular_sector, aspect_color, distance,
    point_segment_distance,
};
pub use inspection::Inspection;

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
        self.draw_lots(frame, &metrics);
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
            let Some(left_longitude) = self.chart.point_longitude(aspect.left) else {
                continue;
            };
            let Some(right_longitude) = self.chart.point_longitude(aspect.right) else {
                continue;
            };
            let selected = self.selected == Some(Inspection::Aspect(index));
            let related = self
                .selected
                .and_then(Inspection::point)
                .is_some_and(|point| aspect.left == point || aspect.right == point);
            let allowed = self
                .chart
                .orb_policy
                .allowed_orb(aspect.kind, aspect.left, aspect.right);
            let strength = (1.0 - (aspect.orb / allowed).clamp(0.0, 1.0)) as f32;
            let color = if selected || related {
                aspect_color(aspect.kind)
            } else {
                aspect_color(aspect.kind).scale_alpha(0.28 + strength * 0.34)
            };
            let left = metrics.point(left_longitude, metrics.aspect_radius());
            let right = metrics.point(right_longitude, metrics.aspect_radius());
            frame.stroke(
                &Path::line(left, right),
                Stroke::default()
                    .with_width(if selected {
                        3.2
                    } else if related {
                        2.2
                    } else {
                        0.9 + strength * 0.5
                    })
                    .with_color(color),
            );

            for endpoint in [left, right] {
                frame.fill(
                    &Path::circle(endpoint, if selected || related { 4.0 } else { 2.0 }),
                    color,
                );
            }

            let midpoint = Point::new(
                f32::midpoint(left.x, right.x),
                f32::midpoint(left.y, right.y),
            );
            if aspect.kind == AspectKind::Conjunction {
                frame.stroke(
                    &Path::circle(midpoint, if selected { 7.0 } else { 4.5 }),
                    Stroke::default()
                        .with_width(if selected { 2.5 } else { 1.4 })
                        .with_color(color),
                );
            }
            if selected {
                frame.fill(&Path::circle(midpoint, 12.0), BACKGROUND);
                frame.stroke(
                    &Path::circle(midpoint, 12.0),
                    Stroke::default().with_width(1.5).with_color(color),
                );
                frame.fill_text(canvas::Text {
                    content: aspect.kind.glyph().to_owned(),
                    position: midpoint,
                    size: 14.0.into(),
                    color,
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }
    }

    fn draw_planets(&self, frame: &mut Frame, metrics: &Metrics) {
        for (position, center) in self.planet_markers(metrics) {
            let selected = self.point_emphasized(PointId::Planet(position.planet));
            let (selected_fill, selected_border) = if selected && position.planet == Planet::Mercury
            {
                let color = match self.chart.mercury_nature().tendency {
                    MercuryTendency::Benefic => BENEFIC,
                    MercuryTendency::Malefic => MALEFIC,
                    MercuryTendency::Mixed => MIXED,
                    MercuryTendency::Convertible => ACCENT,
                };
                (color, color)
            } else {
                (ACCENT, ACCENT_BRIGHT)
            };
            let anchor = metrics.point(position.longitude, metrics.sign_inner - 3.0);
            frame.stroke(
                &Path::line(anchor, center),
                Stroke::default().with_width(0.8).with_color(BORDER_STRONG),
            );
            frame.fill(
                &Path::circle(center, if selected { 16.0 } else { 13.0 }),
                if selected {
                    selected_fill
                } else {
                    PLANET_BACKGROUND
                },
            );
            frame.stroke(
                &Path::circle(center, if selected { 16.0 } else { 13.0 }),
                Stroke::default()
                    .with_width(if selected { 2.0 } else { 1.0 })
                    .with_color(if selected {
                        selected_border
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

    fn draw_lots(&self, frame: &mut Frame, metrics: &Metrics) {
        for (kind, point, glyph) in [
            (LotKind::Fortune, PointId::LotFortune, "⊗"),
            (LotKind::Spirit, PointId::LotSpirit, "⊙"),
        ] {
            let Some(lot) = self.chart.lot(kind) else {
                continue;
            };
            let selected = self.point_emphasized(point);
            let center = metrics.point(lot.longitude, metrics.lot_radius());
            let radius = if selected { 13.0 } else { 10.5 };
            frame.stroke(
                &Path::line(
                    metrics.point(lot.longitude, metrics.aspect_radius()),
                    center,
                ),
                Stroke::default().with_width(0.8).with_color(BORDER_STRONG),
            );
            frame.fill(
                &Path::circle(center, radius),
                if selected { ACCENT } else { LOT_BACKGROUND },
            );
            frame.stroke(
                &Path::circle(center, radius),
                Stroke::default()
                    .with_width(if selected { 2.0 } else { 1.0 })
                    .with_color(if selected { ACCENT_BRIGHT } else { LOT_BORDER }),
            );
            frame.fill_text(canvas::Text {
                content: glyph.to_owned(),
                position: center,
                size: 15.0.into(),
                color: if selected { BACKGROUND } else { TEXT },
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                font: Font::DEFAULT,
                ..canvas::Text::default()
            });
        }
    }

    fn draw_angles(&self, frame: &mut Frame, metrics: &Metrics) {
        for (inspection, longitude, label) in [
            (Inspection::Ascendant, self.chart.houses.ascendant, "ASC"),
            (Inspection::Midheaven, self.chart.houses.midheaven, "MC"),
        ] {
            let point_id = match inspection {
                Inspection::Ascendant => PointId::Ascendant,
                Inspection::Midheaven => PointId::Midheaven,
                _ => unreachable!(),
            };
            let selected = self.point_emphasized(point_id);
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
        for (position, center) in self.planet_markers(&metrics) {
            if distance(point, center) <= 19.0 {
                return Some(Inspection::Planet(position.planet));
            }
        }

        for kind in [LotKind::Fortune, LotKind::Spirit] {
            if self.chart.lot(kind).is_some_and(|lot| {
                distance(point, metrics.point(lot.longitude, metrics.lot_radius())) <= 16.0
            }) {
                return Some(Inspection::Lot(kind));
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

        let closest_aspect = self
            .chart
            .aspects
            .iter()
            .enumerate()
            .filter_map(|(index, aspect)| {
                let left = self.chart.point_longitude(aspect.left)?;
                let right = self.chart.point_longitude(aspect.right)?;
                let distance = point_segment_distance(
                    point,
                    metrics.point(left, metrics.aspect_radius()),
                    metrics.point(right, metrics.aspect_radius()),
                );
                (distance <= 7.0).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1));
        if let Some((index, _)) = closest_aspect {
            return Some(Inspection::Aspect(index));
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

    fn point_emphasized(&self, point: PointId) -> bool {
        if self.selected.and_then(Inspection::point) == Some(point) {
            return true;
        }
        let Some(Inspection::Aspect(index)) = self.selected else {
            return false;
        };
        self.chart
            .aspects
            .get(index)
            .is_some_and(|aspect| aspect.left == point || aspect.right == point)
    }

    fn planet_markers<'a>(
        &'a self,
        metrics: &Metrics,
    ) -> Vec<(&'a crate::astro::PlanetPosition, Point)> {
        let mut positions = self.chart.positions.iter().collect::<Vec<_>>();
        positions.sort_by(|left, right| left.longitude.total_cmp(&right.longitude));
        let mut markers = Vec::with_capacity(positions.len());
        for position in positions {
            let radii = [
                metrics.planet_radius,
                metrics.planet_radius - 28.0,
                metrics.planet_radius + 24.0,
            ];
            let center = radii
                .into_iter()
                .map(|radius| metrics.point(position.longitude, radius))
                .find(|candidate| {
                    markers
                        .iter()
                        .all(|(_, existing)| distance(*candidate, *existing) >= 29.0)
                })
                .unwrap_or_else(|| metrics.point(position.longitude, radii[1]));
            markers.push((position, center));
        }
        markers
    }
}
