use maud::{Markup, PreEscaped, html};
use serde::Serialize;

use crate::astro::{
    AnnualProfection, Calendar, Chart, FirdariaPeriod, PlanetaryHours, PointId, TechniqueChart,
    TraditionalHouseSystem, TransitEvent, civil_from_julian_day,
};
use crate::render::{WheelOptions, render_wheel};
use crate::store::ChartSummary;

use super::components::compact_zodiac;
use super::layout::page;

#[derive(Debug, Serialize)]
#[serde(tag = "technique", content = "result", rename_all = "snake_case")]
pub enum TimingOutput {
    Transits(Vec<TransitEvent>),
    Secondary(TechniqueChart),
    SolarArc(TechniqueChart),
    Harmonic(TechniqueChart),
    Profection(AnnualProfection),
    Firdaria(FirdariaPeriod),
    SolarReturn(Box<Chart>),
    LunarReturn(Box<Chart>),
    PlanetaryHours(PlanetaryHours),
}

#[allow(clippy::too_many_arguments)]
pub fn timing_page(
    charts: &[ChartSummary],
    selected_id: Option<&str>,
    technique: &str,
    target: &str,
    end: &str,
    age: f64,
    harmonic: u16,
    location: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    house_system: Option<TraditionalHouseSystem>,
    output: Option<&TimingOutput>,
) -> Markup {
    page(
        "Timing",
        "timing",
        html! {
            section class="page-heading" {
                div {
                    p class="eyebrow" { "Chronocrators & motion" }
                    h1 { "Timing workbench" }
                    p class="lede" { "Exact transits and returns share the same ephemeris contract as the radix. Symbolic techniques retain their key and target instant." }
                }
            }
            form class="timing-form panel" method="get" action="/tools/timing" {
                label class="field timing-chart-field" {
                    span { "Radix" }
                    select name="chart" required {
                        option value="" { "Select a saved chart…" }
                        @for chart in charts {
                            option value=(chart.id) selected[selected_id.is_some_and(|id| id == chart.id.to_string())] { (&chart.title) " · " (&chart.local_date) }
                        }
                    }
                }
                label class="field" {
                    span { "Technique" }
                    select name="technique" data-technique-select {
                        (choice("transits", "Exact transits", technique))
                        (choice("secondary", "Secondary progressions", technique))
                        (choice("solar_arc", "Solar arc directions", technique))
                        (choice("solar_return", "Solar return", technique))
                        (choice("lunar_return", "Lunar return", technique))
                        (choice("profection", "Annual profection", technique))
                        (choice("firdaria", "Firdaria", technique))
                        (choice("harmonic", "Harmonic chart", technique))
                        (choice("planetary_hours", "Planetary hours", technique))
                    }
                }
                label class="field" { span { "Target / start (UT)" } input type="date" name="target" value=(target) required; }
                label class="field" { span { "End (transits)" } input type="date" name="end" value=(end) required; }
                label class="field" { span { "Age (years)" } input type="number" name="age" value=(age) min="0" max="140" step="0.01"; }
                label class="field" { span { "Harmonic" } input type="number" name="harmonic" value=(harmonic) min="1" max="360"; }
                label class="field timing-location-field" { span { "Return location override" } input name="location" value=[location] placeholder="Use radix place"; }
                label class="field" { span { "Latitude override" } input type="number" name="latitude" value=[latitude] min="-90" max="90" step="0.000001" placeholder="Use radix"; }
                label class="field" { span { "Longitude override" } input type="number" name="longitude" value=[longitude] min="-180" max="180" step="0.000001" placeholder="Use radix"; }
                label class="field" { span { "Elevation m" } input type="number" name="elevation" value=[elevation] min="-500" max="10000" step="1" placeholder="Use radix"; }
                label class="field" {
                    span { "Return houses" }
                    select name="houses" {
                        option value="" selected[house_system.is_none()] { "Use radix system" }
                        @for system in TraditionalHouseSystem::ALL {
                            option value=(house_key(system)) selected[house_system == Some(system)] { (system.name()) }
                        }
                    }
                }
                button class="button primary" type="submit" { "Calculate technique" }
            }
            @if charts.is_empty() {
                div class="empty-state tool-empty" {
                    div class="empty-orbit" { "◷" }
                    h3 { "A radix is required" }
                    p { "Cast and save a natal chart before opening its timing techniques." }
                    a class="button primary" href="/charts/new" { "Cast a radix" }
                }
            }
            @if let Some(output) = output { (render_output(output)) }
        },
    )
}

