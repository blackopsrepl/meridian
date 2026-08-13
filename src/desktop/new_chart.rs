use chrono::{Datelike, Local, Timelike};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, scrollable, text, text_input,
};
use iced::{Color, Element, Fill, Length};

use crate::astro::{
    Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates, OrbPolicy, TimeZoneSpec,
    TraditionalHouseSystem,
};
use crate::locations::{City, CityIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoneMode {
    #[default]
    Iana,
    Fixed,
}

impl ZoneMode {
    const ALL: [Self; 2] = [Self::Iana, Self::Fixed];
}

impl std::fmt::Display for ZoneMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Iana => "IANA time zone",
            Self::Fixed => "Fixed UTC offset",
        })
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub title: String,
    pub purpose: ChartPurpose,
    pub year: String,
    pub month: String,
    pub day: String,
    pub time: String,
    pub calendar: Calendar,
    pub city_query: String,
    pub city_results: Vec<City>,
    pub selected_city: Option<City>,
    pub manual_coordinates: bool,
    pub location_name: String,
    pub latitude: String,
    pub longitude: String,
    pub elevation: String,
    pub manual_timezone: bool,
    pub zone_mode: ZoneMode,
    pub timezone: String,
    pub fixed_offset: String,
    pub fold: Option<u8>,
    pub house_system: TraditionalHouseSystem,
    pub advanced_open: bool,
    pub orb_conjunction: String,
    pub orb_sextile: String,
    pub orb_square: String,
    pub orb_trine: String,
    pub orb_opposition: String,
    pub orb_luminary_bonus: String,
    pub orb_angle: String,
    pub orb_lot: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    PurposeChanged(ChartPurpose),
    YearChanged(String),
    MonthChanged(String),
    DayChanged(String),
    TimeChanged(String),
    CalendarChanged(Calendar),
    CityQueryChanged(String),
    CitySelected(u64),
    ManualCoordinatesChanged(bool),
    LocationChanged(String),
    LatitudeChanged(String),
    LongitudeChanged(String),
    ElevationChanged(String),
    ManualTimezoneChanged(bool),
    ZoneModeChanged(ZoneMode),
    TimezoneChanged(String),
    FixedOffsetChanged(String),
    FoldChanged(Option<u8>),
    HouseSystemChanged(TraditionalHouseSystem),
    AdvancedToggled,
    OrbConjunctionChanged(String),
    OrbSextileChanged(String),
    OrbSquareChanged(String),
    OrbTrineChanged(String),
    OrbOppositionChanged(String),
    OrbLuminaryBonusChanged(String),
    OrbAngleChanged(String),
    OrbLotChanged(String),
    Calculate,
}

