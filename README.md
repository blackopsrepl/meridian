# Meridian

Meridian is a desktop classical astrology workbench written in Rust. It is
deliberately limited to the traditional septenary—Sun, Moon, Mercury, Venus,
Mars, Jupiter, and Saturn—while retaining professional chart calculation,
comparison, forecasting, ephemeris, and export workflows.

The numerical layer reads official Swiss Ephemeris `.se1` coefficient files
through the stateless pure-Rust `swisseph-rs` engine. Meridian never calls a
remote astrology service and rejects silent fallback to an analytical
ephemeris when high-precision mode is requested.

## Install

Download the installer for your operating system from the latest GitHub
release:

- Windows: the signed `-setup.exe` installer
- macOS: the signed and notarized universal `.dmg`
- Linux: the signed `.rpm` for Fedora, RHEL-compatible distributions, and
  openSUSE; `.deb` for Debian/Ubuntu; or `.AppImage` for portable use

Every installer contains the complete pinned Swiss Ephemeris coefficient set
and the verified GeoNames atlas. Meridian does not contact an astrology or
location service and remains fully functional without a network connection.
On Windows 10 and 11, the required WebView2 runtime is normally already part of
the operating system. If it is absent or too old, the installer visibly runs
Microsoft's WebView2 bootstrapper before Meridian starts.

Meridian stores the chart archive in the operating system's per-user
application-data directory. Application resources remain read-only, so
upgrading or uninstalling the program does not silently mix user data with the
installation.

## Develop locally

Requirements: Rust 1.95, Tauri CLI 2.11.4, `curl`, `unzip`, `sha256sum`, and
the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
make setup
make run
```

`make setup` installs the complete long-range coefficient set (about 100 MB)
and the local GeoNames world-city atlas. The current atlas snapshot contains
more than 200,000 cities and administrative seats with alternate names,
coordinates, elevation, population, and IANA time zones.
For a faster present-day development setup, run `make data-current` followed by
`cargo build --locked`; that smaller set covers 1800–2399. The desktop process
opens its own window and does not bind a TCP port or launch an external browser.
Set `MERIDIAN_DATABASE`, `MERIDIAN_EPHE_PATH`, or `MERIDIAN_CITY_PATH` to
override the desktop paths during development.

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

Calculate without starting the desktop application:

```sh
cargo run --locked --bin meridian-cli -- examples/chart-request.json --format json
cargo run --locked --bin meridian-cli -- examples/chart-request.json --format svg --output chart.svg
cargo run --locked --bin meridian-cli -- examples/chart-request.json --format csv --output chart.csv
```

All formats are derived from the same immutable `Chart`; exporters never
recalculate positions.

## Desktop transport

The desktop UI and its JSON/form transport use a private Tauri custom protocol.
They are never exposed through a network listener. `docs/API.md` documents that
internal contract for tests and integrations inside the desktop process.

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

`docs/DISTRIBUTION.md` defines the verified GitHub release process, offline data
contract, signing credentials, and platform artifacts.

## Licensing

Meridian is AGPL-3.0-or-later because the default calculation engine is a
derivative of Swiss Ephemeris. A proprietary deployment must obtain the
appropriate Swiss Ephemeris Professional License and replace or relicense that
provider before distribution or public service activation.
