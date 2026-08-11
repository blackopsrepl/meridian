use maud::{Markup, html};

use crate::astro::{Chart, PlanetPosition, ZodiacSign};

pub fn planet_rows(chart: &Chart) -> Markup {
    html! {
        @for position in &chart.positions {
            tr {
                td class="planet-name-cell" {
                    span class="table-glyph" { (position.planet.glyph()) }
                    strong { (position.planet.name()) }
                }
                td { (zodiac_position(position)) }
                td { "H" (position.house) }
                td class={ "motion " @if position.retrograde { "retrograde" } } {
                    @if position.retrograde { "Retrograde" } @else { "Direct" }
                }
                td { (format!("{:+.3}°/d", position.speed_longitude)) }
            }
        }
    }
}

pub fn mini_planet_list(chart: &Chart) -> Markup {
    html! {
        div class="mini-planet-list" {
            @for position in &chart.positions {
                div class="mini-planet" {
                    span class="mini-glyph" { (position.planet.glyph()) }
                    span class="mini-name" { (position.planet.name()) }
                    strong { (compact_zodiac(position.longitude)) }
                    @if position.retrograde { span class="retrograde-mark" { "℞" } }
                }
            }
        }
    }
}

pub fn zodiac_position(position: &PlanetPosition) -> String {
    let degree = position.degree_in_sign.floor();
    let minutes_float = position.degree_in_sign.fract() * 60.0;
    let minutes = minutes_float.floor();
    let seconds = ((minutes_float - minutes) * 60.0).round();
    format!(
        "{} {:02.0}° {:02.0}′ {:02.0}″",
        position.sign.glyph(),
        degree,
        minutes,
        seconds
    )
}

pub fn compact_zodiac(longitude: f64) -> String {
    let sign = ZodiacSign::from_longitude(longitude);
    let degree = longitude.rem_euclid(30.0);
    format!(
        "{} {:02.0}°{:02.0}′",
        sign.glyph(),
        degree.floor(),
        degree.fract() * 60.0
    )
}

pub fn local_date_label(chart: &Chart) -> String {
    let value = &chart.request.local_time;
    let era = if value.year <= 0 { " BCE" } else { "" };
    let year = if value.year <= 0 {
        1 - value.year
    } else {
        value.year
    };
    format!(
        "{year:04}-{:02}-{:02}{era} · {:02}:{:02}",
        value.month, value.day, value.hour, value.minute
    )
}
