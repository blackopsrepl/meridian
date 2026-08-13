use chrono::{Datelike, Duration, NaiveDate, Utc};
use iced::widget::{button, column, pick_list, row, rule, scrollable, text, text_input};
use iced::{Color, Element, Fill, Length, Task};

use crate::astro::{
    AnnualProfection, Calendar, Chart, ChartCalculator, CivilDateTime, FirdariaPeriod, Planet,
    PlanetaryHours, TechniqueChart, TimeZoneSpec, TimingCalculator, TransitEvent,
    civil_from_julian_day, resolve_moment,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Technique {
    #[default]
    Transits,
    SecondaryProgressions,
    SolarArc,
    Harmonic,
    AnnualProfection,
    Firdaria,
    SolarReturn,
    LunarReturn,
    PlanetaryHours,
}

impl Technique {
    const ALL: [Self; 9] = [
        Self::Transits,
        Self::SecondaryProgressions,
        Self::SolarArc,
        Self::Harmonic,
        Self::AnnualProfection,
        Self::Firdaria,
        Self::SolarReturn,
        Self::LunarReturn,
        Self::PlanetaryHours,
    ];
}

impl std::fmt::Display for Technique {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Transits => "Transits",
            Self::SecondaryProgressions => "Secondary progressions",
            Self::SolarArc => "Solar arc",
            Self::Harmonic => "Harmonic",
            Self::AnnualProfection => "Annual profection",
            Self::Firdaria => "Firdaria",
            Self::SolarReturn => "Solar return",
            Self::LunarReturn => "Lunar return",
            Self::PlanetaryHours => "Planetary hours",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Transits(Vec<TransitEvent>),
    Technique(TechniqueChart),
    Profection(AnnualProfection),
    Firdaria(FirdariaPeriod),
    Return(Box<Chart>),
    Hours(PlanetaryHours),
}

#[derive(Debug, Clone)]
pub struct State {
    pub technique: Technique,
    pub target: String,
    pub end: String,
    pub age: String,
    pub harmonic: String,
    pub location: String,
    pub latitude: String,
    pub longitude: String,
    pub elevation: String,
    pub output: Option<Output>,
    pub error: Option<String>,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TechniqueChanged(Technique),
    TargetChanged(String),
    EndChanged(String),
    AgeChanged(String),
    HarmonicChanged(String),
    LocationChanged(String),
    LatitudeChanged(String),
    LongitudeChanged(String),
    ElevationChanged(String),
    Calculate,
    Calculated(Result<Output, String>),
}

impl Default for State {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            technique: Technique::Transits,
            target: today.format("%Y-%m-%d").to_string(),
            end: (today + Duration::days(90)).format("%Y-%m-%d").to_string(),
            age: "30".to_owned(),
            harmonic: "9".to_owned(),
            location: String::new(),
            latitude: String::new(),
            longitude: String::new(),
            elevation: String::new(),
            output: None,
            error: None,
            busy: false,
        }
    }
}

