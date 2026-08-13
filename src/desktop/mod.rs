mod archive;
mod chart_canvas;
mod elections;
mod ephemeris;
mod new_chart;
mod relationships;
mod timing;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, Timelike, Utc};
use iced::keyboard;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{button, column, container, row, rule, scrollable, space, text};
use iced::{Center, Color, Element, Fill, Length, Size, Subscription, Task, Theme};

use crate::astro::{
    Calendar, Chart, ChartCalculator, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
    SwissEphemerisProvider, TimeZoneSpec, TraditionalHouseSystem,
};
use crate::document::{CHART_EXTENSION, ChartDocument};
use crate::locations::CityIndex;
use crate::store::Store;
use uuid::Uuid;

use chart_canvas::Inspection;

pub fn run() -> iced::Result {
    iced::application(Desktop::boot, Desktop::update, Desktop::view)
        .title(Desktop::window_title)
        .theme(Desktop::theme)
        .subscription(Desktop::subscription)
        .window(iced::window::Settings {
            size: Size::new(1360.0, 860.0),
            min_size: Some(Size::new(980.0, 640.0)),
            ..iced::window::Settings::default()
        })
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    CurrentSky,
    Archive,
    Chart,
    NewChart,
    Ephemeris,
    Timing,
    Relationships,
    Elections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Wheel,
    Data,
    Inspector,
}

#[derive(Debug, Clone, Copy)]
enum ChartExport {
    Svg,
    Csv,
}

#[derive(Debug)]
pub struct Desktop {
    calculator: Option<ChartCalculator>,
    cities: Option<CityIndex>,
    current_sky: Option<Chart>,
    document: Option<ChartDocument>,
    document_path: Option<PathBuf>,
    archive_id: Option<Uuid>,
    screen: Screen,
    panes: pane_grid::State<Pane>,
    inspection: Option<Inspection>,
    status: String,
    busy: bool,
    fatal_error: Option<String>,
    new_chart: new_chart::State,
    ephemeris: ephemeris::State,
    timing: timing::State,
    relationships: relationships::State,
    elections: elections::State,
    store: Option<Store>,
    archive: archive::State,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Screen),
    NewDocument,
    NewConfirmed(bool),
    OpenDocument,
    OpenConfirmed(bool),
    DocumentOpened(Result<(PathBuf, ChartDocument), String>),
    SaveDocument,
    SaveDocumentAs,
    DocumentSaved(Result<PathBuf, String>),
    CloseDocument,
    CloseConfirmed(bool),
    ExportChart(ChartExport),
    ChartExported(Result<PathBuf, String>),
    Inspect(Inspection),
    PaneResized(pane_grid::ResizeEvent),
    NewChart(new_chart::Message),
    Ephemeris(ephemeris::Message),
    Timing(timing::Message),
    Relationships(relationships::Message),
    Elections(elections::Message),
    Archive(archive::Message),
    ArchiveDeleteConfirmed(Uuid, bool),
}

