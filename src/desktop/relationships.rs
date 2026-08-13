use std::path::{Path, PathBuf};

use iced::widget::{button, column, pick_list, row, rule, scrollable, text};
use iced::{Color, Element, Fill, Length, Task};

use crate::astro::{Chart, ChartCalculator, CompositeChart, RelationshipCalculator, Synastry};
use crate::document::{CHART_EXTENSION, ChartDocument};
use crate::render::{WheelOptions, render_composite_wheel, render_synastry_wheel, render_wheel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    #[default]
    Synastry,
    Composite,
    Davison,
}

impl Method {
    const ALL: [Self; 3] = [Self::Synastry, Self::Composite, Self::Davison];
}

impl std::fmt::Display for Method {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Synastry => "Synastry",
            Self::Composite => "Composite midpoint",
            Self::Davison => "Davison",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Synastry {
        report: Synastry,
        first: Box<Chart>,
        second: Box<Chart>,
    },
    Composite(CompositeChart),
    Davison(Box<Chart>),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    first: Option<(PathBuf, ChartDocument)>,
    second: Option<(PathBuf, ChartDocument)>,
    method: Method,
    output: Option<Output>,
    error: Option<String>,
    busy: bool,
    status: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenFirst,
    OpenSecond,
    FirstOpened(Result<(PathBuf, ChartDocument), String>),
    SecondOpened(Result<(PathBuf, ChartDocument), String>),
    UseCurrentFirst,
    UseCurrentSecond,
    MethodChanged(Method),
    Calculate,
    Calculated(Result<Output, String>),
    Export,
    Exported(Result<PathBuf, String>),
}