fn render_output(output: &TimingOutput) -> Markup {
    match output {
        TimingOutput::Transits(events) => html! {
            section class="tool-results panel" {
                (result_heading("Exact transit contacts", &format!("{} perfected aspects", events.len())))
                div class="transit-list" {
                    @for event in events {
                        div class="transit-row" {
                            time { (jd_label(event.exact_jd_ut)) }
                            span class="transit-glyph" { (event.transiting.glyph()) }
                            strong { (event.transiting.name()) }
                            span { (event.aspect.glyph()) " " (event.aspect.name()) }
                            strong { (point_glyph(event.target)) " natal " (event.target.name()) }
                            span class="purpose-chip" { @if event.retrograde { "retrograde" } @else { "direct" } }
                        }
                    }
                }
            }
        },
        TimingOutput::Secondary(chart)
        | TimingOutput::SolarArc(chart)
        | TimingOutput::Harmonic(chart) => html! {
            section class="tool-results panel" {
                (result_heading(&chart.title, &format!("{:?}", chart.technique)))
                @if let Some(key) = chart.key_degrees { div class="technique-key" { span { "Key" } strong { (format!("{key:.6}°")) } } }
                div class="derived-grid" {
                    article {
                        h3 { "Directed positions" }
                        @for position in &chart.positions {
                            div class="derived-row" {
                                span { (position.planet.glyph()) " " (position.planet.name()) }
                                strong { (compact_zodiac(position.longitude)) }
                                small { "Natal H" (position.natal_house) @if position.retrograde { " · ℞" } }
                            }
                        }
                    }
                    article {
                        h3 { "Contacts to radix" }
                        @for contact in chart.contacts.iter().take(30) {
                            div class="derived-row contact" {
                                span { (contact.moving.glyph()) " " (contact.moving.name()) }
                                strong { (contact.aspect.glyph()) " " (contact.natal.glyph()) " " (contact.natal.name()) }
                                small { (format!("{:.2}°", contact.orb)) }
                            }
                        }
                    }
                }
            }
        },
        TimingOutput::Profection(value) => html! {
            section class="tool-results focus-result" {
                p class="eyebrow" { "Annual profection" }
                div class="focus-number" { (value.activated_house) }
                h2 { "House " (value.activated_house) " · " (value.activated_sign.glyph()) " " (value.activated_sign.name()) }
                p { "Age " (value.age) " activates " (value.lord_of_year.glyph()) " " (value.lord_of_year.name()) " as lord of the year." }
            }
        },
        TimingOutput::Firdaria(value) => html! {
            section class="tool-results focus-result firdaria-result" {
                p class="eyebrow" { (format!("{:?}", value.sect)) " firdaria" }
                div class="lord-pair" {
                    div { span { "Major lord" } strong { (value.major_lord.glyph()) " " (value.major_lord.name()) } small { (format!("age {:.2}–{:.2}", value.major_started_at_age, value.major_ends_at_age)) } }
                    b { "→" }
                    div { span { "Sub-lord" } strong { (value.sub_lord.glyph()) " " (value.sub_lord.name()) } small { (format!("age {:.2}–{:.2}", value.sub_started_at_age, value.sub_ends_at_age)) } }
                }
            }
        },
        TimingOutput::SolarReturn(chart) | TimingOutput::LunarReturn(chart) => html! {
            section class="tool-results panel return-result" {
                (result_heading(&chart.request.title, &jd_label(chart.moment.jd_ut)))
                div class="return-layout" {
                    div class="wheel-stage" { (PreEscaped(render_wheel(chart, WheelOptions::default()))) }
                    div class="return-ledger" {
                        @for position in &chart.positions {
                            div { span { (position.planet.glyph()) " " (position.planet.name()) } strong { (compact_zodiac(position.longitude)) } }
                        }
                    }
                }
            }
        },
        TimingOutput::PlanetaryHours(hours) => html! {
            section class="tool-results panel" {
                (result_heading("Planetary hours", &format!("Day ruler: {} {}", hours.day_ruler.glyph(), hours.day_ruler.name())))
                div class="hours-grid" {
                    @for hour in &hours.hours {
                        article class={ "planetary-hour " @if hour.is_daylight { "day" } @else { "night" } } {
                            span { (format!("{:02}", hour.number)) }
                            strong { (hour.ruler.glyph()) " " (hour.ruler.name()) }
                            small { (jd_time(hour.starts_jd_ut)) "–" (jd_time(hour.ends_jd_ut)) " UT" }
                        }
                    }
                }
            }
        },
    }
}

fn result_heading(title: &str, detail: &str) -> Markup {
    html! { header class="panel-header tool-result-header" { div { p class="eyebrow" { "Calculated result" } h2 { (title) } } span class="engine-chip" { (detail) } } }
}

fn choice<'a>(value: &'a str, label: &'a str, selected: &str) -> Markup {
    html! { option value=(value) selected[value == selected] { (label) } }
}

fn jd_label(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!(
        "{:+05}-{:02}-{:02} {:02}:{:02} UT",
        value.year, value.month, value.day, value.hour, value.minute
    )
}

fn jd_time(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!("{:02}:{:02}", value.hour, value.minute)
}

const fn house_key(system: TraditionalHouseSystem) -> &'static str {
    match system {
        TraditionalHouseSystem::WholeSign => "whole_sign",
        TraditionalHouseSystem::Equal => "equal",
        TraditionalHouseSystem::Porphyry => "porphyry",
        TraditionalHouseSystem::Alcabitius => "alcabitius",
        TraditionalHouseSystem::Placidus => "placidus",
        TraditionalHouseSystem::Regiomontanus => "regiomontanus",
        TraditionalHouseSystem::Campanus => "campanus",
        TraditionalHouseSystem::Morinus => "morinus",
    }
}

const fn point_glyph(point: PointId) -> &'static str {
    match point {
        PointId::Planet(planet) => planet.glyph(),
        PointId::Ascendant => "Asc",
        PointId::Midheaven => "MC",
        PointId::LotFortune => "⊗",
        PointId::LotSpirit => "⊙",
    }
}