impl Desktop {
    fn boot() -> (Self, Task<Message>) {
        let (mut panes, wheel) = pane_grid::State::new(Pane::Wheel);
        if let Some((right, split)) = panes.split(pane_grid::Axis::Vertical, wheel, Pane::Data) {
            panes.resize(split, 0.70);
            if let Some((_inspector, split)) =
                panes.split(pane_grid::Axis::Horizontal, right, Pane::Inspector)
            {
                panes.resize(split, 0.60);
            }
        }

        let mut desktop = Self {
            calculator: None,
            cities: None,
            current_sky: None,
            document: None,
            document_path: None,
            archive_id: None,
            screen: Screen::CurrentSky,
            panes,
            inspection: None,
            status: String::new(),
            busy: false,
            fatal_error: None,
            new_chart: new_chart::State::default(),
            ephemeris: ephemeris::State::default(),
            timing: timing::State::default(),
            relationships: relationships::State::default(),
            elections: elections::State::default(),
            store: None,
            archive: archive::State::default(),
        };

        match DesktopResources::resolve().and_then(DesktopResources::load) {
            Ok((calculator, cities, store)) => {
                match calculator.calculate(current_sky_request()) {
                    Ok(chart) => desktop.current_sky = Some(chart),
                    Err(error) => desktop.fatal_error = Some(error.to_string()),
                }
                desktop.calculator = Some(calculator);
                desktop.cities = Some(cities);
                desktop.archive.refresh(&store);
                desktop.store = Some(store);

                if let Some(path) = startup_document_path() {
                    match ChartDocument::open(&path) {
                        Ok(document) => {
                            desktop.document = Some(document);
                            desktop.document_path = Some(path);
                            desktop.screen = Screen::Chart;
                        }
                        Err(error) => desktop.status = error.to_string(),
                    }
                }
            }
            Err(error) => desktop.fatal_error = Some(format!("{error:#}")),
        }

        (desktop, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(screen) => {
                self.screen = screen;
                self.inspection = None;
                Task::none()
            }
            Message::NewDocument => {
                if self.has_unsaved_document() {
                    return Task::perform(confirm_discard(), Message::NewConfirmed);
                }
                self.start_new_document();
                Task::none()
            }
            Message::NewConfirmed(confirmed) => {
                if confirmed {
                    self.start_new_document();
                }
                Task::none()
            }
            Message::OpenDocument => {
                if self.has_unsaved_document() {
                    return Task::perform(confirm_discard(), Message::OpenConfirmed);
                }
                self.start_open_document()
            }
            Message::OpenConfirmed(confirmed) => {
                if confirmed {
                    self.start_open_document()
                } else {
                    Task::none()
                }
            }
            Message::DocumentOpened(result) => {
                self.busy = false;
                match result {
                    Ok((path, document)) => {
                        self.document = Some(document);
                        self.document_path = Some(path.clone());
                        self.archive_id = None;
                        self.screen = Screen::Chart;
                        self.inspection = None;
                        self.status = format!("Opened {}", display_name(&path));
                    }
                    Err(error) if error == "cancelled" => self.status.clear(),
                    Err(error) => self.status = error,
                }
                Task::none()
            }
            Message::SaveDocument => {
                let Some(document) = self.document.clone() else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                "Saving chart…".clone_into(&mut self.status);
                Task::perform(
                    save_document(document, self.document_path.clone(), false),
                    Message::DocumentSaved,
                )
            }
            Message::SaveDocumentAs => {
                let Some(document) = self.document.clone() else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                "Choose a chart file…".clone_into(&mut self.status);
                Task::perform(
                    save_document(document, self.document_path.clone(), true),
                    Message::DocumentSaved,
                )
            }
            Message::DocumentSaved(result) => {
                self.busy = false;
                match result {
                    Ok(path) => {
                        self.document_path = Some(path.clone());
                        self.status = format!("Saved {}", display_name(&path));
                    }
                    Err(error) if error == "cancelled" => self.status.clear(),
                    Err(error) => self.status = error,
                }
                Task::none()
            }
            Message::CloseDocument => {
                if self.has_unsaved_document() {
                    Task::perform(confirm_discard(), Message::CloseConfirmed)
                } else {
                    self.close_document();
                    Task::none()
                }
            }
            Message::CloseConfirmed(confirmed) => {
                if confirmed {
                    self.close_document();
                }
                Task::none()
            }
            Message::ExportChart(kind) => {
                let Some(chart) = self.active_chart().cloned() else {
                    return Task::none();
                };
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                Task::perform(export_chart(chart, kind), Message::ChartExported)
            }
            Message::ChartExported(result) => {
                self.busy = false;
                match result {
                    Ok(path) => self.status = format!("Saved {}", path.display()),
                    Err(error) if error == "cancelled" => self.status.clear(),
                    Err(error) => self.status = error,
                }
                Task::none()
            }
            Message::Inspect(inspection) => {
                self.inspection = Some(inspection);
                Task::none()
            }
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                Task::none()
            }
            Message::NewChart(new_chart::Message::Calculate) => {
                let result = self
                    .new_chart
                    .calculate()
                    .and_then(|(request, orb_policy)| {
                        self.calculator
                            .as_ref()
                            .ok_or_else(|| "Calculation engine is unavailable".to_owned())?
                            .calculate_with_orb_policy(request, orb_policy)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(chart) => {
                        let archive_id = self
                            .store
                            .as_ref()
                            .and_then(|store| store.insert_chart(&chart).ok())
                            .map(|record| record.id);
                        if archive_id.is_none() {
                            self.new_chart.error =
                                Some("Chart archive could not be updated".to_owned());
                            return Task::none();
                        }
                        self.document = Some(ChartDocument::new(chart));
                        self.document_path = None;
                        self.archive_id = archive_id;
                        if let Some(store) = &self.store {
                            self.archive.refresh(store);
                        }
                        self.screen = Screen::Chart;
                        self.inspection = None;
                        "Archived".clone_into(&mut self.status);
                    }
                    Err(error) => self.new_chart.error = Some(error),
                }
                Task::none()
            }
            Message::NewChart(message) => {
                if let Some(cities) = &self.cities {
                    self.new_chart.update(message, cities);
                }
                Task::none()
            }
            Message::Ephemeris(message) => self
                .calculator
                .as_ref()
                .map_or_else(Task::none, |calculator| {
                    self.ephemeris.update(message, calculator.provider())
                })
                .map(Message::Ephemeris),
            Message::Timing(message) => self
                .calculator
                .as_ref()
                .map_or_else(Task::none, |calculator| {
                    self.timing.update(
                        message,
                        calculator,
                        self.document.as_ref().map(|document| &document.chart),
                    )
                })
                .map(Message::Timing),
            Message::Relationships(message) => self
                .calculator
                .as_ref()
                .map_or_else(Task::none, |calculator| {
                    self.relationships.update(
                        message,
                        calculator,
                        self.document.as_ref(),
                        self.document_path.as_deref(),
                    )
                })
                .map(Message::Relationships),
            Message::Elections(elections::Message::OpenCandidate(index)) => {
                if let Some(chart) = self.elections.candidate(index).cloned() {
                    self.archive_id = self
                        .store
                        .as_ref()
                        .and_then(|store| store.insert_chart(&chart).ok())
                        .map(|record| record.id);
                    self.document = Some(ChartDocument::new(chart));
                    self.document_path = None;
                    if let Some(store) = &self.store {
                        self.archive.refresh(store);
                    }
                    self.screen = Screen::Chart;
                    self.inspection = None;
                    "Archived".clone_into(&mut self.status);
                }
                Task::none()
            }
            Message::Elections(message) => match (&self.calculator, &self.cities) {
                (Some(calculator), Some(cities)) => self
                    .elections
                    .update(message, calculator, cities)
                    .map(Message::Elections),
                _ => Task::none(),
            },
            Message::Archive(archive::Message::Refresh) => {
                if let Some(store) = &self.store {
                    self.archive.refresh(store);
                }
                Task::none()
            }
            Message::Archive(archive::Message::Open(id)) => {
                if let Some(store) = &self.store {
                    match store.get_chart(id) {
                        Ok(Some(record)) => {
                            self.document = Some(ChartDocument::new(record.chart));
                            self.document_path = None;
                            self.archive_id = Some(record.id);
                            self.screen = Screen::Chart;
                            self.inspection = None;
                            self.status.clear();
                        }
                        Ok(None) => "Chart not found".clone_into(&mut self.status),
                        Err(error) => self.status = error.to_string(),
                    }
                }
                Task::none()
            }
            Message::Archive(archive::Message::Delete(id)) => {
                Task::perform(confirm_delete_chart(), move |confirmed| {
                    Message::ArchiveDeleteConfirmed(id, confirmed)
                })
            }
            Message::ArchiveDeleteConfirmed(id, confirmed) => {
                if confirmed && let Some(store) = &self.store {
                    match store.delete_chart(id) {
                        Ok(_) => {
                            self.archive.refresh(store);
                            if self.archive_id == Some(id) {
                                self.close_document();
                            }
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(error) = &self.fatal_error {
            return container(
                column![
                    text("Meridian could not start").size(26),
                    text(error).size(14).color(MUTED),
                    text("Install the ephemeris and city data, or set MERIDIAN_EPHE_PATH and MERIDIAN_CITY_PATH.")
                        .size(13)
                        .color(MUTED),
                ]
                .spacing(14)
                .max_width(620),
            )
            .center(Fill)
            .padding(32)
            .into();
        }

        column![
            self.command_bar(),
            rule::horizontal(1),
            row![self.navigation(), self.workspace()].height(Fill),
            self.status_bar(),
        ]
        .height(Fill)
        .into()
    }

    fn command_bar(&self) -> Element<'_, Message> {
        let document_available = self.document.is_some() && !self.busy;
        let new = toolbar_button("New", Some(Message::NewDocument));
        let open = toolbar_button("Open", (!self.busy).then_some(Message::OpenDocument));
        let save = toolbar_button("Save", document_available.then_some(Message::SaveDocument));
        let save_as = toolbar_button(
            "Save As",
            document_available.then_some(Message::SaveDocumentAs),
        );
        let close = toolbar_button(
            "Close",
            (self.document.is_some() && !self.busy).then_some(Message::CloseDocument),
        );
        let export_available = self.active_chart().is_some() && !self.busy;
        let export_svg = toolbar_button(
            "Export SVG",
            export_available.then_some(Message::ExportChart(ChartExport::Svg)),
        );
        let export_csv = toolbar_button(
            "Export CSV",
            export_available.then_some(Message::ExportChart(ChartExport::Csv)),
        );
        let document_label = self
            .document_path
            .as_deref()
            .map(display_name)
            .or_else(|| {
                self.document
                    .as_ref()
                    .map(|document| document.chart.request.title.clone())
            })
            .unwrap_or_default();

        container(
            row![
                text("MERIDIAN").size(15).color(ACCENT),
                space::horizontal().width(Length::Fixed(18.0)),
                new,
                open,
                save,
                save_as,
                close,
                space::horizontal().width(Length::Fixed(8.0)),
                export_svg,
                export_csv,
                space::horizontal(),
                text(document_label).size(13).color(MUTED),
            ]
            .align_y(Center)
            .spacing(6),
        )
        .height(Length::Fixed(52.0))
        .padding([8, 14])
        .into()
    }

    fn navigation(&self) -> Element<'_, Message> {
        let items = [
            (Screen::CurrentSky, "Current Sky"),
            (Screen::Archive, "Charts"),
            (Screen::NewChart, "New Chart"),
            (Screen::Ephemeris, "Ephemeris"),
            (Screen::Timing, "Timing"),
            (Screen::Relationships, "Relationships"),
            (Screen::Elections, "Elections"),
        ];
        let mut navigation = column![]
            .spacing(5)
            .padding([16, 10])
            .width(Length::Fixed(178.0));
        for (screen, label) in items {
            let active = self.screen == screen;
            navigation = navigation.push(
                button(text(label).size(14))
                    .width(Fill)
                    .padding([9, 11])
                    .style(if active {
                        button::primary
                    } else {
                        button::text
                    })
                    .on_press(Message::Navigate(screen)),
            );
        }
        container(navigation)
            .height(Fill)
            .style(sidebar_style)
            .into()
    }

    fn workspace(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::CurrentSky | Screen::Chart => self.chart_workspace(),
            Screen::Archive => self.archive.view().map(Message::Archive),
            Screen::NewChart => self.new_chart.view().map(Message::NewChart),
            Screen::Ephemeris => self.ephemeris.view().map(Message::Ephemeris),
            Screen::Timing => self
                .timing
                .view(self.document.as_ref().map(|document| &document.chart))
                .map(Message::Timing),
            Screen::Relationships => self
                .relationships
                .view(self.document.is_some())
                .map(Message::Relationships),
            Screen::Elections => self.elections.view().map(Message::Elections),
        }
    }

    fn chart_workspace(&self) -> Element<'_, Message> {
        let Some(chart) = self.active_chart() else {
            return container(text("No chart").size(18))
                .padding(24)
                .width(Fill)
                .height(Fill)
                .into();
        };
        let inspection = self.inspection;
        let grid = PaneGrid::new(&self.panes, |_, pane, _| {
            let (title, content) = match pane {
                Pane::Wheel => (
                    "Chart",
                    chart_canvas::view(chart, inspection).map(Message::Inspect),
                ),
                Pane::Data => ("Positions", Self::data_pane(chart)),
                Pane::Inspector => ("Inspector", self.inspector_pane(chart)),
            };
            pane_grid::Content::new(content)
                .title_bar(
                    pane_grid::TitleBar::new(text(title).size(12).color(MUTED)).padding([8, 12]),
                )
                .style(pane_style)
        })
        .spacing(7)
        .on_resize(9, Message::PaneResized);
        container(grid).padding(7).width(Fill).height(Fill).into()
    }

    fn data_pane(chart: &Chart) -> Element<'_, Message> {
        let heading = text(format!(
            "{}  ·  {}  ·  {} houses",
            chart.request.location_name,
            format_civil_time(&chart.request.local_time),
            chart.request.house_system.name()
        ))
        .size(12)
        .color(MUTED);

        let mut positions = column![section_label("PLANETS")].spacing(4);
        for position in &chart.positions {
            let label = format!(
                "{}  {:02}°{:02}′ {}   H{}{}",
                position.planet.glyph(),
                position.degree_in_sign.floor() as u8,
                ((position.degree_in_sign.fract() * 60.0).round() as u8).min(59),
                position.sign,
                position.house,
                if position.retrograde { "  R" } else { "" },
            );
            positions = positions.push(
                button(text(label).size(13))
                    .width(Fill)
                    .padding([6, 8])
                    .style(button::text)
                    .on_press(Message::Inspect(Inspection::Planet(position.planet))),
            );
        }

        let mut aspects = column![section_label("ASPECTS")].spacing(3);
        for (index, aspect) in chart.aspects.iter().enumerate() {
            aspects = aspects.push(
                button(
                    text(format!(
                        "{} {} {}   {:.2}°",
                        aspect.left.name(),
                        aspect.kind.glyph(),
                        aspect.right.name(),
                        aspect.orb
                    ))
                    .size(12),
                )
                .width(Fill)
                .padding([5, 8])
                .style(button::text)
                .on_press(Message::Inspect(Inspection::Aspect(index))),
            );
        }

        scrollable(
            column![heading, rule::horizontal(1), positions, aspects]
                .spacing(14)
                .padding(14),
        )
        .height(Fill)
        .into()
    }

