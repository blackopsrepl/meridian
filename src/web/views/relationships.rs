use maud::{Markup, PreEscaped, html};

use crate::astro::{Chart, CompositeChart, PointId, Synastry};
use crate::render::{WheelOptions, render_composite_wheel, render_synastry_wheel, render_wheel};
use crate::store::ChartSummary;

use super::components::compact_zodiac;
use super::layout::page;

#[derive(Debug)]
pub enum RelationshipOutput {
    Synastry {
        report: Synastry,
        first: Box<Chart>,
        second: Box<Chart>,
    },
    Composite(CompositeChart),
    Davison(Box<Chart>),
}

pub fn relationships_page(
    charts: &[ChartSummary],
    first_id: Option<&str>,
    second_id: Option<&str>,
    method: &str,
    output: Option<&RelationshipOutput>,
) -> Markup {
    page(
        "Relationships",
        "relationships",
        html! {
            section class="page-heading" {
                div {
                    p class="eyebrow" { "Two nativities" }
                    h1 { "Relationship atelier" }
                    p class="lede" { "Compare the traditional seven directly, derive a circular-midpoint composite, or cast the real Davison midpoint in time and place." }
                }
            }
            form class="relationship-form panel" method="get" action="/tools/relationships" {
                (chart_select("first", "First radix", charts, first_id))
                div class="relationship-join" aria-hidden="true" { "⋈" }
                (chart_select("second", "Second radix", charts, second_id))
                label class="field" {
                    span { "Method" }
                    select name="method" {
                        (method_choice("synastry", "Synastry bi-wheel", method))
                        (method_choice("composite", "Midpoint composite", method))
                        (method_choice("davison", "Davison relationship chart", method))
                    }
                }
                button class="button primary" type="submit" { "Compare charts" }
                @if let (Some(first), Some(second)) = (first_id, second_id) {
                    a class="button secondary" href=(format!("/tools/relationships.svg?first={first}&second={second}&method={method}")) { "SVG" }
                }
            }
            @if charts.len() < 2 {
                div class="empty-state tool-empty" {
                    div class="empty-orbit" { "⋈" }
                    h3 { "Two saved charts are required" }
                    p { "Cast both nativities first. Their complete calculated records remain local in the Meridian archive." }
                    a class="button primary" href="/charts/new" { "Cast a radix" }
                }
            }
            @if let Some(output) = output { (render_output(output)) }
        },
    )
}

fn render_output(output: &RelationshipOutput) -> Markup {
    match output {
        RelationshipOutput::Synastry {
            report,
            first,
            second,
        } => synastry_output(report, first, second),
        RelationshipOutput::Composite(composite) => composite_output(composite),
        RelationshipOutput::Davison(chart) => davison_output(chart),
    }
}

