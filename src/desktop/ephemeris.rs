use std::path::PathBuf;

use chrono::{Datelike, NaiveDate, Utc};
use iced::widget::{button, column, row, rule, scrollable, text, text_input};
use iced::{Color, Element, Fill, Length, Task};

use crate::astro::{
    Calendar, CivilDateTime, EphemerisTable, SkyEvent, SkyEventKind, SkyEventSearch,
    SwissEphemerisProvider, TimeZoneSpec, civil_from_julian_day, resolve_moment,
};

#[derive(Debug, Clone)]
pub struct Output {
    pub table: EphemerisTable,
    pub events: Vec<SkyEvent>,
}

#[derive(Debug, Clone)]
pub struct State {
    pub start: String,
    pub days: String,
    pub step: String,
    pub output: Option<Output>,
    pub error: Option<String>,
    pub busy: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    StartChanged(String),
    DaysChanged(String),
    StepChanged(String),
    Calculate,
    Calculated(Result<Output, String>),
    Export,
    Exported(Result<PathBuf, String>),
}

impl Default for State {
    fn default() -> Self {
        Self {
            start: Utc::now().format("%Y-%m-%d").to_string(),
            days: "31".to_owned(),
            step: "1".to_owned(),
            output: None,
            error: None,
            busy: false,
            status: String::new(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, provider: &SwissEphemerisProvider) -> Task<Message> {
        self.error = None;
        match message {
            Message::StartChanged(value) => self.start = value,
            Message::DaysChanged(value) => self.days = digits(value),
            Message::StepChanged(value) => self.step = decimal(value),
            Message::Calculate => {
                if self.busy {
                    return Task::none();
                }
                let request = self.request();
                let provider = provider.clone();
                match request {
                    Ok((start, days, step)) => {
                        self.busy = true;
                        "Calculating…".clone_into(&mut self.status);
                        return Task::perform(
                            async move { calculate(provider, start, days, step) },
                            Message::Calculated,
                        );
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Calculated(result) => {
                self.busy = false;
                self.status.clear();
                match result {
                    Ok(output) => self.output = Some(output),
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Export => {
                if self.busy {
                    return Task::none();
                }
                let Some(output) = self.output.clone() else {
                    return Task::none();
                };
                self.busy = true;
                return Task::perform(export(output.table), Message::Exported);
            }
            Message::Exported(result) => {
                self.busy = false;
                match result {
                    Ok(path) => self.status = format!("Saved {}", path.display()),
                    Err(error) if error == "cancelled" => self.status.clear(),
                    Err(error) => self.error = Some(error),
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let controls = row![
            field(
                "Start (UTC)",
                text_input("YYYY-MM-DD", &self.start).on_input(Message::StartChanged)
            ),
            field(
                "Rows",
                text_input("31", &self.days).on_input(Message::DaysChanged)
            ),
            field(
                "Step (days)",
                text_input("1", &self.step).on_input(Message::StepChanged)
            ),
            button(if self.busy { "Working" } else { "Calculate" })
                .padding([8, 15])
                .style(button::primary)
                .on_press_maybe((!self.busy).then_some(Message::Calculate)),
            button("Export CSV")
                .padding([8, 12])
                .style(button::secondary)
                .on_press_maybe((self.output.is_some() && !self.busy).then_some(Message::Export)),
        ]
        .spacing(9)
        .align_y(iced::Center);

        let mut content = column![
            row![
                text("Ephemeris").size(23),
                iced::widget::space::horizontal(),
                text(&self.status).size(11).color(MUTED)
            ],
            controls,
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

    fn request(&self) -> Result<(f64, usize, f64), String> {
        let start = parse_date(&self.start)?;
        let days = self
            .days
            .parse::<usize>()
            .map_err(|_| "Rows must be a whole number".to_owned())?;
        let step = self
            .step
            .parse::<f64>()
            .map_err(|_| "Step must be a number".to_owned())?;
        if !(1..=5_000).contains(&days)
            || !step.is_finite()
            || !(1.0 / 24.0..=31.0).contains(&step)
            || step * days as f64 > 3_660.0
        {
            return Err("Interval is outside the supported range".to_owned());
        }
        Ok((start, days, step))
    }
}

fn output_view(output: &Output) -> Element<'_, Message> {
    let mut table = column![
        row![
            text("UTC")
                .size(10)
                .color(MUTED)
                .width(Length::Fixed(118.0)),
            text("SUN").size(10).color(MUTED).width(Fill),
            text("MOON").size(10).color(MUTED).width(Fill),
            text("MERCURY").size(10).color(MUTED).width(Fill),
            text("VENUS").size(10).color(MUTED).width(Fill),
            text("MARS").size(10).color(MUTED).width(Fill),
            text("JUPITER").size(10).color(MUTED).width(Fill),
            text("SATURN").size(10).color(MUTED).width(Fill),
        ]
        .spacing(6),
        rule::horizontal(1),
    ]
    .spacing(5);
    for row_data in &output.table.rows {
        let mut line = row![
            text(format_jd(row_data.jd_ut))
                .size(11)
                .width(Length::Fixed(118.0))
        ]
        .spacing(6);
        for position in &row_data.positions {
            line = line.push(
                text(format!(
                    "{:02}°{:02}′ {}{}",
                    position.degree_in_sign.floor() as u8,
                    ((position.degree_in_sign.fract() * 60.0).round() as u8).min(59),
                    position.sign.glyph(),
                    if position.retrograde { " R" } else { "" }
                ))
                .size(11)
                .width(Fill),
            );
        }
        table = table.push(line);
    }

    let mut events = column![text("EVENTS").size(10).color(MUTED)].spacing(5);
    for event in &output.events {
        events = events.push(
            row![
                text(format_jd(event.jd_ut))
                    .size(11)
                    .width(Length::Fixed(118.0)),
                text(event_label(&event.event)).size(12),
            ]
            .spacing(8),
        );
    }
    column![table, rule::horizontal(1), events]
        .spacing(14)
        .into()
}

fn calculate(
    provider: SwissEphemerisProvider,
    start: f64,
    days: usize,
    step: f64,
) -> Result<Output, String> {
    let table = EphemerisTable::calculate(&provider, start, days, step)
        .map_err(|error| error.to_string())?;
    let events = SkyEventSearch::new(provider)
        .search(start, start + step * days as f64)
        .map_err(|error| error.to_string())?;
    Ok(Output { table, events })
}

async fn export(table: EphemerisTable) -> Result<PathBuf, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Export ephemeris")
        .set_file_name("meridian-ephemeris.csv")
        .add_filter("CSV", &["csv"])
        .save_file()
        .await
        .ok_or_else(|| "cancelled".to_owned())?;
    let bytes = crate::render::ephemeris_csv(&table).map_err(|error| error.to_string())?;
    std::fs::write(file.path(), bytes).map_err(|error| error.to_string())?;
    Ok(file.path().to_path_buf())
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

fn event_label(event: &SkyEventKind) -> String {
    match event {
        SkyEventKind::Ingress { planet, sign } => format!("{planet} enters {sign}"),
        SkyEventKind::Station { planet, change } => format!("{planet} {change:?}"),
        SkyEventKind::Lunation { phase } => format!("{phase:?}"),
        SkyEventKind::SolarEclipse { eclipse } => format!("{eclipse:?} solar eclipse"),
        SkyEventKind::LunarEclipse { eclipse } => format!("{eclipse:?} lunar eclipse"),
    }
}

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(10).color(MUTED), input.into()]
        .spacing(4)
        .width(Length::FillPortion(1))
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

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ERROR: Color = Color::from_rgb(0.97, 0.44, 0.44);