    fn inspector_pane<'a>(&self, chart: &'a Chart) -> Element<'a, Message> {
        let content = match self.inspection {
            Some(Inspection::Planet(planet)) => chart.planet(planet).map_or_else(
                || column![text("Planet not found")],
                |position| {
                    column![
                        text(format!("{}  {}", planet.glyph(), planet)).size(20),
                        key_value("Position", format_position(position.longitude)),
                        key_value("Sign", position.sign.name()),
                        key_value("House", position.house.to_string()),
                        key_value(
                            "Motion",
                            if position.retrograde { "Retrograde" } else { "Direct" }
                        ),
                        key_value("Daily motion", format!("{:.4}°", position.speed_longitude)),
                        key_value("Declination", format!("{:+.4}°", position.declination)),
                        text(format!(
                            "{} rules {} and has its traditional joy in house {}.",
                            planet,
                            ruled_signs(planet),
                            planet.joy_house()
                        ))
                        .size(12)
                        .color(MUTED),
                    ]
                    .spacing(9)
                },
            ),
            Some(Inspection::Aspect(index)) => chart.aspects.get(index).map_or_else(
                || column![text("Aspect not found")],
                |aspect| {
                    column![
                        text(format!("{}  {}", aspect.kind.glyph(), aspect.kind.name())).size(20),
                        key_value("Points", format!("{} — {}", aspect.left.name(), aspect.right.name())),
                        key_value("Exact angle", format!("{:.0}°", aspect.kind.angle())),
                        key_value("Orb", format!("{:.3}°", aspect.orb)),
                        key_value("Phase", format!("{:?}", aspect.phase)),
                        key_value("Partile", if aspect.partile { "Yes" } else { "No" }),
                        text(aspect_description(aspect.kind)).size(12).color(MUTED),
                    ]
                    .spacing(9)
                },
            ),
            Some(Inspection::Sign(sign)) => column![
                text(format!("{}  {}", sign.glyph(), sign)).size(20),
                key_value("Element", format!("{:?}", sign.element())),
                key_value("Modality", format!("{:?}", sign.modality())),
                key_value("Ruler", sign.ruler().name()),
                key_value("Tropical span", {
                    let start = u16::from(sign.index()) * 30;
                    format!("{start}°–{}°", start + 30)
                }),
            ]
            .spacing(9),
            Some(Inspection::House(house)) => {
                let cusp = chart.houses.cusps[usize::from(house.saturating_sub(1))];
                column![
                    text(format!("House {house}")).size(20),
                    key_value("Cusp", format_position(cusp)),
                    key_value("System", chart.houses.system.name()),
                    text(house_description(house)).size(12).color(MUTED),
                ]
                .spacing(9)
            }
            Some(Inspection::Ascendant) => column![
                text("Ascendant").size(20),
                key_value("Position", format_position(chart.houses.ascendant)),
                key_value("Chart ruler", chart.chart_ruler.name()),
                text("The eastern horizon at the chart moment. It begins the first house and establishes the chart ruler.")
                    .size(12)
                    .color(MUTED),
            ]
            .spacing(9),
            Some(Inspection::Midheaven) => column![
                text("Midheaven").size(20),
                key_value("Position", format_position(chart.houses.midheaven)),
                text("The upper meridian. It is the culminating point of the local sky and a primary angular point.")
                    .size(12)
                    .color(MUTED),
            ]
            .spacing(9),
            None => column![],
        };
        scrollable(content.padding(14)).height(Fill).into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        container(
            row![
                text(&self.status).size(11).color(MUTED),
                space::horizontal(),
            ]
            .align_y(Center),
        )
        .height(Length::Fixed(28.0))
        .padding([5, 12])
        .style(status_style)
        .into()
    }

    fn active_chart(&self) -> Option<&Chart> {
        match self.screen {
            Screen::Chart => self.document.as_ref().map(|document| &document.chart),
            Screen::CurrentSky => self.current_sky.as_ref(),
            _ => None,
        }
    }

    fn subscription(_: &Self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };
            if !modifiers.command() {
                return None;
            }
            match key.as_ref() {
                keyboard::Key::Character("n") => Some(Message::NewDocument),
                keyboard::Key::Character("o") => Some(Message::OpenDocument),
                keyboard::Key::Character("s") if modifiers.shift() => Some(Message::SaveDocumentAs),
                keyboard::Key::Character("s") => Some(Message::SaveDocument),
                keyboard::Key::Character("w") => Some(Message::CloseDocument),
                _ => None,
            }
        })
    }

    fn window_title(&self) -> String {
        if let Some(path) = self.document_path.as_deref() {
            format!("{} — Meridian", display_name(path))
        } else if let Some(document) = &self.document {
            format!("{} — Meridian", document.chart.request.title)
        } else {
            "Meridian".to_owned()
        }
    }

    fn theme(_: &Self) -> Theme {
        Theme::custom(
            "Meridian",
            iced::theme::Palette {
                background: Color::from_rgb8(12, 16, 26),
                text: Color::from_rgb8(224, 230, 240),
                primary: Color::from_rgb8(47, 189, 166),
                success: Color::from_rgb8(74, 222, 128),
                warning: Color::from_rgb8(245, 183, 72),
                danger: Color::from_rgb8(248, 113, 113),
            },
        )
    }

    fn start_new_document(&mut self) {
        self.new_chart = new_chart::State::default();
        self.document = None;
        self.document_path = None;
        self.archive_id = None;
        self.screen = Screen::NewChart;
        self.inspection = None;
        self.status.clear();
    }

    fn start_open_document(&mut self) -> Task<Message> {
        if self.busy {
            return Task::none();
        }
        self.busy = true;
        self.status.clear();
        Task::perform(open_document(), Message::DocumentOpened)
    }

    fn close_document(&mut self) {
        self.document = None;
        self.document_path = None;
        self.archive_id = None;
        self.screen = Screen::CurrentSky;
        self.inspection = None;
        self.status.clear();
    }

    fn has_unsaved_document(&self) -> bool {
        self.document.is_some() && self.document_path.is_none() && self.archive_id.is_none()
    }
}