impl Default for State {
    fn default() -> Self {
        let now = Local::now();
        Self {
            title: String::new(),
            purpose: ChartPurpose::Natal,
            year: now.year().to_string(),
            month: now.month().to_string(),
            day: now.day().to_string(),
            time: format!("{:02}:{:02}", now.hour(), now.minute()),
            calendar: Calendar::Gregorian,
            city_query: String::new(),
            city_results: Vec::new(),
            selected_city: None,
            manual_coordinates: false,
            location_name: String::new(),
            latitude: String::new(),
            longitude: String::new(),
            elevation: "0".to_owned(),
            manual_timezone: false,
            zone_mode: ZoneMode::Iana,
            timezone: String::new(),
            fixed_offset: "0".to_owned(),
            fold: None,
            house_system: TraditionalHouseSystem::WholeSign,
            advanced_open: false,
            orb_conjunction: "8".to_owned(),
            orb_sextile: "5".to_owned(),
            orb_square: "7".to_owned(),
            orb_trine: "7".to_owned(),
            orb_opposition: "8".to_owned(),
            orb_luminary_bonus: "2".to_owned(),
            orb_angle: "5".to_owned(),
            orb_lot: "3".to_owned(),
            error: None,
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, cities: &CityIndex) {
        self.error = None;
        match message {
            Message::TitleChanged(value) => self.title = value,
            Message::PurposeChanged(value) => self.purpose = value,
            Message::YearChanged(value) => self.year = numeric(value, true),
            Message::MonthChanged(value) => self.month = numeric(value, false),
            Message::DayChanged(value) => self.day = numeric(value, false),
            Message::TimeChanged(value) => self.time = value,
            Message::CalendarChanged(value) => self.calendar = value,
            Message::CityQueryChanged(value) => {
                self.city_query = value;
                self.selected_city = None;
                self.city_results = cities.search(&self.city_query, 8);
            }
            Message::CitySelected(id) => {
                if let Some(city) = cities.get(id).cloned() {
                    self.city_query.clone_from(&city.display_name);
                    self.location_name.clone_from(&city.display_name);
                    self.latitude = city.latitude.to_string();
                    self.longitude = city.longitude.to_string();
                    self.elevation = city.elevation_m.to_string();
                    self.timezone.clone_from(&city.timezone);
                    self.selected_city = Some(city);
                    self.city_results.clear();
                }
            }
            Message::ManualCoordinatesChanged(value) => self.manual_coordinates = value,
            Message::LocationChanged(value) => self.location_name = value,
            Message::LatitudeChanged(value) => self.latitude = decimal(value),
            Message::LongitudeChanged(value) => self.longitude = decimal(value),
            Message::ElevationChanged(value) => self.elevation = decimal(value),
            Message::ManualTimezoneChanged(value) => self.manual_timezone = value,
            Message::ZoneModeChanged(value) => self.zone_mode = value,
            Message::TimezoneChanged(value) => self.timezone = value,
            Message::FixedOffsetChanged(value) => self.fixed_offset = numeric(value, true),
            Message::FoldChanged(value) => self.fold = value,
            Message::HouseSystemChanged(value) => self.house_system = value,
            Message::AdvancedToggled => self.advanced_open = !self.advanced_open,
            Message::OrbConjunctionChanged(value) => self.orb_conjunction = decimal(value),
            Message::OrbSextileChanged(value) => self.orb_sextile = decimal(value),
            Message::OrbSquareChanged(value) => self.orb_square = decimal(value),
            Message::OrbTrineChanged(value) => self.orb_trine = decimal(value),
            Message::OrbOppositionChanged(value) => self.orb_opposition = decimal(value),
            Message::OrbLuminaryBonusChanged(value) => self.orb_luminary_bonus = decimal(value),
            Message::OrbAngleChanged(value) => self.orb_angle = decimal(value),
            Message::OrbLotChanged(value) => self.orb_lot = decimal(value),
            Message::Calculate => {}
        }
    }

    pub fn calculate(&self) -> Result<(ChartRequest, OrbPolicy), String> {
        let title = required(&self.title, "Chart name")?;
        let year = parse::<i32>(&self.year, "Year")?;
        let month = parse_range::<u8>(&self.month, "Month", 1, 12)?;
        let day = parse_range::<u8>(&self.day, "Day", 1, 31)?;
        let (hour, minute, second) = parse_time(&self.time)?;

        let selected_city = self.selected_city.as_ref();
        let coordinates = if self.manual_coordinates {
            Coordinates {
                latitude: parse(&self.latitude, "Latitude")?,
                longitude: parse(&self.longitude, "Longitude")?,
                elevation_m: parse(&self.elevation, "Elevation")?,
            }
        } else {
            let city = selected_city.ok_or_else(|| {
                "Choose a city from the search results or enable manual coordinates".to_owned()
            })?;
            Coordinates {
                latitude: city.latitude,
                longitude: city.longitude,
                elevation_m: city.elevation_m,
            }
        };
        coordinates.validate().map_err(str::to_owned)?;

        let location_name = if self.manual_coordinates {
            required(&self.location_name, "Location name")?
        } else {
            selected_city
                .map(|city| city.display_name.clone())
                .ok_or_else(|| "Choose a city from the search results".to_owned())?
        };

        let time_zone = if self.manual_timezone {
            match self.zone_mode {
                ZoneMode::Iana => TimeZoneSpec::Iana {
                    name: required(&self.timezone, "IANA time zone")?,
                    fold: self.fold,
                },
                ZoneMode::Fixed => TimeZoneSpec::FixedOffset {
                    minutes_east: parse_range::<i32>(
                        &self.fixed_offset,
                        "UTC offset",
                        -1_440,
                        1_440,
                    )?,
                    label: None,
                },
            }
        } else {
            let city = selected_city.ok_or_else(|| {
                "Choose a city or enable the manual time-zone override".to_owned()
            })?;
            TimeZoneSpec::Iana {
                name: city.timezone.clone(),
                fold: self.fold,
            }
        };

        Ok((
            ChartRequest {
                title,
                purpose: self.purpose,
                local_time: CivilDateTime {
                    year,
                    month,
                    day,
                    hour,
                    minute,
                    second,
                    calendar: self.calendar,
                },
                time_zone,
                location_name,
                coordinates,
                house_system: self.house_system,
            },
            OrbPolicy {
                conjunction: orb(&self.orb_conjunction, "Conjunction")?,
                sextile: orb(&self.orb_sextile, "Sextile")?,
                square: orb(&self.orb_square, "Square")?,
                trine: orb(&self.orb_trine, "Trine")?,
                opposition: orb(&self.orb_opposition, "Opposition")?,
                luminary_bonus: orb(&self.orb_luminary_bonus, "Luminary bonus")?,
                angle_orb: orb(&self.orb_angle, "Angle")?,
                lot_orb: orb(&self.orb_lot, "Lot")?,
            },
        ))
    }

    pub fn view(&self) -> Element<'_, Message> {
        let identity = group(
            "CHART",
            column![
                field(
                    "Name",
                    text_input("Name this chart", &self.title).on_input(Message::TitleChanged)
                ),
                row![
                    field(
                        "Purpose",
                        pick_list(
                            &ChartPurpose::ALL[..],
                            Some(self.purpose),
                            Message::PurposeChanged
                        )
                        .width(Fill)
                    ),
                    field(
                        "Calendar",
                        pick_list(
                            &Calendar::ALL[..],
                            Some(self.calendar),
                            Message::CalendarChanged
                        )
                        .width(Fill)
                    ),
                ]
                .spacing(10),
                row![
                    field(
                        "Year",
                        text_input("YYYY", &self.year).on_input(Message::YearChanged)
                    ),
                    field(
                        "Month",
                        text_input("MM", &self.month).on_input(Message::MonthChanged)
                    ),
                    field(
                        "Day",
                        text_input("DD", &self.day).on_input(Message::DayChanged)
                    ),
                    field(
                        "Time",
                        text_input("HH:MM", &self.time).on_input(Message::TimeChanged)
                    ),
                ]
                .spacing(8),
            ]
            .spacing(10),
        );

        let mut city_search = column![field(
            "City",
            text_input("City name", &self.city_query).on_input(Message::CityQueryChanged)
        )]
        .spacing(4);
        for city in &self.city_results {
            city_search = city_search.push(
                button(
                    column![
                        text(&city.display_name).size(13),
                        text(format!(
                            "{} · population {}",
                            city.timezone, city.population
                        ))
                        .size(11)
                        .color(MUTED),
                    ]
                    .spacing(2),
                )
                .width(Fill)
                .padding([7, 9])
                .style(button::text)
                .on_press(Message::CitySelected(city.id)),
            );
        }
        let coordinates = if self.manual_coordinates {
            column![
                field(
                    "Location name",
                    text_input("Location", &self.location_name).on_input(Message::LocationChanged)
                ),
                row![
                    field(
                        "Latitude",
                        text_input("-90 … 90", &self.latitude).on_input(Message::LatitudeChanged)
                    ),
                    field(
                        "Longitude",
                        text_input("-180 … 180", &self.longitude)
                            .on_input(Message::LongitudeChanged)
                    ),
                    field(
                        "Elevation (m)",
                        text_input("0", &self.elevation).on_input(Message::ElevationChanged)
                    ),
                ]
                .spacing(8),
            ]
            .spacing(8)
        } else {
            column![]
        };
        let place = group(
            "PLACE",
            column![
                city_search,
                checkbox(self.manual_coordinates)
                    .label("Enter coordinates manually")
                    .on_toggle(Message::ManualCoordinatesChanged),
                coordinates,
            ]
            .spacing(9),
        );

        let timezone_fields = if self.manual_timezone {
            let zone = match self.zone_mode {
                ZoneMode::Iana => column![
                    field(
                        "Time zone",
                        text_input("Europe/Rome", &self.timezone)
                            .on_input(Message::TimezoneChanged)
                    ),
                    row![
                        button("Automatic fold")
                            .style(if self.fold.is_none() {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .on_press(Message::FoldChanged(None)),
                        button("First occurrence")
                            .style(if self.fold == Some(0) {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .on_press(Message::FoldChanged(Some(0))),
                        button("Second occurrence")
                            .style(if self.fold == Some(1) {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .on_press(Message::FoldChanged(Some(1))),
                    ]
                    .spacing(6),
                ]
                .spacing(8),
                ZoneMode::Fixed => column![field(
                    "Minutes east of UTC",
                    text_input("60", &self.fixed_offset).on_input(Message::FixedOffsetChanged)
                )],
            };
            column![
                pick_list(
                    &ZoneMode::ALL[..],
                    Some(self.zone_mode),
                    Message::ZoneModeChanged
                )
                .width(Fill),
                zone,
            ]
            .spacing(8)
        } else {
            column![]
        };
        let settings = group(
            "SETTINGS",
            column![
                field(
                    "House system",
                    pick_list(
                        &TraditionalHouseSystem::ALL[..],
                        Some(self.house_system),
                        Message::HouseSystemChanged
                    )
                    .width(Fill)
                ),
                checkbox(self.manual_timezone)
                    .label("Override the city time zone")
                    .on_toggle(Message::ManualTimezoneChanged),
                timezone_fields,
            ]
            .spacing(9),
        );

        let advanced = if self.advanced_open {
            column![
                row![
                    orb_field(
                        "Conjunction",
                        &self.orb_conjunction,
                        Message::OrbConjunctionChanged
                    ),
                    orb_field("Sextile", &self.orb_sextile, Message::OrbSextileChanged),
                    orb_field("Square", &self.orb_square, Message::OrbSquareChanged),
                    orb_field("Trine", &self.orb_trine, Message::OrbTrineChanged),
                ]
                .spacing(8),
                row![
                    orb_field(
                        "Opposition",
                        &self.orb_opposition,
                        Message::OrbOppositionChanged
                    ),
                    orb_field(
                        "Luminary bonus",
                        &self.orb_luminary_bonus,
                        Message::OrbLuminaryBonusChanged
                    ),
                    orb_field("Angles", &self.orb_angle, Message::OrbAngleChanged),
                    orb_field("Lots", &self.orb_lot, Message::OrbLotChanged),
                ]
                .spacing(8),
            ]
            .spacing(8)
        } else {
            column![]
        };

        let mut content = column![
            row![
                text("New Chart").size(23),
                iced::widget::space::horizontal(),
                button("Calculate")
                    .padding([9, 18])
                    .style(button::primary)
                    .on_press(Message::Calculate),
            ]
            .align_y(iced::Center),
            rule::horizontal(1),
            row![
                column![identity, place]
                    .spacing(12)
                    .width(Length::FillPortion(3)),
                column![settings].spacing(12).width(Length::FillPortion(2)),
            ]
            .spacing(12),
            button(if self.advanced_open {
                "Hide aspect orbs"
            } else {
                "Aspect orbs"
            })
            .style(button::text)
            .on_press(Message::AdvancedToggled),
            advanced,
        ]
        .spacing(14)
        .padding(18);
        if let Some(error) = &self.error {
            content = content.push(
                container(text(error).size(13).color(Color::from_rgb8(248, 113, 113)))
                    .padding(10)
                    .width(Fill),
            );
        }
        scrollable(content).height(Fill).into()
    }
}

fn group<'a>(label: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(column![text(label).size(10).color(MUTED), content.into()].spacing(9))
        .padding(12)
        .width(Fill)
        .style(|_| {
            container::Style::default()
                .background(Color::from_rgb8(15, 20, 31))
                .border(iced::Border {
                    color: Color::from_rgb8(36, 45, 62),
                    width: 1.0,
                    radius: 6.0.into(),
                })
        })
        .into()
}

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(11).color(MUTED), input.into()]
        .spacing(4)
        .width(Fill)
        .into()
}

fn orb_field<'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    field(label, text_input("degrees", value).on_input(on_input))
}

fn required(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else if value.chars().count() > 160 {
        Err(format!("{label} is too long"))
    } else {
        Ok(value.to_owned())
    }
}

fn parse<T>(value: &str, label: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value
        .trim()
        .parse()
        .map_err(|_| format!("{label} is not a valid number"))
}

fn parse_range<T>(value: &str, label: &str, minimum: T, maximum: T) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + std::fmt::Display + Copy,
{
    let value = parse(value, label)?;
    if value < minimum || value > maximum {
        Err(format!("{label} must be between {minimum} and {maximum}"))
    } else {
        Ok(value)
    }
}

fn orb(value: &str, label: &str) -> Result<f64, String> {
    let value = parse::<f64>(value, label)?;
    if value.is_finite() && (0.0..=30.0).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{label} orb must be between 0 and 30 degrees"))
    }
}

fn parse_time(value: &str) -> Result<(u8, u8, f64), String> {
    let parts = value.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("Time must use HH:MM or HH:MM:SS".to_owned());
    }
    let hour = parse_range::<u8>(parts[0], "Hour", 0, 23)?;
    let minute = parse_range::<u8>(parts[1], "Minute", 0, 59)?;
    let second = if parts.len() == 3 {
        let second = parse::<f64>(parts[2], "Second")?;
        if !(0.0..60.0).contains(&second) {
            return Err("Second must be at least 0 and below 60".to_owned());
        }
        second
    } else {
        0.0
    };
    Ok((hour, minute, second))
}

fn numeric(value: String, signed: bool) -> String {
    value
        .chars()
        .enumerate()
        .filter(|(index, character)| {
            character.is_ascii_digit() || (signed && *index == 0 && *character == '-')
        })
        .map(|(_, character)| character)
        .collect()
}

fn decimal(value: String) -> String {
    value
        .chars()
        .enumerate()
        .filter(|(index, character)| {
            character.is_ascii_digit() || (*index == 0 && *character == '-') || *character == '.'
        })
        .map(|(_, character)| character)
        .collect()
}

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