impl State {
    pub fn update(
        &mut self,
        message: Message,
        calculator: &ChartCalculator,
        chart: Option<&Chart>,
    ) -> Task<Message> {
        self.error = None;
        match message {
            Message::TechniqueChanged(value) => {
                self.technique = value;
                self.output = None;
            }
            Message::TargetChanged(value) => self.target = value,
            Message::EndChanged(value) => self.end = value,
            Message::AgeChanged(value) => self.age = decimal(value),
            Message::HarmonicChanged(value) => self.harmonic = digits(value),
            Message::LocationChanged(value) => self.location = value,
            Message::LatitudeChanged(value) => self.latitude = signed_decimal(value),
            Message::LongitudeChanged(value) => self.longitude = signed_decimal(value),
            Message::ElevationChanged(value) => self.elevation = signed_decimal(value),
            Message::Calculate => {
                let Some(chart) = chart.cloned() else {
                    self.error = Some("Open a chart file first".to_owned());
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                let request = Request::from_state(self, &chart);
                match request {
                    Ok(request) => {
                        self.busy = true;
                        let calculator = calculator.clone();
                        return Task::perform(
                            async move { calculate(calculator, chart, request) },
                            Message::Calculated,
                        );
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Calculated(result) => {
                self.busy = false;
                match result {
                    Ok(output) => self.output = Some(output),
                    Err(error) => self.error = Some(error),
                }
            }
        }
        Task::none()
    }

    pub fn view(&self, chart: Option<&Chart>) -> Element<'_, Message> {
        let chart_name = chart.map_or_else(
            || "No chart file".to_owned(),
            |chart| chart.request.title.clone(),
        );
        let mut controls = row![
            field(
                "Technique",
                pick_list(
                    &Technique::ALL[..],
                    Some(self.technique),
                    Message::TechniqueChanged
                )
                .width(Fill)
            ),
            field(
                "Target (UTC)",
                text_input("YYYY-MM-DD", &self.target).on_input(Message::TargetChanged)
            ),
        ]
        .spacing(9);
        match self.technique {
            Technique::Transits => {
                controls = controls.push(field(
                    "End (UTC)",
                    text_input("YYYY-MM-DD", &self.end).on_input(Message::EndChanged),
                ));
            }
            Technique::Harmonic => {
                controls = controls.push(field(
                    "Harmonic",
                    text_input("9", &self.harmonic).on_input(Message::HarmonicChanged),
                ));
            }
            Technique::AnnualProfection | Technique::Firdaria => {
                controls = controls.push(field(
                    "Age",
                    text_input("30", &self.age).on_input(Message::AgeChanged),
                ));
            }
            _ => {}
        }
        controls = controls.push(
            button(if self.busy { "Working" } else { "Calculate" })
                .style(button::primary)
                .padding([8, 15])
                .on_press_maybe((chart.is_some() && !self.busy).then_some(Message::Calculate)),
        );

        let location = if matches!(
            self.technique,
            Technique::SolarReturn | Technique::LunarReturn | Technique::PlanetaryHours
        ) {
            column![
                rule::horizontal(1),
                row![
                    field(
                        "Location",
                        text_input("Chart location", &self.location)
                            .on_input(Message::LocationChanged)
                    ),
                    field(
                        "Latitude",
                        text_input("Chart latitude", &self.latitude)
                            .on_input(Message::LatitudeChanged)
                    ),
                    field(
                        "Longitude",
                        text_input("Chart longitude", &self.longitude)
                            .on_input(Message::LongitudeChanged)
                    ),
                    field(
                        "Elevation (m)",
                        text_input("Chart elevation", &self.elevation)
                            .on_input(Message::ElevationChanged)
                    ),
                ]
                .spacing(9),
            ]
            .spacing(9)
        } else {
            column![]
        };

        let mut content = column![
            row![
                text("Timing").size(23),
                iced::widget::space::horizontal(),
                text(chart_name).size(12).color(MUTED)
            ],
            controls,
            location,
            rule::horizontal(1),
        ]
        .spacing(12)
        .padding(18);
        if let Some(error) = &self.error {
            content = content.push(text(error).size(13).color(ERROR));
        }
        if let Some(output) = &self.output {
            content = content.push(output_view(output));
        }
        scrollable(content).height(Fill).into()
    }
}

#[derive(Debug, Clone)]
struct Request {
    technique: Technique,
    target: f64,
    end: f64,
    age: f64,
    harmonic: u16,
    location: String,
    coordinates: crate::astro::Coordinates,
}

impl Request {
    fn from_state(state: &State, chart: &Chart) -> Result<Self, String> {
        let target = parse_date(&state.target)?;
        let end = parse_date(&state.end)?;
        let age = state
            .age
            .parse::<f64>()
            .map_err(|_| "Age must be a number".to_owned())?;
        if !age.is_finite() || !(0.0..=140.0).contains(&age) {
            return Err("Age must be between 0 and 140".to_owned());
        }
        let harmonic = state
            .harmonic
            .parse::<u16>()
            .map_err(|_| "Harmonic must be a whole number".to_owned())?;
        if !(1..=360).contains(&harmonic) {
            return Err("Harmonic must be between 1 and 360".to_owned());
        }
        let mut coordinates = chart.request.coordinates;
        if !state.latitude.trim().is_empty() || !state.longitude.trim().is_empty() {
            coordinates.latitude = state
                .latitude
                .parse()
                .map_err(|_| "Latitude must be a number".to_owned())?;
            coordinates.longitude = state
                .longitude
                .parse()
                .map_err(|_| "Longitude must be a number".to_owned())?;
        }
        if !state.elevation.trim().is_empty() {
            coordinates.elevation_m = state
                .elevation
                .parse()
                .map_err(|_| "Elevation must be a number".to_owned())?;
        }
        coordinates.validate().map_err(str::to_owned)?;
        Ok(Self {
            technique: state.technique,
            target,
            end,
            age,
            harmonic,
            location: if state.location.trim().is_empty() {
                chart.request.location_name.clone()
            } else {
                state.location.trim().to_owned()
            },
            coordinates,
        })
    }
}

fn calculate(
    chart_calculator: ChartCalculator,
    natal: Chart,
    request: Request,
) -> Result<Output, String> {
    let timing = TimingCalculator::from_chart_calculator(&chart_calculator);
    match request.technique {
        Technique::Transits => timing
            .transits(&natal, request.target, request.end)
            .map(Output::Transits)
            .map_err(|error| error.to_string()),
        Technique::SecondaryProgressions => timing
            .secondary_progressions(&natal, request.target)
            .map(Output::Technique)
            .map_err(|error| error.to_string()),
        Technique::SolarArc => timing
            .solar_arc(&natal, request.target)
            .map(Output::Technique)
            .map_err(|error| error.to_string()),
        Technique::Harmonic => timing
            .harmonic(&natal, request.harmonic)
            .map(Output::Technique)
            .map_err(|error| error.to_string()),
        Technique::AnnualProfection => Ok(Output::Profection(AnnualProfection::at_age(
            &natal,
            request.age as u32,
        ))),
        Technique::Firdaria => Ok(Output::Firdaria(FirdariaPeriod::at_age(
            natal.sect,
            request.age,
        ))),
        Technique::SolarReturn | Technique::LunarReturn => {
            let (planet, days, label) = if request.technique == Technique::SolarReturn {
                (Planet::Sun, 370.0, "Solar return")
            } else {
                (Planet::Moon, 35.0, "Lunar return")
            };
            timing
                .return_chart(
                    &chart_calculator,
                    &natal,
                    planet,
                    request.target,
                    request.target + days,
                    format!("{} · {label}", natal.request.title),
                    request.location,
                    request.coordinates,
                    natal.request.house_system,
                )
                .map(|chart| Output::Return(Box::new(chart)))
                .map_err(|error| error.to_string())
        }
        Technique::PlanetaryHours => timing
            .planetary_hours(request.target, request.coordinates)
            .map(Output::Hours)
            .map_err(|error| error.to_string()),
    }
}

fn output_view(output: &Output) -> Element<'_, Message> {
    match output {
        Output::Transits(events) => {
            let mut rows = column![
                text(format!("{} exact contacts", events.len()))
                    .size(12)
                    .color(MUTED)
            ]
            .spacing(5);
            for event in events {
                rows = rows.push(row![
                    text(format_jd(event.exact_jd_ut))
                        .size(11)
                        .width(Length::Fixed(112.0)),
                    text(format!(
                        "{} {} {}{}",
                        event.transiting,
                        event.aspect.glyph(),
                        event.target.name(),
                        if event.retrograde { "  R" } else { "" }
                    ))
                    .size(12),
                ]);
            }
            rows.into()
        }
        Output::Technique(chart) => {
            let mut rows = column![text(&chart.title).size(17)].spacing(5);
            for position in &chart.positions {
                rows = rows.push(
                    text(format!(
                        "{}  {:02}°{:02}′ {}  natal H{}{}",
                        position.planet.glyph(),
                        position.degree_in_sign.floor() as u8,
                        ((position.degree_in_sign.fract() * 60.0).round() as u8).min(59),
                        position.sign,
                        position.natal_house,
                        if position.retrograde { "  R" } else { "" }
                    ))
                    .size(12),
                );
            }
            rows = rows.push(rule::horizontal(1));
            for contact in &chart.contacts {
                rows = rows.push(
                    text(format!(
                        "{} {} {}  {:.2}°",
                        contact.moving,
                        contact.aspect.glyph(),
                        contact.natal,
                        contact.orb
                    ))
                    .size(12),
                );
            }
            rows.into()
        }
        Output::Profection(value) => column![
            text(format!("Age {}", value.age)).size(18),
            key_value("House", value.activated_house.to_string()),
            key_value("Sign", value.activated_sign.name()),
            key_value("Lord of year", value.lord_of_year.name()),
        ]
        .spacing(8)
        .into(),
        Output::Firdaria(value) => column![
            text(format!("{} firdaria", value.sect)).size(18),
            key_value("Major lord", value.major_lord.name()),
            key_value("Sub-lord", value.sub_lord.name()),
            key_value(
                "Major period",
                format!(
                    "{:.2}–{:.2}",
                    value.major_started_at_age, value.major_ends_at_age
                )
            ),
            key_value(
                "Sub-period",
                format!(
                    "{:.2}–{:.2}",
                    value.sub_started_at_age, value.sub_ends_at_age
                )
            ),
        ]
        .spacing(8)
        .into(),
        Output::Return(chart) => {
            let mut rows = column![
                text(&chart.request.title).size(18),
                text(format_jd(chart.moment.jd_ut)).size(12).color(MUTED)
            ]
            .spacing(6);
            for position in &chart.positions {
                rows = rows.push(
                    text(format!(
                        "{}  {:02}°{:02}′ {}  H{}",
                        position.planet.glyph(),
                        position.degree_in_sign.floor() as u8,
                        ((position.degree_in_sign.fract() * 60.0).round() as u8).min(59),
                        position.sign,
                        position.house
                    ))
                    .size(12),
                );
            }
            rows.into()
        }
        Output::Hours(value) => {
            let mut rows = column![
                key_value("Day ruler", value.day_ruler.name()),
                key_value("Sunrise", format_jd(value.sunrise_jd_ut)),
                key_value("Sunset", format_jd(value.sunset_jd_ut)),
            ]
            .spacing(6);
            for hour in &value.hours {
                rows = rows.push(
                    text(format!(
                        "{:02}  {}  {}–{}{}",
                        hour.number,
                        hour.ruler,
                        format_time(hour.starts_jd_ut),
                        format_time(hour.ends_jd_ut),
                        if hour.is_daylight { "  day" } else { "  night" }
                    ))
                    .size(12),
                );
            }
            rows.into()
        }
    }
}

fn parse_date(value: &str) -> Result<f64, String> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Date must use YYYY-MM-DD".to_owned())?;
    resolve_moment(
        &CivilDateTime {
            year: date.year(),
            month: date.month() as u8,
            day: date.day() as u8,
            hour: 0,
            minute: 0,
            second: 0.0,
            calendar: Calendar::Gregorian,
        },
        &TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC".to_owned()),
        },
    )
    .map(|moment| moment.jd_ut)
    .map_err(|error| error.to_string())
}

fn format_jd(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        value.year, value.month, value.day, value.hour, value.minute
    )
}

fn format_time(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!("{:02}:{:02}", value.hour, value.minute)
}

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(10).color(MUTED), input.into()]
        .spacing(4)
        .width(Length::FillPortion(1))
        .into()
}

fn key_value(label: &str, value: impl Into<String>) -> Element<'_, Message> {
    row![
        text(label)
            .size(12)
            .color(MUTED)
            .width(Length::Fixed(120.0)),
        text(value.into()).size(12),
    ]
    .spacing(8)
    .into()
}

fn digits(value: String) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

fn decimal(value: String) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '.')
        .collect()
}

fn signed_decimal(value: String) -> String {
    value
        .chars()
        .enumerate()
        .filter(|(index, character)| {
            character.is_ascii_digit() || *character == '.' || (*index == 0 && *character == '-')
        })
        .map(|(_, character)| character)
        .collect()
}

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ERROR: Color = Color::from_rgb(0.97, 0.44, 0.44);