#[derive(Debug)]
struct DesktopResources {
    database: PathBuf,
    ephemeris: PathBuf,
    cities: PathBuf,
}

impl DesktopResources {
    fn resolve() -> Result<Self> {
        Ok(Self {
            database: database_path()?,
            ephemeris: resource_path("MERIDIAN_EPHE_PATH", "ephe")?,
            cities: resource_path("MERIDIAN_CITY_PATH", "geonames")?,
        })
    }

    fn load(self) -> Result<(ChartCalculator, CityIndex, Store)> {
        let provider = SwissEphemerisProvider::new(&self.ephemeris).with_context(|| {
            format!(
                "could not open ephemeris data at {}",
                self.ephemeris.display()
            )
        })?;
        let cities = CityIndex::load(&self.cities)
            .with_context(|| format!("could not open city atlas at {}", self.cities.display()))?;
        let store = Store::open(&self.database).with_context(|| {
            format!(
                "could not open chart archive at {}",
                self.database.display()
            )
        })?;
        Ok((ChartCalculator::new(provider), cities, store))
    }
}

fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("MERIDIAN_DATABASE") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Meridian").join("meridian.sqlite3"))
            .ok_or_else(|| anyhow!("LOCALAPPDATA is not set"));
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| {
                path.join("Library/Application Support/Meridian")
                    .join("meridian.sqlite3")
            })
            .ok_or_else(|| anyhow!("HOME is not set"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(path)
                .join("meridian")
                .join("meridian.sqlite3"));
        }
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".local/share/meridian").join("meridian.sqlite3"))
            .ok_or_else(|| anyhow!("HOME is not set"))
    }
}

