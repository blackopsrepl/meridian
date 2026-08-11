use maud::{Markup, html};

use crate::astro::TraditionalHouseSystem;

use super::layout::page;

pub fn new_chart_page(default_purpose: &str) -> Markup {
    page(
        "Cast a chart",
        "new",
        html! {
            section class="page-heading form-heading" {
                div {
                    p class="eyebrow" { "New calculation" }
                    h1 { "Cast a chart" }
                    p class="lede" { "Record the civil time exactly as it was observed. Meridian resolves it to UT and preserves both values." }
                }
                div class="precision-seal" { span { "0.001″" } small { "Coefficient precision" } }
            }
            form class="chart-form" method="post" action="/charts" data-chart-form {
                section class="form-card" {
                    header class="form-card-header" {
                        span class="step-number" { "01" }
                        div { h2 { "Chart identity" } p { "Name the radix and choose its purpose." } }
                    }
                    div class="form-grid two" {
                        label class="field span-two" {
                            span { "Chart title" }
                            input type="text" name="title" maxlength="160" required placeholder="e.g. Ada Lovelace" autocomplete="off";
                        }
                        label class="field" {
                            span { "Purpose" }
                            select name="purpose" {
                                (option("natal", "Natal", default_purpose))
                                (option("event", "Event / mundane", default_purpose))
                                (option("horary", "Horary", default_purpose))
                                (option("electional", "Electional", default_purpose))
                                (option("mundane", "Ingress / mundane", default_purpose))
                            }
                        }
                        label class="field" {
                            span { "House system" }
                            select name="house_system" {
                                @for system in TraditionalHouseSystem::ALL {
                                    option value=(house_value(system)) selected[system == TraditionalHouseSystem::WholeSign] { (system.name()) }
                                }
                            }
                        }
                    }
                }
                section class="form-card" {
                    header class="form-card-header" {
                        span class="step-number" { "02" }
                        div { h2 { "Civil date and time" } p { "Signed years and the Julian calendar remain available for historical work." } }
                    }
                    div class="form-grid date-grid" {
                        label class="field" { span { "Year" } input type="number" name="year" value="2000" min="-12999" max="16999" required; }
                        label class="field" { span { "Month" } input type="number" name="month" value="1" min="1" max="12" required; }
                        label class="field" { span { "Day" } input type="number" name="day" value="1" min="1" max="31" required; }
                        label class="field" { span { "Local time" } input type="time" name="time" value="12:00" step="1" required; }
                        label class="field" {
                            span { "Calendar" }
                            select name="calendar" { option value="gregorian" { "Gregorian" } option value="julian" { "Julian" } }
                        }
                    }
                    div class="segmented-control" role="group" aria-label="Time zone mode" {
                        label { input type="radio" name="zone_mode" value="iana" checked data-zone-mode; span { "IANA zone" } }
                        label { input type="radio" name="zone_mode" value="fixed" data-zone-mode; span { "Fixed historical offset" } }
                    }
                    div class="form-grid two zone-panel" data-zone-panel="iana" {
                        label class="field" {
                            span { "IANA time zone" }
                            input type="text" name="timezone" value="Europe/Rome" placeholder="Europe/Rome" autocomplete="off";
                            small { "Historical daylight-saving rules are applied." }
                        }
                        label class="field" {
                            span { "Repeated-time fold" }
                            select name="fold" { option value="" { "Reject ambiguity" } option value="0" { "First occurrence" } option value="1" { "Second occurrence" } }
                            small { "Only needed during a backward clock change." }
                        }
                    }
                    div class="form-grid one zone-panel hidden" data-zone-panel="fixed" {
                        label class="field" {
                            span { "Minutes east of UTC" }
                            input type="number" name="fixed_offset_minutes" value="0" min="-1440" max="1440";
                            small { "Use negative values west of Greenwich. Local mean time is accepted." }
                        }
                    }
                }
                section class="form-card" {
                    header class="form-card-header" {
                        span class="step-number" { "03" }
                        div { h2 { "Terrestrial place" } p { "Coordinates drive the angles and houses; no network atlas is consulted." } }
                    }
                    div class="form-grid location-grid" {
                        label class="field span-two" { span { "Place name" } input type="text" name="location_name" value="Bergamo, Italy" maxlength="160" required; }
                        label class="field" { span { "Latitude" } input type="number" name="latitude" value="45.6983" min="-90" max="90" step="0.000001" required; small { "North positive" } }
                        label class="field" { span { "Longitude" } input type="number" name="longitude" value="9.6773" min="-180" max="180" step="0.000001" required; small { "East positive" } }
                        label class="field" { span { "Elevation (m)" } input type="number" name="elevation_m" value="249" min="-500" max="10000" step="1"; }
                    }
                }
                details class="form-card advanced-options" {
                    summary {
                        span class="step-number" { "04" }
                        div { h2 { "Aspect orb policy" } p { "Open to replace the classical defaults for this chart." } }
                        b { "Configure" }
                    }
                    div class="form-grid orb-grid" {
                        label class="field" { span { "Conjunction" } input type="number" name="orb_conjunction" value="8" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Sextile" } input type="number" name="orb_sextile" value="5" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Square" } input type="number" name="orb_square" value="7" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Trine" } input type="number" name="orb_trine" value="7" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Opposition" } input type="number" name="orb_opposition" value="8" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Luminary bonus" } input type="number" name="orb_luminary_bonus" value="2" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Angle maximum" } input type="number" name="orb_angle" value="5" min="0" max="30" step="0.1" required; }
                        label class="field" { span { "Lot maximum" } input type="number" name="orb_lot" value="3" min="0" max="30" step="0.1" required; }
                    }
                }
                div class="form-submit-bar" {
                    div { span class="status-light" {} p { strong { "Strict ephemeris mode" } small { "Missing coefficients stop the calculation." } } }
                    button class="button primary large" type="submit" { "Cast and save chart →" }
                }
            }
        },
    )
}

fn option<'a>(value: &'a str, label: &'a str, selected: &str) -> Markup {
    html! { option value=(value) selected[value == selected] { (label) } }
}

const fn house_value(system: TraditionalHouseSystem) -> &'static str {
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
