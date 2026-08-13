# Native data contract

Meridian has no HTTP API, web route, custom protocol, or browser transport. The
Iced desktop widgets call the Rust calculation, location, persistence, timing,
relationship, election, and export modules directly. `meridian-cli` uses those
same modules without starting the window system.

## Chart requests

`ChartRequest` is the complete calculation input. It records the chart title
and purpose, civil date and time, Gregorian or Julian calendar, IANA time zone
or fixed offset, location name, coordinates, elevation, and traditional house
system. `OrbPolicy` records the aspect-orb settings separately.

Desktop city search resolves a selected GeoNames identifier against the bundled
atlas. Unless manual location entry is enabled, the atlas coordinates,
elevation, and IANA time zone are used as one canonical location record.

## Portable chart documents

`.meridian` files contain a versioned `ChartDocument` serialized as JSON. A
document retains the original `ChartRequest`, resolved UT instant, calculation
provenance, house cusps, positions, aspects, dignities, lots, and derived
points. Opening a document does not recalculate it.

The desktop registers `application/x-meridian-chart` on Linux and the
`.meridian` extension on Windows and macOS. It also provides explicit **Open**,
**Save**, and **Save As** commands.

## Local archive

`Store` owns the per-user SQLite archive. A newly calculated chart and an
election candidate opened as a chart are inserted automatically. The archive
stores the complete `Chart`, engine version, coefficient revision, and creation
time under a UUIDv7 identifier. Archive operations are list, retrieve, insert,
and delete; no database connection is exposed outside the process.

## Locations

`CityIndex` searches canonical, ASCII, and alternate city names together with
country and first-level administrative names. Queries shorter than two
characters return no results. Each match includes its stable GeoNames ID,
display name, WGS84 coordinates, elevation, population, and IANA time zone.

## Timing and relationships

The timing layer accepts an open radix chart plus the selected technique and
its typed settings. It supports transits, secondary progressions, solar arc,
solar and lunar returns, profection, firdaria, harmonics, and planetary hours.
Return charts and planetary hours can use a relocated place.

Relationship calculation consumes two complete chart documents and applies
synastry, circular-midpoint composite, or Davison calculation. It never looks
up charts by a remote identifier.

## Exports

SVG, JSON, and CSV are projections of an already calculated chart. Exporters do
not call the ephemeris or modify the SQLite archive. Ephemeris CSV and
relationship SVG follow the same rule.
