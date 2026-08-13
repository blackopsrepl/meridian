use chrono::{Datelike, Duration, NaiveDateTime, Timelike, Utc};
use iced::widget::{button, column, pick_list, row, rule, scrollable, text, text_input};
use iced::{Color, Element, Fill, Length, Task};

use crate::astro::{
    Calendar, Chart, ChartCalculator, CivilDateTime, Coordinates, ElectionRequest, ElectionSearch,
    ElectionSearchResult, ElectionTopic, TimeZoneSpec, TraditionalHouseSystem, resolve_moment,
};
use crate::locations::{City, CityIndex};

#[derive(Debug, Clone)]
pub struct State {
    title: String,
    start: String,
    end: String,
    step: String,
    topic: ElectionTopic,
    limit: String,
    city_query: String,
    city_results: Vec<City>,
    selected_city: Option<City>,
    location: String,
    latitude: String,
    longitude: String,
    elevation: String,
    house_system: TraditionalHouseSystem,
    result: Option<ElectionSearchResult>,
    error: Option<String>,
    busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    StartChanged(String),
    EndChanged(String),
    StepChanged(String),
    TopicChanged(ElectionTopic),
    LimitChanged(String),
    CityQueryChanged(String),
    CitySelected(u64),
    LocationChanged(String),
    LatitudeChanged(String),
    LongitudeChanged(String),
    ElevationChanged(String),
    HouseSystemChanged(TraditionalHouseSystem),
    Search,
    Searched(Result<ElectionSearchResult, String>),
    OpenCandidate(usize),
}

