use maud::{Markup, html};

use crate::astro::{Calendar, EphemerisTable, SkyEvent, SkyEventKind, civil_from_julian_day};

use super::components::compact_zodiac;
use super::layout::page;

pub fn ephemeris_page(
    table: &EphemerisTable,
    events: &[SkyEvent],
    start_date: &str,
    days: usize,
    step: f64,
) -> Markup {
    page(
        "Ephemeris",
        "ephemeris",
        html! {
            section class="page-heading" {
                div {
                    p class="eyebrow" { "Coefficient table" }
                    h1 { "Septenary ephemeris" }
                    p class="lede" { "Daily apparent positions, stations, ingresses, lunations, and eclipses calculated directly from the installed DE441-derived files." }
                }
            }
            form class="tool-form panel" method="get" action="/tools/ephemeris" {
                label class="field" { span { "Start date (UT)" } input type="date" name="start" value=(start_date) required; }
                label class="field" { span { "Rows" } input type="number" name="days" value=(days) min="1" max="366" required; }
                label class="field" { span { "Step in days" } input type="number" name="step" value=(step) min="0.0416667" max="31" step="0.0416667" required; }
                button class="button primary" type="submit" { "Recalculate" }
                a class="button secondary" href=(format!("/api/v1/ephemeris?start_jd={:.8}&rows={days}&step={step}", table.start_jd_ut)) { "JSON" }
                a class="button secondary" href=(format!("/tools/ephemeris.csv?start={start_date}&days={days}&step={step}")) { "CSV" }
            }
            section class="panel ephemeris-panel" {
                header class="panel-header" {
                    div { p class="eyebrow" { "Geocentric tropical" } h2 { (start_date) " · " (days) " rows" } }
                    span class="engine-chip" { "UT · apparent positions" }
                }
                div class="table-wrap ephemeris-wrap" {
                    table class="data-table ephemeris-table" {
                        thead {
                            tr {
                                th { "Date UT" }
                                @if let Some(first) = table.rows.first() {
                                    @for cell in &first.positions { th { (cell.planet.glyph()) " " (cell.planet.name()) } }
                                }
                            }
                        }
                        tbody {
                            @for row in &table.rows {
                                tr {
                                    th scope="row" { (date_time_label(row.jd_ut)) }
                                    @for cell in &row.positions {
                                        td class={ @if cell.retrograde { "retrograde" } } {
                                            strong { (compact_zodiac(cell.longitude)) }
                                            small { (format!("{:+.3}°/d", cell.speed_longitude)) @if cell.retrograde { " ℞" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            section class="section-block" {
                header class="section-header" {
                    div { p class="eyebrow" { "Exact events" } h2 { "Ingresses, stations & lunations" } }
                    span class="purpose-chip" { (events.len()) " events" }
                }
                div class="event-timeline" {
                    @if events.is_empty() {
                        div class="empty-state compact-empty" { h3 { "No events in this interval" } }
                    }
                    @for event in events {
                        article class="event-card" {
                            time { (date_time_label(event.jd_ut)) }
                            span class="event-symbol" { (event_symbol(&event.event)) }
                            div {
                                h3 { (event_title(&event.event)) }
                                p {
                                    @if let Some(longitude) = event.longitude { (compact_zodiac(longitude)) " · " }
                                    "JD " (format!("{:.6}", event.jd_ut))
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn date_time_label(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!(
        "{:+05}-{:02}-{:02} {:02}:{:02}",
        value.year, value.month, value.day, value.hour, value.minute
    )
}

fn event_symbol(event: &SkyEventKind) -> &'static str {
    match event {
        SkyEventKind::Ingress { planet, .. } | SkyEventKind::Station { planet, .. } => {
            planet.glyph()
        }
        SkyEventKind::Lunation { .. } | SkyEventKind::LunarEclipse { .. } => "☽",
        SkyEventKind::SolarEclipse { .. } => "☉",
    }
}

fn event_title(event: &SkyEventKind) -> String {
    match event {
        SkyEventKind::Ingress { planet, sign } => {
            format!("{} enters {}", planet.name(), sign.name())
        }
        SkyEventKind::Station { planet, change } => {
            format!("{} {:?}", planet.name(), change).replace('_', " ")
        }
        SkyEventKind::Lunation { phase } => format!("{phase:?}").replace('_', " "),
        SkyEventKind::SolarEclipse { eclipse } => format!("{eclipse:?} solar eclipse"),
        SkyEventKind::LunarEclipse { eclipse } => format!("{eclipse:?} lunar eclipse"),
    }
}
