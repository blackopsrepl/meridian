use maud::{Markup, PreEscaped, html};

use crate::astro::{Chart, PointId};
use crate::render::{WheelOptions, render_wheel};
use crate::store::ChartRecord;

use super::components::{local_date_label, planet_rows};
use super::layout::page;

pub fn chart_page(record: &ChartRecord) -> Markup {
    let chart = &record.chart;
    page(
        &record.title,
        "",
        html! {
            section class="chart-titlebar" {
                div {
                    p class="eyebrow" { (format!("{:?}", chart.request.purpose)) " chart" }
                    h1 { (&record.title) }
                    p class="chart-subtitle" {
                        (local_date_label(chart)) " · " (&chart.request.location_name)
                        " · " (chart.request.house_system.name())
                    }
                }
                div class="title-actions" {
                    a class="button secondary compact" href=(format!("/charts/{}/wheel.svg", record.id)) { "SVG" }
                    a class="button secondary compact" href=(format!("/charts/{}/data.csv", record.id)) { "CSV" }
                    a class="button secondary compact" href=(format!("/api/v1/charts/{}", record.id)) { "JSON" }
                    button class="button primary compact" type="button" data-print { "Print report" }
                }
            }
            nav class="chart-tabs" aria-label="Chart sections" {
                a href="#wheel" class="active" { "Wheel" }
                a href="#positions" { "Positions" }
                a href="#condition" { "Condition" }
                a href="#aspects" { "Aspects" }
                a href="#lots" { "Lots" }
                a href="#provenance" { "Provenance" }
            }
            section id="wheel" class="chart-hero-grid" {
                article class="panel wheel-panel" {
                    div class="wheel-stage detail-wheel" {
                        (PreEscaped(render_wheel(chart, WheelOptions::default())))
                    }
                }
                aside class="chart-summary-stack" {
                    article class="panel summary-card primary-summary" {
                        p class="eyebrow" { "Chart signature" }
                        div class="signature-row" {
                            span { "Ascendant" }
                            strong { (super::components::compact_zodiac(chart.houses.ascendant)) }
                        }
                        div class="signature-row" {
                            span { "Midheaven" }
                            strong { (super::components::compact_zodiac(chart.houses.midheaven)) }
                        }
                        div class="signature-row" {
                            span { "Sect" }
                            strong { (format!("{:?} ({:+.2}° Sun)", chart.sect, chart.solar_altitude)) }
                        }
                        div class="signature-row" {
                            span { "Chart ruler" }
                            strong { (chart.chart_ruler.glyph()) " " (chart.chart_ruler.name()) }
                        }
                        div class="signature-row" {
                            span { "Almuten" }
                            strong { (winner_names(chart)) }
                        }
                    }
                    article class="panel summary-card" {
                        p class="eyebrow" { "Lunation" }
                        div class="lunar-phase-mark" { "☽" }
                        h3 { (format!("{:?}", chart.lunar_phase)) " Moon" }
                        p { (format!("{:.2}° elongation from the Sun", chart.lunar_elongation)) }
                    }
                    article class="panel source-card" {
                        span class="status-light" {}
                        div { strong { "Coefficient-backed" } p { "Every planetary row returned the Swiss-file source flag." } }
                    }
                }
            }
            section id="positions" class="report-section panel" {
                (section_heading("Celestial positions", "Apparent geocentric ecliptic positions with daily motion and equatorial declination."))
                div class="table-wrap" {
                    table class="data-table position-table" {
                        thead { tr { th { "Planet" } th { "Longitude" } th { "House" } th { "Motion" } th { "Daily speed" } } }
                        tbody { (planet_rows(chart)) }
                    }
                }
            }
            section id="condition" class="report-section" {
                (section_heading("Classical condition", "Essential dignity uses domicile, exaltation, active triplicity, Egyptian term, and Chaldean face."))
                div class="condition-grid" {
                    @for condition in &chart.conditions {
                        article class="condition-card" {
                            header {
                                span class="condition-glyph" { (condition.planet.glyph()) }
                                div { h3 { (condition.planet.name()) } p { (format!("{:?}", condition.combustion)) } }
                                strong class={ "score " @if condition.total_score >= 0 { "positive" } @else { "negative" } } { (format!("{:+}", condition.total_score)) }
                            }
                            div class="dignity-list" {
                                @if condition.essential.is_empty() { span class="muted-chip" { "Peregrine" } }
                                @for dignity in &condition.essential { span { (humanize(&format!("{dignity:?}"))) } }
                            }
                            footer {
                                span { "Essential " strong { (format!("{:+}", condition.essential_score)) } }
                                span { "Accidental " strong { (format!("{:+}", condition.accidental_score)) } }
                            }
                        }
                    }
                }
            }
            section id="aspects" class="report-section panel" {
                (section_heading("Ptolemaic aspects", "Only conjunction, sextile, square, trine, and opposition are admitted."))
                div class="aspect-list" {
                    @for aspect in &chart.aspects {
                        div class="aspect-row" {
                            span class=(format!("aspect-glyph {:?}", aspect.kind).to_lowercase()) { (aspect.kind.glyph()) }
                            strong { (point_name(aspect.left)) }
                            span class="aspect-line-label" { (aspect.kind.name()) }
                            strong { (point_name(aspect.right)) }
                            span class="aspect-orb" { (format!("{:.2}°", aspect.orb)) }
                            span class="phase-chip" { (humanize(&format!("{:?}", aspect.phase))) }
                        }
                    }
                }
            }
            section id="lots" class="report-section panel" {
                (section_heading("Hermetic lots", "Sect-aware formulas derived from the Ascendant and the seven planetary significators."))
                div class="lots-grid" {
                    @for lot in &chart.lots {
                        article class="lot-card" {
                            span class="lot-symbol" { @if lot.kind == crate::astro::LotKind::Fortune { "⊗" } @else { "◇" } }
                            div { h3 { (lot.kind.name()) } p { (super::components::compact_zodiac(lot.longitude)) " · House " (lot.house) } }
                            span class="ruler-label" { "Ruler " (lot.ruler.glyph()) " " (lot.ruler.name()) }
                        }
                    }
                }
            }
            section id="provenance" class="report-section provenance-panel" {
                (section_heading("Calculation provenance", "Stored with the chart so its numerical contract remains inspectable."))
                dl class="provenance-grid" {
                    dt { "Application" } dd { "Meridian " (&chart.metadata.application_version) }
                    dt { "Engine" } dd { (&chart.metadata.engine_version) }
                    dt { "Coefficient revision" } dd { code { (&chart.metadata.data_revision) } }
                    dt { "Source" } dd { (&chart.metadata.ephemeris_source) }
                    dt { "Zodiac" } dd { (&chart.metadata.zodiac) }
                    dt { "Planet set" } dd { (&chart.metadata.planet_set) }
                    dt { "Ptolemaic orbs" } dd { code { (format!("☌ {:.1} · ⚹ {:.1} · □ {:.1} · △ {:.1} · ☍ {:.1}", chart.orb_policy.conjunction, chart.orb_policy.sextile, chart.orb_policy.square, chart.orb_policy.trine, chart.orb_policy.opposition)) } }
                    dt { "Point modifiers" } dd { code { (format!("luminary +{:.1} · angles ≤{:.1} · lots ≤{:.1}", chart.orb_policy.luminary_bonus, chart.orb_policy.angle_orb, chart.orb_policy.lot_orb)) } }
                    dt { "Julian day UT" } dd { code { (format!("{:.10}", chart.moment.jd_ut)) } }
                    dt { "Resolved offset" } dd { (format!("{:+} minutes", chart.moment.offset_minutes)) }
                }
            }
            section class="danger-zone no-print" {
                div { h2 { "Archive controls" } p { "Deleting a chart removes its retained inputs and calculated snapshot." } }
                form method="post" action=(format!("/charts/{}/delete", record.id)) data-confirm="Delete this chart permanently?" {
                    button class="button danger compact" type="submit" { "Delete chart" }
                }
            }
        },
    )
}

fn section_heading(title: &str, description: &str) -> Markup {
    html! { header class="report-heading" { div { p class="eyebrow" { "Data sheet" } h2 { (title) } } p { (description) } } }
}

fn winner_names(chart: &Chart) -> String {
    chart
        .almuten
        .winners
        .iter()
        .map(|planet| format!("{} {}", planet.glyph(), planet.name()))
        .collect::<Vec<_>>()
        .join(" / ")
}

const fn point_name(point: PointId) -> &'static str {
    point.name()
}

fn humanize(value: &str) -> String {
    let mut result = String::new();
    for (index, character) in value.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            result.push(' ');
        }
        result.push(character);
    }
    result
}