impl State {
    pub fn update(
        &mut self,
        message: Message,
        calculator: &ChartCalculator,
        current: Option<&ChartDocument>,
        current_path: Option<&Path>,
    ) -> Task<Message> {
        self.error = None;
        match message {
            Message::OpenFirst => {
                if !self.busy {
                    self.busy = true;
                    return Task::perform(open_chart(), Message::FirstOpened);
                }
            }
            Message::OpenSecond => {
                if !self.busy {
                    self.busy = true;
                    return Task::perform(open_chart(), Message::SecondOpened);
                }
            }
            Message::FirstOpened(result) => {
                self.busy = false;
                match result {
                    Ok(chart) => {
                        self.first = Some(chart);
                        self.output = None;
                    }
                    Err(error) if error == "cancelled" => {}
                    Err(error) => self.error = Some(error),
                }
            }
            Message::SecondOpened(result) => {
                self.busy = false;
                match result {
                    Ok(chart) => {
                        self.second = Some(chart);
                        self.output = None;
                    }
                    Err(error) if error == "cancelled" => {}
                    Err(error) => self.error = Some(error),
                }
            }
            Message::UseCurrentFirst => {
                if let Some(document) = current.cloned() {
                    self.first = Some((
                        current_path
                            .map_or_else(|| PathBuf::from("Unsaved chart"), Path::to_path_buf),
                        document,
                    ));
                    self.output = None;
                }
            }
            Message::UseCurrentSecond => {
                if let Some(document) = current.cloned() {
                    self.second = Some((
                        current_path
                            .map_or_else(|| PathBuf::from("Unsaved chart"), Path::to_path_buf),
                        document,
                    ));
                    self.output = None;
                }
            }
            Message::MethodChanged(method) => {
                self.method = method;
                self.output = None;
            }
            Message::Calculate => {
                if self.busy {
                    return Task::none();
                }
                let (Some((_, first)), Some((_, second))) = (&self.first, &self.second) else {
                    self.error = Some("Choose two chart files".to_owned());
                    return Task::none();
                };
                if first.chart == second.chart {
                    self.error = Some("Choose two different charts".to_owned());
                    return Task::none();
                }
                self.busy = true;
                let first = first.chart.clone();
                let second = second.chart.clone();
                let method = self.method;
                let calculator = calculator.clone();
                return Task::perform(
                    async move { calculate(calculator, first, second, method) },
                    Message::Calculated,
                );
            }
            Message::Calculated(result) => {
                self.busy = false;
                match result {
                    Ok(output) => self.output = Some(output),
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Export => {
                let Some(output) = self.output.clone() else {
                    return Task::none();
                };
                if !self.busy {
                    self.busy = true;
                    return Task::perform(export(output), Message::Exported);
                }
            }
            Message::Exported(result) => {
                self.busy = false;
                match result {
                    Ok(path) => self.status = format!("Saved {}", path.display()),
                    Err(error) if error == "cancelled" => {}
                    Err(error) => self.error = Some(error),
                }
            }
        }
        Task::none()
    }

    pub fn view(&self, current_available: bool) -> Element<'_, Message> {
        let first = file_picker(
            "First chart",
            self.first.as_ref().map(|(_, document)| document),
            Message::OpenFirst,
            current_available.then_some(Message::UseCurrentFirst),
        );
        let second = file_picker(
            "Second chart",
            self.second.as_ref().map(|(_, document)| document),
            Message::OpenSecond,
            current_available.then_some(Message::UseCurrentSecond),
        );
        let controls = row![
            pick_list(&Method::ALL[..], Some(self.method), Message::MethodChanged)
                .width(Length::Fixed(210.0)),
            button(if self.busy { "Working" } else { "Calculate" })
                .style(button::primary)
                .padding([8, 15])
                .on_press_maybe(
                    (self.first.is_some() && self.second.is_some() && !self.busy)
                        .then_some(Message::Calculate)
                ),
            button("Export SVG")
                .style(button::secondary)
                .padding([8, 12])
                .on_press_maybe((self.output.is_some() && !self.busy).then_some(Message::Export)),
            iced::widget::space::horizontal(),
            text(&self.status).size(11).color(MUTED),
        ]
        .spacing(8)
        .align_y(iced::Center);

        let mut content = column![
            text("Relationships").size(23),
            row![first, second].spacing(10),
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
}

fn file_picker<'a>(
    label: &'a str,
    document: Option<&'a ChartDocument>,
    open: Message,
    use_current: Option<Message>,
) -> Element<'a, Message> {
    column![
        text(label).size(10).color(MUTED),
        row![
            text(document.map_or("None", |document| document.chart.request.title.as_str()))
                .size(13)
                .width(Fill),
            button("Open…").style(button::secondary).on_press(open),
            button("Use open chart")
                .style(button::text)
                .on_press_maybe(use_current),
        ]
        .spacing(5)
        .align_y(iced::Center),
    ]
    .spacing(5)
    .padding(10)
    .width(Length::FillPortion(1))
    .into()
}

fn calculate(
    chart_calculator: ChartCalculator,
    first: Chart,
    second: Chart,
    method: Method,
) -> Result<Output, String> {
    let relationships = RelationshipCalculator::default();
    match method {
        Method::Synastry => Ok(Output::Synastry {
            report: relationships.synastry(&first, &second),
            first: Box::new(first),
            second: Box::new(second),
        }),
        Method::Composite => Ok(Output::Composite(relationships.composite(
            &first,
            &second,
            format!("{} × {}", first.request.title, second.request.title),
        ))),
        Method::Davison => relationships
            .davison(
                &chart_calculator,
                &first,
                &second,
                format!(
                    "{} × {} · Davison",
                    first.request.title, second.request.title
                ),
                first.request.house_system,
            )
            .map(|chart| Output::Davison(Box::new(chart)))
            .map_err(|error| error.to_string()),
    }
}

fn output_view(output: &Output) -> Element<'_, Message> {
    match output {
        Output::Synastry { report, .. } => {
            let mut content =
                column![text(format!("{} × {}", report.first_title, report.second_title)).size(18)]
                    .spacing(5);
            for aspect in &report.aspects {
                content = content.push(
                    text(format!(
                        "{} {} {}  {:.2}°{}",
                        aspect.first,
                        aspect.kind.glyph(),
                        aspect.second,
                        aspect.orb,
                        if aspect.partile { "  partile" } else { "" }
                    ))
                    .size(12),
                );
            }
            content = content.push(rule::horizontal(1));
            for overlay in &report.first_in_second_houses {
                content = content.push(
                    text(format!(
                        "First {} in second house {}",
                        overlay.planet, overlay.house
                    ))
                    .size(12),
                );
            }
            for overlay in &report.second_in_first_houses {
                content = content.push(
                    text(format!(
                        "Second {} in first house {}",
                        overlay.planet, overlay.house
                    ))
                    .size(12),
                );
            }
            for reception in &report.mutual_receptions {
                content = content.push(
                    text(format!(
                        "Mutual reception: {} / {}",
                        reception.first, reception.second
                    ))
                    .size(12),
                );
            }
            content.into()
        }
        Output::Composite(chart) => {
            let mut content = column![
                text(&chart.title).size(18),
                text(&chart.method).size(11).color(MUTED)
            ]
            .spacing(5);
            for position in &chart.positions {
                content = content.push(
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
            content.into()
        }
        Output::Davison(chart) => {
            let mut content = column![text(&chart.request.title).size(18)].spacing(5);
            for position in &chart.positions {
                content = content.push(
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
            content.into()
        }
    }
}

async fn open_chart() -> Result<(PathBuf, ChartDocument), String> {
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

async fn export(output: Output) -> Result<PathBuf, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Export relationship chart")
        .set_file_name("meridian-relationship.svg")
        .add_filter("SVG", &["svg"])
        .save_file()
        .await
        .ok_or_else(|| "cancelled".to_owned())?;
    let svg = match output {
        Output::Synastry {
            report,
            first,
            second,
        } => render_synastry_wheel(&first, &second, &report, 960),
        Output::Composite(chart) => render_composite_wheel(&chart, 960),
        Output::Davison(chart) => render_wheel(
            &chart,
            WheelOptions {
                size: 960,
                ..WheelOptions::default()
            },
        ),
    };
    std::fs::write(file.path(), svg).map_err(|error| error.to_string())?;
    Ok(file.path().to_path_buf())
}

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ERROR: Color = Color::from_rgb(0.97, 0.44, 0.44);