fn resource_path(variable: &str, directory: &str) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(variable) {
        return Ok(PathBuf::from(path));
    }
    let executable = std::env::current_exe().context("could not locate the Meridian executable")?;
    let executable_directory = executable
        .parent()
        .ok_or_else(|| anyhow!("the Meridian executable has no parent directory"))?;
    let candidates = [
        executable_directory.join("data").join(directory),
        executable_directory
            .parent()
            .unwrap_or(executable_directory)
            .join("share/meridian/data")
            .join(directory),
        executable_directory
            .parent()
            .unwrap_or(executable_directory)
            .join("lib/data")
            .join(directory),
        executable_directory
            .parent()
            .unwrap_or(executable_directory)
            .join("lib/meridian/data")
            .join(directory),
        executable_directory
            .parent()
            .unwrap_or(executable_directory)
            .join("Resources/data")
            .join(directory),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join(directory),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow!("could not locate bundled data/{directory}"))
}

fn startup_document_path() -> Option<PathBuf> {
    std::env::args_os().skip(1).map(PathBuf::from).find(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some(CHART_EXTENSION)
    })
}

fn current_sky_request() -> ChartRequest {
    let now = Utc::now();
    ChartRequest {
        title: "Current Sky".to_owned(),
        purpose: ChartPurpose::Event,
        local_time: CivilDateTime {
            year: now.year(),
            month: now.month() as u8,
            day: now.day() as u8,
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: f64::from(now.second()),
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

async fn open_document() -> Result<(PathBuf, ChartDocument), String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Open Meridian chart")
        .add_filter("Meridian chart", &[CHART_EXTENSION])
        .pick_file()
        .await
        .ok_or_else(|| "cancelled".to_owned())?;
    let path = file.path().to_path_buf();
    let document = ChartDocument::open(&path).map_err(|error| error.to_string())?;
    Ok((path, document))
}

async fn save_document(
    document: ChartDocument,
    current_path: Option<PathBuf>,
    save_as: bool,
) -> Result<PathBuf, String> {
    let path = if save_as { None } else { current_path.clone() };
    let path = if let Some(path) = path {
        path
    } else {
        let mut dialog = rfd::AsyncFileDialog::new()
            .set_title("Save Meridian chart")
            .add_filter("Meridian chart", &[CHART_EXTENSION]);
        if let Some(current_path) = current_path {
            if let Some(name) = current_path.file_name().and_then(|name| name.to_str()) {
                dialog = dialog.set_file_name(name);
            }
        } else {
            dialog = dialog.set_file_name(safe_file_name(&document.chart.request.title));
        }
        dialog
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
            .ok_or_else(|| "cancelled".to_owned())?
    };
    let path = ensure_extension(path);
    document.save(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

async fn confirm_discard() -> bool {
    rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Discard unsaved chart?")
        .set_description("The unsaved chart will be discarded.")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        .await
        == rfd::MessageDialogResult::Yes
}

async fn confirm_delete_chart() -> bool {
    rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Delete chart?")
        .set_description("This removes the chart from the archive.")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        .await
        == rfd::MessageDialogResult::Yes
}

async fn export_chart(chart: Chart, kind: ChartExport) -> Result<PathBuf, String> {
    let (title, name, extension) = match kind {
        ChartExport::Svg => ("Export chart", "meridian-chart.svg", "svg"),
        ChartExport::Csv => ("Export chart data", "meridian-chart.csv", "csv"),
    };
    let file = rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_file_name(name)
        .add_filter(extension.to_uppercase(), &[extension])
        .save_file()
        .await
        .ok_or_else(|| "cancelled".to_owned())?;
    let bytes = match kind {
        ChartExport::Svg => {
            crate::render::render_wheel(&chart, crate::render::WheelOptions::default()).into_bytes()
        }
        ChartExport::Csv => crate::render::chart_csv(&chart).map_err(|error| error.to_string())?,
    };
    std::fs::write(file.path(), bytes).map_err(|error| error.to_string())?;
    Ok(file.path().to_path_buf())
}

fn ensure_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().and_then(|extension| extension.to_str()) != Some(CHART_EXTENSION) {
        let mut name = path
            .file_name()
            .map_or_else(OsString::new, std::ffi::OsStr::to_os_string);
        name.push(format!(".{CHART_EXTENSION}"));
        path.set_file_name(name);
    }
    path
}

fn safe_file_name(title: &str) -> String {
    let base = title
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    format!(
        "{}.{CHART_EXTENSION}",
        if base.is_empty() { "chart" } else { &base }
    )
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| path.display().to_string(), str::to_owned)
}

fn toolbar_button(label: &str, message: Option<Message>) -> iced::widget::Button<'_, Message> {
    button(text(label).size(13))
        .padding([7, 11])
        .style(button::secondary)
        .on_press_maybe(message)
}

fn section_label(label: &str) -> iced::widget::Text<'_> {
    text(label).size(10).color(MUTED)
}