impl Default for State {
    fn default() -> Self {
        let start = Utc::now();
        Self {
            title: "Election".to_owned(),
            start: start.format("%Y-%m-%dT%H:00").to_string(),
            end: (start + Duration::days(7))
                .format("%Y-%m-%dT%H:00")
                .to_string(),
            step: "60".to_owned(),
            topic: ElectionTopic::General,
            limit: "10".to_owned(),
            city_query: String::new(),
            city_results: Vec::new(),
            selected_city: None,
            location: "Greenwich".to_owned(),
            latitude: "51.4779".to_owned(),
            longitude: "0".to_owned(),
            elevation: "46".to_owned(),
            house_system: TraditionalHouseSystem::Regiomontanus,
            result: None,
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
        cities: &CityIndex,
    ) -> Task<Message> {
        self.error = None;
        match message {
            Message::TitleChanged(value) => self.title = value,
            Message::StartChanged(value) => self.start = value,
            Message::EndChanged(value) => self.end = value,
            Message::StepChanged(value) => self.step = digits(value),
            Message::TopicChanged(value) => self.topic = value,
            Message::LimitChanged(value) => self.limit = digits(value),
            Message::CityQueryChanged(value) => {
                self.city_query = value;
                self.selected_city = None;
                self.city_results = cities.search(&self.city_query, 8);
            }
            Message::CitySelected(id) => {
                if let Some(city) = cities.get(id).cloned() {
                    self.city_query.clone_from(&city.display_name);
                    self.location.clone_from(&city.display_name);
                    self.latitude = city.latitude.to_string();
                    self.longitude = city.longitude.to_string();
                    self.elevation = city.elevation_m.to_string();
                    self.selected_city = Some(city);
                    self.city_results.clear();
                }
            }
            Message::LocationChanged(value) => self.location = value,
            Message::LatitudeChanged(value) => self.latitude = signed_decimal(value),
            Message::LongitudeChanged(value) => self.longitude = signed_decimal(value),
            Message::ElevationChanged(value) => self.elevation = signed_decimal(value),
            Message::HouseSystemChanged(value) => self.house_system = value,
            Message::Search => {
                if self.busy {
                    return Task::none();
                }
                match self.request() {
                    Ok(request) => {
                        self.busy = true;
                        let calculator = calculator.clone();
                        return Task::perform(
                            async move {
                                ElectionSearch::new(calculator)
                                    .search(request)
                                    .map_err(|error| error.to_string())
                            },
                            Message::Searched,
                        );
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Searched(result) => {
                self.busy = false;
                match result {
                    Ok(result) => self.result = Some(result),
                    Err(error) => self.error = Some(error),
                }
            }
            Message::OpenCandidate(_) => {}
        }
        Task::none()
    }

    pub fn candidate(&self, index: usize) -> Option<&Chart> {
        self.result
            .as_ref()
            .and_then(|result| result.candidates.get(index))
            .map(|candidate| &candidate.chart)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let top = row![
            field(
                "Name",
                text_input("Election", &self.title).on_input(Message::TitleChanged)
            ),
            field(
                "Topic",
                pick_list(
                    &ElectionTopic::ALL[..],
                    Some(self.topic),
                    Message::TopicChanged
                )
                .width(Fill)
            ),
            field(
                "Start (UTC)",
                text_input("YYYY-MM-DDTHH:MM", &self.start).on_input(Message::StartChanged)
            ),
            field(
                "End (UTC)",
                text_input("YYYY-MM-DDTHH:MM", &self.end).on_input(Message::EndChanged)
            ),
        ]
        .spacing(8);
        let range = row![
            field(
                "Step (minutes)",
                text_input("60", &self.step).on_input(Message::StepChanged)
            ),
            field(
                "Results",
                text_input("10", &self.limit).on_input(Message::LimitChanged)
            ),
            field(
                "House system",
                pick_list(
                    &TraditionalHouseSystem::ALL[..],
                    Some(self.house_system),
                    Message::HouseSystemChanged
                )
                .width(Fill)
            ),
            button(if self.busy { "Searching" } else { "Search" })
                .style(button::primary)
                .padding([8, 18])
                .on_press_maybe((!self.busy).then_some(Message::Search)),
        ]
        .spacing(8)
        .align_y(iced::Center);

        let mut city_search = column![field(
            "City",
            text_input("City name", &self.city_query).on_input(Message::CityQueryChanged)
        )]
        .spacing(4)
        .width(Length::FillPortion(2));
        for city in &self.city_results {
            city_search = city_search.push(
                button(text(&city.display_name).size(12))
                    .width(Fill)
                    .style(button::text)
                    .on_press(Message::CitySelected(city.id)),
            );
        }
        let place = row![
            city_search,
            field(
                "Location",
                text_input("Location", &self.location).on_input(Message::LocationChanged)
            ),
            field(
                "Latitude",
                text_input("Latitude", &self.latitude).on_input(Message::LatitudeChanged)
            ),
            field(
                "Longitude",
                text_input("Longitude", &self.longitude).on_input(Message::LongitudeChanged)
            ),
            field(
                "Elevation (m)",
                text_input("0", &self.elevation).on_input(Message::ElevationChanged)
            ),
        ]
        .spacing(8);

        let mut content = column![
            text("Elections").size(23),
            top,
            range,
            place,
            rule::horizontal(1),
        ]
        .spacing(12)
        .padding(18);
        if let Some(error) = &self.error {
            content = content.push(text(error).size(13).color(ERROR));
        }
        if let Some(result) = &self.result {
            content = content.push(result_view(result));
        }
        scrollable(content).height(Fill).into()
    }

    fn request(&self) -> Result<ElectionRequest, String> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err("Name is required".to_owned());
        }
        let coordinates = Coordinates {
            latitude: self
                .latitude
                .parse()
                .map_err(|_| "Latitude must be a number")?,
            longitude: self
                .longitude
                .parse()
                .map_err(|_| "Longitude must be a number")?,
            elevation_m: self
                .elevation
                .parse()
                .map_err(|_| "Elevation must be a number")?,
        };
        coordinates.validate().map_err(str::to_owned)?;
        Ok(ElectionRequest {
            title: title.to_owned(),
            start_jd_ut: parse_datetime(&self.start)?,
            end_jd_ut: parse_datetime(&self.end)?,
            step_minutes: self
                .step
                .parse()
                .map_err(|_| "Step must be a whole number".to_owned())?,
            location_name: self.location.trim().to_owned(),
            coordinates,
            house_system: self.house_system,
            topic: self.topic,
            limit: self
                .limit
                .parse()
                .map_err(|_| "Results must be a whole number".to_owned())?,
        })
    }
}

fn result_view(result: &ElectionSearchResult) -> Element<'_, Message> {
    let mut content = column![
        text(format!("{} instants evaluated", result.evaluated_instants))
            .size(11)
            .color(MUTED)
    ]
    .spacing(8);
    for (index, candidate) in result.candidates.iter().enumerate() {
        let moment = &candidate.chart.request.local_time;
        let heading = row![
            text(format!("#{}", candidate.rank))
                .size(15)
                .width(Length::Fixed(38.0)),
            text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02} UTC",
                moment.year, moment.month, moment.day, moment.hour, moment.minute
            ))
            .size(14)
            .width(Length::Fixed(190.0)),
            text(format!("Score {:+}", candidate.score))
                .size(14)
                .width(Length::Fixed(90.0)),
            button("Open")
                .style(button::secondary)
                .on_press(Message::OpenCandidate(index)),
        ]
        .spacing(8)
        .align_y(iced::Center);
        let mut details = column![heading].spacing(4);
        for item in &candidate.score_items {
            details = details.push(
                row![
                    text(format!("{:+}", item.score))
                        .size(11)
                        .width(Length::Fixed(30.0)),
                    text(&item.label).size(11).width(Length::Fixed(170.0)),
                    text(&item.rationale).size(11).color(MUTED),
                ]
                .spacing(6),
            );
        }
        for advisory in &candidate.advisories {
            details = details.push(text(advisory).size(11).color(WARNING));
        }
        content = content.push(details).push(rule::horizontal(1));
    }
    content.into()
}

fn parse_datetime(value: &str) -> Result<f64, String> {
    let value = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| "Date-time must use YYYY-MM-DDTHH:MM".to_owned())?;
    resolve_moment(
        &CivilDateTime {
            year: value.year(),
            month: value.month() as u8,
            day: value.day() as u8,
            hour: value.hour() as u8,
            minute: value.minute() as u8,
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

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(10).color(MUTED), input.into()]
        .spacing(4)
        .width(Length::FillPortion(1))
        .into()
}

fn digits(value: String) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
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
const WARNING: Color = Color::from_rgb(0.96, 0.71, 0.35);