fn synastry_output(report: &Synastry, first: &Chart, second: &Chart) -> Markup {
    html! {
        section class="relationship-result-grid" {
            article class="panel relationship-wheel-panel" {
                header class="panel-header" {
                    div { p class="eyebrow" { "Synastry" } h2 { (&report.first_title) " × " (&report.second_title) } }
                    span class="engine-chip" { (report.aspects.len()) " contacts" }
                }
                div class="wheel-stage relationship-wheel-stage" {
                    (PreEscaped(render_synastry_wheel(first, second, report, 760)))
                }
                footer class="biwheel-legend" {
                    span { b class="first-dot" {} "Outer ring · " (&report.first_title) }
                    span { b class="second-dot" {} "Inner ring · " (&report.second_title) }
                }
            }
            aside class="relationship-ledger" {
                article class="panel relationship-card" {
                    p class="eyebrow" { "Partile first" }
                    h2 { "Cross-chart aspects" }
                    div class="relationship-aspects" {
                        @for aspect in report.aspects.iter().take(24) {
                            div {
                                span { (aspect.first.glyph()) " " (aspect.first.name()) }
                                strong class=(format!("aspect-glyph {}", format!("{:?}", aspect.kind).to_lowercase())) { (aspect.kind.glyph()) }
                                span { (aspect.second.glyph()) " " (aspect.second.name()) }
                                small { (format!("{:.2}°", aspect.orb)) @if aspect.partile { " · partile" } }
                            }
                        }
                    }
                }
                article class="panel relationship-card" {
                    p class="eyebrow" { "Reception & topical place" }
                    h2 { "House overlays" }
                    div class="overlay-columns" {
                        div {
                            small { (&report.first_title) " in " (&report.second_title) }
                            @for overlay in &report.first_in_second_houses {
                                span { (overlay.planet.glyph()) " " (overlay.planet.name()) b { "H" (overlay.house) } }
                            }
                        }
                        div {
                            small { (&report.second_title) " in " (&report.first_title) }
                            @for overlay in &report.second_in_first_houses {
                                span { (overlay.planet.glyph()) " " (overlay.planet.name()) b { "H" (overlay.house) } }
                            }
                        }
                    }
                    @if !report.mutual_receptions.is_empty() {
                        div class="reception-strip" {
                            @for reception in &report.mutual_receptions {
                                span { (reception.first.glyph()) " " (reception.first.name()) " ⇄ " (reception.second.glyph()) " " (reception.second.name()) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn composite_output(composite: &CompositeChart) -> Markup {
    html! {
        section class="relationship-result-grid" {
            article class="panel relationship-wheel-panel" {
                header class="panel-header" { div { p class="eyebrow" { "Derived chart" } h2 { (&composite.title) } } span class="engine-chip" { (&composite.method) } }
                div class="wheel-stage relationship-wheel-stage" { (PreEscaped(render_composite_wheel(composite, 760))) }
            }
            aside class="relationship-ledger" {
                article class="panel relationship-card" {
                    p class="eyebrow" { "Circular midpoints" }
                    h2 { "Composite positions" }
                    @for position in &composite.positions {
                        div class="derived-row" {
                            span { (position.planet.glyph()) " " (position.planet.name()) }
                            strong { (compact_zodiac(position.longitude)) }
                            small { "H" (position.house) @if position.retrograde { " · ℞" } }
                        }
                    }
                }
                article class="panel relationship-card" {
                    p class="eyebrow" { "Internal figure" }
                    h2 { "Composite aspects" }
                        @for aspect in &composite.aspects {
                            div class="derived-row" {
                            span { (point_label(aspect.left)) }
                            strong { (aspect.kind.glyph()) " " (aspect.kind.name()) }
                            small { (format!("{:.2}°", aspect.orb)) }
                        }
                    }
                }
            }
        }
    }
}

fn davison_output(chart: &Chart) -> Markup {
    html! {
        section class="relationship-result-grid" {
            article class="panel relationship-wheel-panel" {
                header class="panel-header" { div { p class="eyebrow" { "Real midpoint in spacetime" } h2 { (&chart.request.title) } } span class="engine-chip" { (format!("JD {:.6}", chart.moment.jd_ut)) } }
                div class="wheel-stage relationship-wheel-stage" { (PreEscaped(render_wheel(chart, WheelOptions::default()))) }
            }
            aside class="relationship-ledger" {
                article class="panel relationship-card focus-result" {
                    p class="eyebrow" { "Davison location" }
                    h2 { (&chart.request.location_name) }
                    p { (format!("{:.6}°, {:.6}°", chart.request.coordinates.latitude, chart.request.coordinates.longitude)) }
                }
                article class="panel relationship-card" {
                    p class="eyebrow" { "Ephemeris positions" }
                    @for position in &chart.positions {
                        div class="derived-row" { span { (position.planet.glyph()) " " (position.planet.name()) } strong { (compact_zodiac(position.longitude)) } small { "H" (position.house) } }
                    }
                }
            }
        }
    }
}

fn chart_select(
    name: &str,
    label: &str,
    charts: &[ChartSummary],
    selected: Option<&str>,
) -> Markup {
    html! {
        label class="field relationship-chart-field" {
            span { (label) }
            select name=(name) required {
                option value="" { "Select a saved chart…" }
                @for chart in charts {
                    option value=(chart.id) selected[selected.is_some_and(|id| id == chart.id.to_string())] { (&chart.title) " · " (&chart.local_date) }
                }
            }
        }
    }
}

fn method_choice<'a>(value: &'a str, label: &'a str, selected: &str) -> Markup {
    html! { option value=(value) selected[value == selected] { (label) } }
}

fn point_label(point: PointId) -> String {
    match point {
        PointId::Planet(planet) => format!("{} {}", planet.glyph(), planet.name()),
        PointId::Ascendant => "Ascendant".to_owned(),
        PointId::Midheaven => "Midheaven".to_owned(),
        PointId::LotFortune => "Lot of Fortune".to_owned(),
        PointId::LotSpirit => "Lot of Spirit".to_owned(),
    }
}
