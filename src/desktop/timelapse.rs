use std::time::Duration as TickDuration;

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use iced::widget::{button, column, container, row, slider, space, text};
use iced::{Background, Border, Center, Color, Element, Fill, Subscription, Theme};

use crate::astro::{
    Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, TimeZoneSpec,
    TraditionalHouseSystem,
};

const TICK_INTERVAL: TickDuration = TickDuration::from_millis(50);
const TICKS_PER_SECOND: i32 = 20;

const PANEL: Color = Color::from_rgb(0.055, 0.082, 0.129);
const PANEL_RAISED: Color = Color::from_rgb(0.075, 0.110, 0.169);
const BORDER: Color = Color::from_rgb(0.16, 0.22, 0.32);
const TEXT: Color = Color::from_rgb(0.88, 0.91, 0.95);
const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ACCENT: Color = Color::from_rgb(0.30, 0.82, 0.72);
const PAST: Color = Color::from_rgb(0.49, 0.68, 0.98);
const FUTURE: Color = Color::from_rgb(0.96, 0.72, 0.29);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Day,
    Month,
    Year,
}

impl Span {
    const ALL: [Self; 3] = [Self::Day, Self::Month, Self::Year];

    const fn label(self) -> &'static str {
        match self {
            Self::Day => "24 HOURS",
            Self::Month => "30 DAYS",
            Self::Year => "1 YEAR",
        }
    }

    const fn radius_minutes(self) -> i32 {
        match self {
            Self::Day => 12 * 60,
            Self::Month => 15 * 24 * 60,
            Self::Year => 183 * 24 * 60,
        }
    }

    const fn slider_step(self) -> i32 {
        match self {
            Self::Day => 5,
            Self::Month => 60,
            Self::Year => 12 * 60,
        }
    }

    const fn nudge_minutes(self) -> i32 {
        match self {
            Self::Day => 15,
            Self::Month => 6 * 60,
            Self::Year => 24 * 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    Hour,
    Day,
    Week,
}

impl Rate {
    const ALL: [Self; 3] = [Self::Hour, Self::Day, Self::Week];

    const fn label(self) -> &'static str {
        match self {
            Self::Hour => "1H / SEC",
            Self::Day => "1D / SEC",
            Self::Week => "1W / SEC",
        }
    }

    const fn minutes_per_tick(self) -> i32 {
        let minutes_per_second = match self {
            Self::Hour => 60,
            Self::Day => 24 * 60,
            Self::Week => 7 * 24 * 60,
        };
        minutes_per_second / TICKS_PER_SECOND
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePlayback,
    Scrubbed(i32),
    Nudge(i32),
    ResetNow,
    SpanChanged(Span),
    RateChanged(Rate),
    Tick,
}

#[derive(Debug, Clone)]
pub struct State {
    anchor: DateTime<Utc>,
    offset_minutes: i32,
    span: Span,
    rate: Rate,
    playing: bool,
}

impl State {
    pub fn now() -> Self {
        Self::new(Utc::now())
    }

    fn new(anchor: DateTime<Utc>) -> Self {
        Self {
            anchor,
            offset_minutes: 0,
            span: Span::Month,
            rate: Rate::Day,
            playing: false,
        }
    }

    pub fn update(&mut self, message: Message) -> bool {
        match message {
            Message::TogglePlayback => {
                self.playing = !self.playing;
                false
            }
            Message::Scrubbed(offset) => {
                self.playing = false;
                self.set_offset(offset)
            }
            Message::Nudge(direction) => {
                self.playing = false;
                self.set_offset(
                    self.offset_minutes
                        .saturating_add(direction.signum() * self.span.nudge_minutes()),
                )
            }
            Message::ResetNow => {
                self.anchor = Utc::now();
                self.offset_minutes = 0;
                self.playing = false;
                true
            }
            Message::SpanChanged(span) => {
                self.span = span;
                self.playing = false;
                self.set_offset(self.offset_minutes)
            }
            Message::RateChanged(rate) => {
                self.rate = rate;
                false
            }
            Message::Tick if self.playing => {
                let maximum = self.span.radius_minutes();
                let next = self
                    .offset_minutes
                    .saturating_add(self.rate.minutes_per_tick());
                if next >= maximum {
                    self.playing = false;
                }
                self.set_offset(next)
            }
            Message::Tick => false,
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn chart_request(&self) -> ChartRequest {
        let target = self.target();
        ChartRequest {
            title: "Current Sky".to_owned(),
            purpose: ChartPurpose::Event,
            local_time: CivilDateTime {
                year: target.year(),
                month: target.month() as u8,
                day: target.day() as u8,
                hour: target.hour() as u8,
                minute: target.minute() as u8,
                second: f64::from(target.second())
                    + f64::from(target.nanosecond()) / 1_000_000_000.0,
                calendar: Calendar::Gregorian,
            },
            time_zone: TimeZoneSpec::FixedOffset {
                minutes_east: 0,
                label: Some("UTC".to_owned()),
            },
            location_name: "Greenwich".to_owned(),
            coordinates: Coordinates {
                latitude: 51.4779,
                longitude: 0.0,
                elevation_m: 46.0,
            },
            house_system: TraditionalHouseSystem::WholeSign,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.playing {
            iced::time::every(TICK_INTERVAL).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let target = self.target();
        let (temporal_label, temporal_color) = match self.offset_minutes.cmp(&0) {
            std::cmp::Ordering::Less => ("PAST", PAST),
            std::cmp::Ordering::Equal => ("LIVE", ACCENT),
            std::cmp::Ordering::Greater => ("FUTURE", FUTURE),
        };

        let badge = container(text(temporal_label).size(10).color(temporal_color))
            .padding([4, 8])
            .style(move |_| badge_style(temporal_color));
        let timestamp = column![
            row![
                text("SKY TIMELAPSE").size(10).color(MUTED),
                badge,
                text(relative_label(self.offset_minutes))
                    .size(11)
                    .color(MUTED),
            ]
            .align_y(Center)
            .spacing(8),
            text(target.format("%a, %d %b %Y  ·  %H:%M:%S UTC").to_string())
                .size(18)
                .color(TEXT),
        ]
        .spacing(4)
        .width(Fill);

        let playback_label = if self.playing {
            "Ⅱ  PAUSE"
        } else {
            "▶  PLAY"
        };
        let controls = row![
            control_button("− STEP", Message::Nudge(-1), false),
            control_button(playback_label, Message::TogglePlayback, self.playing),
            control_button("STEP +", Message::Nudge(1), false),
            control_button("NOW", Message::ResetNow, self.offset_minutes == 0),
        ]
        .spacing(6)
        .align_y(Center);

        let radius = self.span.radius_minutes();
        let timeline = slider(-radius..=radius, self.offset_minutes, Message::Scrubbed)
            .step(self.span.slider_step())
            .shift_step(1)
            .default(0)
            .height(22)
            .style(timeline_style);
        let range_labels = row![
            text(range_label(
                self.anchor - Duration::minutes(i64::from(radius)),
                self.span
            ))
            .size(10)
            .color(MUTED),
            space::horizontal(),
            text("NOW").size(10).color(ACCENT),
            space::horizontal(),
            text(range_label(
                self.anchor + Duration::minutes(i64::from(radius)),
                self.span
            ))
            .size(10)
            .color(MUTED),
        ];

        let mut spans = row![text("RANGE").size(9).color(MUTED)]
            .spacing(5)
            .align_y(Center);
        for span in Span::ALL {
            spans = spans.push(chip_button(
                span.label(),
                self.span == span,
                Message::SpanChanged(span),
            ));
        }

        let mut rates = row![text("SPEED").size(9).color(MUTED)]
            .spacing(5)
            .align_y(Center);
        for rate in Rate::ALL {
            rates = rates.push(chip_button(
                rate.label(),
                self.rate == rate,
                Message::RateChanged(rate),
            ));
        }

        container(
            column![
                row![timestamp, controls].align_y(Center).spacing(16),
                column![timeline, range_labels].spacing(1),
                row![spans, space::horizontal(), rates].align_y(Center),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding([11, 14])
        .style(panel_style)
        .into()
    }

    fn target(&self) -> DateTime<Utc> {
        self.anchor + Duration::minutes(i64::from(self.offset_minutes))
    }

    fn set_offset(&mut self, offset: i32) -> bool {
        let radius = self.span.radius_minutes();
        let clamped = offset.clamp(-radius, radius);
        let changed = clamped != self.offset_minutes;
        self.offset_minutes = clamped;
        changed
    }
}

fn control_button(
    label: &str,
    message: Message,
    active: bool,
) -> iced::widget::Button<'_, Message> {
    button(text(label).size(11))
        .padding([7, 10])
        .style(move |theme, status| {
            if active {
                button::primary(theme, status)
            } else {
                button::secondary(theme, status)
            }
        })
        .on_press(message)
}

fn chip_button(label: &str, selected: bool, message: Message) -> iced::widget::Button<'_, Message> {
    button(text(label).size(9))
        .padding([4, 7])
        .style(move |_theme, status| chip_style(selected, status))
        .on_press(message)
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(PANEL)
        .border(Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        })
}

fn badge_style(color: Color) -> container::Style {
    container::Style::default()
        .background(Color::from_rgba(color.r, color.g, color.b, 0.10))
        .border(Border {
            color: Color::from_rgba(color.r, color.g, color.b, 0.60),
            width: 1.0,
            radius: 99.0.into(),
        })
}

fn chip_style(selected: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let color = if selected { ACCENT } else { MUTED };
    button::Style {
        background: Some(Background::Color(if selected {
            Color::from_rgba(
                ACCENT.r,
                ACCENT.g,
                ACCENT.b,
                if hovered { 0.22 } else { 0.14 },
            )
        } else if hovered {
            PANEL_RAISED
        } else {
            Color::TRANSPARENT
        })),
        text_color: color,
        border: Border {
            color: if selected { ACCENT } else { BORDER },
            width: 1.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

fn timeline_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle = match status {
        slider::Status::Active => ACCENT,
        slider::Status::Hovered | slider::Status::Dragged => Color::from_rgb(0.48, 0.95, 0.85),
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (ACCENT.into(), PANEL_RAISED.into()),
            width: 5.0,
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 3.0.into(),
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.0 },
            background: handle.into(),
            border_width: 2.0,
            border_color: PANEL,
        },
    }
}

fn relative_label(offset_minutes: i32) -> String {
    if offset_minutes == 0 {
        return "At the present moment".to_owned();
    }
    let absolute = offset_minutes.abs();
    let days = absolute / (24 * 60);
    let hours = absolute % (24 * 60) / 60;
    let minutes = absolute % 60;
    let mut parts = Vec::with_capacity(2);
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 && days == 0 {
        parts.push(format!("{minutes}m"));
    }
    format!(
        "{} {}",
        parts.join(" "),
        if offset_minutes < 0 { "ago" } else { "ahead" }
    )
}

fn range_label(moment: DateTime<Utc>, span: Span) -> String {
    match span {
        Span::Day => moment.format("%H:%M").to_string(),
        Span::Month => moment.format("%d %b").to_string(),
        Span::Year => moment.format("%b %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, LocalResult, TimeZone, Timelike, Utc};

    use super::{Message, Rate, Span, State, relative_label};

    fn fixed_state() -> State {
        let LocalResult::Single(anchor) = Utc.with_ymd_and_hms(2026, 8, 15, 12, 30, 45) else {
            unreachable!("the fixed test instant must be valid");
        };
        State::new(anchor)
    }

    #[test]
    fn request_tracks_the_scrubbed_utc_instant() {
        let mut state = fixed_state();
        assert!(state.update(Message::Scrubbed(90)));

        let request = state.chart_request();
        assert_eq!(request.local_time.year, 2026);
        assert_eq!(request.local_time.month, 8);
        assert_eq!(request.local_time.day, 15);
        assert_eq!(request.local_time.hour, 14);
        assert_eq!(request.local_time.minute, 0);
        assert!((request.local_time.second - 45.0).abs() < f64::EPSILON);
    }

    #[test]
    fn range_changes_clamp_without_changing_the_anchor() {
        let mut state = fixed_state();
        assert!(state.update(Message::Scrubbed(10 * 24 * 60)));
        assert!(state.update(Message::SpanChanged(Span::Day)));

        assert_eq!(state.offset_minutes, 12 * 60);
        assert_eq!(state.target().hour(), 0);
        assert_eq!(state.target().day(), 16);
    }

    #[test]
    fn playback_uses_the_selected_celestial_rate_and_stops_at_the_edge() {
        let mut state = fixed_state();
        state.span = Span::Day;
        state.rate = Rate::Hour;
        assert!(!state.update(Message::TogglePlayback));
        assert!(state.update(Message::Tick));
        assert_eq!(state.offset_minutes, 3);

        state.offset_minutes = state.span.radius_minutes() - 1;
        assert!(state.update(Message::Tick));
        assert_eq!(state.offset_minutes, state.span.radius_minutes());
        assert!(!state.playing);
    }

    #[test]
    fn relative_labels_distinguish_past_present_and_future() {
        assert_eq!(relative_label(-1500), "1d 1h ago");
        assert_eq!(relative_label(0), "At the present moment");
        assert_eq!(relative_label(135), "2h 15m ahead");
    }
}
