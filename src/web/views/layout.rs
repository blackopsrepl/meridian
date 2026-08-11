use maud::{DOCTYPE, Markup, html};

pub fn page(title: &str, active: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="theme-color" content="#151b22";
                meta name="color-scheme" content="light";
                meta http-equiv="Content-Security-Policy" content="default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'";
                title { (title) " · Meridian" }
                link rel="stylesheet" href="/assets/app.css";
                script src="/assets/app.js" defer {}
            }
            body {
                a class="skip-link" href="#main-content" { "Skip to content" }
                div class="app-shell" {
                    aside class="sidebar" id="sidebar" {
                        div class="brand" {
                            a class="brand-mark" href="/" aria-label="Meridian home" { "M" }
                            div {
                                a class="brand-name" href="/" { "Meridian" }
                                span { "Classical observatory" }
                            }
                        }
                        nav class="primary-nav" aria-label="Primary navigation" {
                            (nav_link("/", "overview", "⌾", "Observatory", active))
                            (nav_link("/charts/new", "new", "+", "Cast a chart", active))
                            p class="nav-section" { "Workspaces" }
                            (nav_link("/tools/ephemeris", "ephemeris", "≋", "Ephemeris", active))
                            (nav_link("/tools/timing", "timing", "◷", "Timing", active))
                            (nav_link("/tools/relationships", "relationships", "⋈", "Relationships", active))
                            (nav_link("/tools/elections", "elections", "✦", "Election lab", active))
                        }
                        div class="sidebar-status" {
                            span class="status-light" {}
                            div {
                                strong { "DE441 coefficients" }
                                span { "High precision · local" }
                            }
                        }
                    }
                    div class="workspace" {
                        header class="topbar" {
                            button class="icon-button menu-button" type="button" data-sidebar-toggle aria-label="Open navigation" { "☰" }
                            div class="topbar-context" {
                                span class="eyebrow" { "Tropical zodiac" }
                                strong { "Traditional seven" }
                            }
                            div class="topbar-actions" {
                                span class="engine-chip" { span class="status-light" {} " Ephemeris ready" }
                                a class="button primary compact" href="/charts/new" { "+ New chart" }
                            }
                        }
                        main id="main-content" class="main-content" { (content) }
                        footer class="app-footer" {
                            span { "Meridian " (env!("CARGO_PKG_VERSION")) }
                            span { "Swiss Ephemeris DE441 · AGPL-3.0-or-later" }
                        }
                    }
                }
                div class="sidebar-scrim" data-sidebar-toggle {}
            }
        }
    }
}

fn nav_link(href: &str, key: &str, icon: &str, label: &str, active: &str) -> Markup {
    let class = if active == key {
        "nav-link active"
    } else {
        "nav-link"
    };
    html! {
        a class=(class) href=(href) {
            span class="nav-icon" aria-hidden="true" { (icon) }
            span { (label) }
        }
    }
}
