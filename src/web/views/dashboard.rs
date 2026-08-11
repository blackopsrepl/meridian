use maud::{Markup, PreEscaped, html};

use crate::astro::Chart;
use crate::render::{WheelOptions, render_wheel};
use crate::store::ChartSummary;

use super::components::{local_date_label, mini_planet_list};
use super::layout::page;

pub fn dashboard_page(current: &Chart, recent: &[ChartSummary]) -> Markup {
    page(
        "Observatory",
        "overview",
        html! {
            section class="page-heading dashboard-heading" {
                div {
                    p class="eyebrow" { "Live sky · Greenwich" }
                    h1 { "The classical sky, now." }
                    p class="lede" { "Seven visible planets. No modern bodies, no remote calculation, no hidden fallback." }
                }
                div class="heading-metrics" {
                    div { strong { (current.sect_label()) } span { "Sect" } }
                    div { strong { (format!("{:?}", current.lunar_phase)) } span { "Moon phase" } }
                    div { strong { (current.request.house_system.name()) } span { "Houses" } }
                }
            }
            section class="dashboard-grid" {
                article class="panel sky-panel" {
                    header class="panel-header" {
                        div {
                            p class="eyebrow" { "Chart of the moment" }
                            h2 { (local_date_label(current)) }
                        }
                        span class="live-badge" { span {} " Live" }
                    }
                    div class="sky-content" {
                        div class="wheel-stage dashboard-wheel" {
                            (PreEscaped(render_wheel(current, WheelOptions { size: 700, show_aspects: true, show_lots: false })))
                        }
                        div class="sky-ledger" {
                            div class="ledger-intro" {
                                span { "ASC" }
                                strong { (super::components::compact_zodiac(current.houses.ascendant)) }
                            }
                            (mini_planet_list(current))
                            a class="text-link" href="/charts/new" { "Cast this moment elsewhere →" }
                        }
                    }
                }
                aside class="dashboard-side" {
                    article class="panel quick-panel" {
                        header class="panel-header" {
                            div { p class="eyebrow" { "Begin" } h2 { "Cast a chart" } }
                        }
                        div class="quick-actions" {
                            (quick_link("/charts/new?purpose=natal", "☉", "Natal", "Birth and radix"))
                            (quick_link("/charts/new?purpose=horary", "?", "Horary", "Question of the hour"))
                            (quick_link("/charts/new?purpose=event", "✦", "Event", "Mundane or ingress"))
                            (quick_link("/tools/relationships", "⋈", "Compare", "Synastry and composite"))
                        }
                    }
                    article class="panel doctrine-panel" {
                        p class="eyebrow" { "Doctrine" }
                        h2 { "A closed planetary system" }
                        p { "Every rulership, dignity, aspect, lot, and time lord resolves to the traditional seven." }
                        div class="septenary-strip" aria-label="The seven classical planets" {
                            @for position in &current.positions {
                                span title=(position.planet.name()) { (position.planet.glyph()) }
                            }
                        }
                    }
                }
            }
            section class="section-block" {
                header class="section-header" {
                    div { p class="eyebrow" { "Archive" } h2 { "Recent charts" } }
                    a class="button secondary compact" href="/charts/new" { "New chart" }
                }
                @if recent.is_empty() {
                    div class="empty-state" {
                        div class="empty-orbit" { "☉" }
                        h3 { "No saved charts yet" }
                        p { "Your first calculation will appear here with its exact inputs and engine provenance." }
                        a class="button primary" href="/charts/new" { "Cast the first chart" }
                    }
                } @else {
                    div class="recent-grid" {
                        @for chart in recent {
                            a class="chart-card" href=(format!("/charts/{}", chart.id)) {
                                span class="chart-card-glyph" { "◎" }
                                div { h3 { (&chart.title) } p { (&chart.location_name) " · " (&chart.local_date) } }
                                span class="purpose-chip" { (&chart.purpose) }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn quick_link(href: &str, glyph: &str, title: &str, description: &str) -> Markup {
    html! {
        a class="quick-link" href=(href) {
            span class="quick-glyph" { (glyph) }
            span { strong { (title) } small { (description) } }
            b { "→" }
        }
    }
}

trait SectLabel {
    fn sect_label(&self) -> &'static str;
}

impl SectLabel for Chart {
    fn sect_label(&self) -> &'static str {
        match self.sect {
            crate::astro::Sect::Day => "Diurnal",
            crate::astro::Sect::Night => "Nocturnal",
        }
    }
}
