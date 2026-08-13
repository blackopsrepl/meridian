use iced::widget::{button, column, row, rule, scrollable, text};
use iced::{Color, Element, Fill, Length};
use uuid::Uuid;

use crate::store::{ChartSummary, Store};

#[derive(Debug, Default)]
pub struct State {
    charts: Vec<ChartSummary>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    Open(Uuid),
    Delete(Uuid),
}

impl State {
    pub fn refresh(&mut self, store: &Store) {
        match store.list_charts(500) {
            Ok(charts) => {
                self.charts = charts;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![
            row![
                text("Charts").size(23),
                iced::widget::space::horizontal(),
                button("Refresh")
                    .style(button::secondary)
                    .on_press(Message::Refresh),
            ]
            .align_y(iced::Center),
            rule::horizontal(1),
            row![
                text("NAME")
                    .size(10)
                    .color(MUTED)
                    .width(Length::FillPortion(3)),
                text("DATE")
                    .size(10)
                    .color(MUTED)
                    .width(Length::FillPortion(2)),
                text("LOCATION")
                    .size(10)
                    .color(MUTED)
                    .width(Length::FillPortion(3)),
                text("TYPE")
                    .size(10)
                    .color(MUTED)
                    .width(Length::FillPortion(1)),
                iced::widget::space::horizontal().width(Length::Fixed(126.0)),
            ]
            .spacing(8),
        ]
        .spacing(9)
        .padding(18);
        if let Some(error) = &self.error {
            content = content.push(text(error).size(13).color(ERROR));
        }
        for chart in &self.charts {
            content = content.push(
                row![
                    text(&chart.title).size(13).width(Length::FillPortion(3)),
                    text(&chart.local_date)
                        .size(12)
                        .width(Length::FillPortion(2)),
                    text(&chart.location_name)
                        .size(12)
                        .width(Length::FillPortion(3)),
                    text(&chart.purpose).size(11).width(Length::FillPortion(1)),
                    button("Open")
                        .style(button::secondary)
                        .on_press(Message::Open(chart.id)),
                    button("Delete")
                        .style(button::danger)
                        .on_press(Message::Delete(chart.id)),
                ]
                .spacing(8)
                .align_y(iced::Center),
            );
            content = content.push(rule::horizontal(1));
        }
        scrollable(content).height(Fill).into()
    }
}

const MUTED: Color = Color::from_rgb(0.51, 0.57, 0.67);
const ERROR: Color = Color::from_rgb(0.97, 0.44, 0.44);
