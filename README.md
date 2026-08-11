# Meridian

Meridian is a full-stack classical astrology workbench written in Rust. It is
deliberately limited to the traditional septenary—Sun, Moon, Mercury, Venus,
Mars, Jupiter, and Saturn—while retaining professional chart calculation,
comparison, forecasting, ephemeris, and export workflows.

The numerical layer reads official Swiss Ephemeris `.se1` coefficient files
through the stateless pure-Rust `swisseph-rs` engine. Meridian never calls a
remote astrology service and rejects silent fallback to an analytical
ephemeris when high-precision mode is requested.

## Run locally

Requirements: Rust 1.95, `curl`, `unzip`, `sha256sum`, and standard POSIX
utilities.

```sh
make setup
make run
```

`make setup` installs the complete long-range coefficient set (about 100 MB)
and the local GeoNames world-city atlas. The current atlas snapshot contains
more than 200,000 cities and administrative seats with alternate names,
coordinates, elevation, population, and IANA time zones.
For a faster present-day development setup, run `make data-current` followed by
`cargo build --locked`; that smaller set covers 1800–2399.

Then open <http://127.0.0.1:3001>. The database defaults to
`data/meridian.sqlite3`; set `MERIDIAN_DATABASE`, `MERIDIAN_EPHE_PATH`,
`MERIDIAN_CITY_PATH`, or `MERIDIAN_BIND` to override the defaults.

## Surfaces

- High-precision tropical chart calculation with classical house systems
- Natal, mundane, horary, electional, synastry, composite, Davison, transit,
  secondary progression, solar-arc, solar-return, and lunar-return charts
- Ptolemaic aspects with configurable orbs and applying/separating state
- Essential and accidental dignity, reception, sect, lots, antiscia,
  dodecatemoria, planetary days/hours, and traditional time-lord techniques
- Date-bounded transit/event timelines and printable ephemeris tables
- Local city autocomplete that resolves canonical coordinates and historical
  time zones, with optional surveyed-coordinate and fixed-offset overrides
- SQLite chart archive retaining exact inputs, resolved time, engine revision,
  coefficient revision, house system, and orb policy
- HTML/SVG, JSON, and CSV output from the same calculation model

`docs/PARITY.md` tracks the classical subset of Astrodienst's chart surface and
the exact Meridian workflow that owns each capability.

## Command line

Calculate without starting the server:

```sh
cargo run --locked -- chart examples/chart-request.json --format json
cargo run --locked -- chart examples/chart-request.json --format svg --output chart.svg
cargo run --locked -- chart examples/chart-request.json --format csv --output chart.csv
```

All formats are derived from the same immutable `Chart`; exporters never
recalculate positions.

## HTTP API

The local service exposes versioned JSON endpoints:

- `POST /api/v1/calculate` — calculate a chart without saving it
- `GET|POST /api/v1/charts` and `GET|DELETE /api/v1/charts/{id}` — archive
- `GET /api/v1/charts/{id}/timing` — transits, returns, progressions,
  directions, profections, firdaria, harmonics, and planetary hours
- `GET /api/v1/relationships` — synastry, composite, or Davison results
- `GET /api/v1/ephemeris` and `GET /api/v1/events` — tables and exact events
- `GET /api/v1/locations?q={name}` — ranked local city and alternate-name search
- `POST /api/v1/elections` — ranked electional search with testimony ledger

`POST /api/v1/calculate` accepts the plain request in
`examples/chart-request.json`. To override the aspect policy, send an envelope:

```json
{
  "chart": { "title": "…", "purpose": "natal", "local_time": {}, "time_zone": {}, "location_name": "…", "coordinates": {}, "house_system": "whole_sign" },
  "orb_policy": {
    "conjunction": 8.0,
    "sextile": 5.0,
    "square": 7.0,
    "trine": 7.0,
    "opposition": 8.0,
    "luminary_bonus": 2.0,
    "angle_orb": 5.0,
    "lot_orb": 3.0
  }
}
```

The abbreviated nested objects above show the envelope only; use the complete
chart fields from the example file.

## Numerical contract

- Tropical, apparent geocentric positions in UT
- Swiss-file source flag required for every planetary result
- Official coefficient revision
  `3fd0f956d73898b91cc4f67cf18b21af656d1342`
- IANA historical-zone resolution for Gregorian years 1–9999; explicit fixed
  offsets and Julian dates for earlier work
- Ambiguous civil times require an explicit fold; nonexistent clock times fail
- Sect is calculated from the Sun's equatorial altitude, independent of houses
- Only Sun through Saturn can inhabit the `Planet` type
- Chart wheels are oriented with the Midheaven fixed at 12 o'clock

Run the complete quality gate with `make check`.

## Licensing

Meridian is AGPL-3.0-or-later because the default calculation engine is a
derivative of Swiss Ephemeris. A proprietary deployment must obtain the
appropriate Swiss Ephemeris Professional License and replace or relicense that
provider before distribution or public service activation.