fn key_value(label: &str, value: impl Into<String>) -> Element<'_, Message> {
    row![
        text(label)
            .size(12)
            .color(MUTED)
            .width(Length::FillPortion(2)),
        text(value.into()).size(12).width(Length::FillPortion(3)),
    ]
    .spacing(8)
    .into()
}

fn format_position(longitude: f64) -> String {
    let sign = crate::astro::ZodiacSign::from_longitude(longitude);
    let degree = longitude.rem_euclid(30.0);
    format!(
        "{:02}°{:02}′ {}",
        degree.floor() as u8,
        ((degree.fract() * 60.0).round() as u8).min(59),
        sign
    )
}

fn format_civil_time(time: &CivilDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        time.year, time.month, time.day, time.hour, time.minute
    )
}

fn ruled_signs(planet: crate::astro::Planet) -> &'static str {
    use crate::astro::Planet;
    match planet {
        Planet::Sun => "Leo",
        Planet::Moon => "Cancer",
        Planet::Mercury => "Gemini and Virgo",
        Planet::Venus => "Taurus and Libra",
        Planet::Mars => "Aries and Scorpio",
        Planet::Jupiter => "Sagittarius and Pisces",
        Planet::Saturn => "Capricorn and Aquarius",
    }
}

fn house_description(house: u8) -> &'static str {
    match house {
        1 => "Life, body, character and the manner of beginning.",
        2 => "Possessions, livelihood and movable resources.",
        3 => "Siblings, local journeys, messages and familiar learning.",
        4 => "Home, parents, land and the foundations or endings of matters.",
        5 => "Children, creativity, pleasure and good fortune.",
        6 => "Illness, labor, obligations and small animals.",
        7 => "Partners, contracts, rivals and other people encountered directly.",
        8 => "Shared assets, inheritance, debt and mortality.",
        9 => "Religion, divination, higher learning and distant travel.",
        10 => "Action, reputation, authority and public office.",
        11 => "Friends, allies, benefactors and hopes.",
        12 => "Confinement, hidden enemies, sorrow and large animals.",
        _ => "House information is unavailable.",
    }
}

fn aspect_description(kind: crate::astro::AspectKind) -> &'static str {
    use crate::astro::AspectKind;
    match kind {
        AspectKind::Conjunction => {
            "Two points occupy the same degree, combining their significations."
        }
        AspectKind::Sextile => {
            "A 60° aspect traditionally associated with Venus and moderate cooperation."
        }
        AspectKind::Square => "A 90° aspect traditionally associated with Mars and active tension.",
        AspectKind::Trine => {
            "A 120° aspect traditionally associated with Jupiter and easy support."
        }
        AspectKind::Opposition => {
            "A 180° aspect traditionally associated with Saturn and polarity."
        }
    }
}

fn sidebar_style(_theme: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgb8(15, 20, 31))
}

fn status_style(_theme: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgb8(15, 20, 31))
}

fn pane_style(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(12, 16, 26))
        .border(iced::Border {
            color: Color::from_rgb8(36, 45, 62),
            width: 1.0,
            radius: 7.0.into(),
        })
}

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ACCENT: Color = Color::from_rgb(0.30, 0.82, 0.72);
