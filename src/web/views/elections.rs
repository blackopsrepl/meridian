use maud::{Markup, PreEscaped, html};

use crate::astro::{
    Calendar, ElectionSearchResult, ElectionTopic, TraditionalHouseSystem, civil_from_julian_day,
};
use crate::render::{WheelOptions, render_wheel};

use super::components::compact_zodiac;
use super::layout::page;

#[derive(Debug, Clone)]
pub struct ElectionFormValues {
    pub title: String,
    pub start: String,
    pub end: String,
    pub step_minutes: u16,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub house_system: TraditionalHouseSystem,
    pub topic: ElectionTopic,
    pub limit: usize,
}

pub fn elections_page(
    values: &ElectionFormValues,
    result: Option<&ElectionSearchResult>,
) -> Markup {
    page(
        "Election lab",
        "elections",
        html! {
            section class="page-heading" {
                div {
                    p class="eyebrow" { "Choose the hour" }
                    h1 { "Election laboratory" }
                    p class="lede" { "Scan real charts from the installed ephemeris, rank classical testimonies, and inspect every score instead of accepting an opaque auspicious-time claim." }
                }
            }
            form class="election-form panel" method="get" action="/tools/elections" {
                label class="field election-title" { span { "Undertaking" } input name="title" value=(&values.title) required; }
                label class="field" {
                    span { "Topic" }
                    select name="topic" {
                        @for topic in ElectionTopic::ALL {
                            option value=(topic_key(topic)) selected[topic == values.topic] { (topic.name()) }
                        }
                    }
                }
                label class="field" { span { "Start (UT)" } input type="datetime-local" name="start" value=(&values.start) required; }
                label class="field" { span { "End (UT)" } input type="datetime-local" name="end" value=(&values.end) required; }
                label class="field" { span { "Step (minutes)" } input type="number" name="step" min="5" max="1440" value=(values.step_minutes) required; }
                label class="field" { span { "Results" } input type="number" name="limit" min="1" max="50" value=(values.limit) required; }
                label class="field election-location" { span { "Location" } input name="location" value=(&values.location) required; }
                label class="field" { span { "Latitude" } input type="number" name="latitude" min="-90" max="90" step="0.000001" value=(values.latitude) required; }
                label class="field" { span { "Longitude" } input type="number" name="longitude" min="-180" max="180" step="0.000001" value=(values.longitude) required; }
                label class="field" { span { "Elevation m" } input type="number" name="elevation" min="-500" max="10000" step="1" value=(values.elevation); }
                label class="field" {
                    span { "Houses" }
                    select name="houses" {
                        @for system in TraditionalHouseSystem::ALL {
                            option value=(house_key(system)) selected[system == values.house_system] { (system.name()) }
                        }
                    }
                }
                button class="button primary election-submit" type="submit" { "Search ephemeris" }
            }
            @if let Some(result) = result { (render_result(result)) }
        },
    )
}

fn render_result(result: &ElectionSearchResult) -> Markup {
    html! {
        section class="election-summary" {
            div { p class="eyebrow" { "Ranked candidates" } h2 { (result.request.topic.name()) } }
            div class="heading-metrics" {
                div { strong { (result.evaluated_instants) } span { "charts judged" } }
                div { strong { (result.candidates.len()) } span { "retained" } }
                div { strong { (result.request.step_minutes) " min" } span { "resolution" } }
            }
        }
        @if let Some(best) = result.candidates.first() {
            section class="election-best-grid" {
                article class="panel election-wheel-panel" {
                    header class="panel-header" {
                        div { p class="eyebrow" { "Highest testimony" } h2 { (jd_label(best.chart.moment.jd_ut)) " UT" } }
                        span class="election-score positive" { (format!("{:+}", best.score)) }
                    }
                    div class="wheel-stage relationship-wheel-stage" { (PreEscaped(render_wheel(&best.chart, WheelOptions::default()))) }
                }
                aside class="panel election-testimony" {
                    p class="eyebrow" { "Auditable judgment" }
                    h2 { "Testimonies" }
                    @for item in &best.score_items {
                        div class="testimony-row" {
                            span { (&item.label) small { (&item.rationale) } }
                            strong class={ @if item.score >= 0 { "positive" } @else { "negative" } } { (format!("{:+}", item.score)) }
                        }
                    }
                    @if !best.advisories.is_empty() {
                        div class="advisory-box" {
                            strong { "Cautions" }
                            @for advisory in &best.advisories { span { (&advisory) } }
                        }
                    }
                }
            }
        }
        section class="section-block" {
            header class="section-header" { div { p class="eyebrow" { "Shortlist" } h2 { "Alternative instants" } } }
            div class="candidate-grid" {
                @for candidate in &result.candidates {
                    article class="panel candidate-card" {
                        header {
                            span class="candidate-rank" { "#" (candidate.rank) }
                            div { h3 { (jd_label(candidate.chart.moment.jd_ut)) } small { "UT · " (&candidate.chart.request.location_name) } }
                            strong class={ "election-score " @if candidate.score >= 0 { "positive" } @else { "negative" } } { (format!("{:+}", candidate.score)) }
                        }
                        div class="candidate-signature" {
                            span { "ASC" b { (compact_zodiac(candidate.chart.houses.ascendant)) } }
                            @if let Some(moon) = candidate.chart.planet(crate::astro::Planet::Moon) { span { "Moon" b { (compact_zodiac(moon.longitude)) } } }
                            span { "Sect" b { (format!("{:?}", candidate.chart.sect)) } }
                        }
                        details {
                            summary { "Score ledger" }
                            @for item in &candidate.score_items {
                                div { span { (&item.label) } b { (format!("{:+}", item.score)) } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn jd_label(jd: f64) -> String {
    let value = civil_from_julian_day(jd, Calendar::Gregorian);
    format!(
        "{:+05}-{:02}-{:02} {:02}:{:02}",
        value.year, value.month, value.day, value.hour, value.minute
    )
}

const fn topic_key(topic: ElectionTopic) -> &'static str {
    match topic {
        ElectionTopic::General => "general",
        ElectionTopic::Marriage => "marriage",
        ElectionTopic::Commerce => "commerce",
        ElectionTopic::Travel => "travel",
        ElectionTopic::Career => "career",
        ElectionTopic::Home => "home",
        ElectionTopic::Healing => "healing",
        ElectionTopic::Litigation => "litigation",
    }
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
