# Architecture

Meridian keeps astronomy, astrological doctrine, native presentation,
persistence, and file export as explicit boundaries:

1. `astro::ephemeris` converts civil instants and locations into high-precision
   apparent positions and house cusps. It is the only module allowed to depend
   on `swisseph-rs`.
2. `astro::chart` builds immutable septenary charts from those positions.
3. Doctrine modules derive aspects, dignities, lots, receptions, elections,
   relationships, and timing techniques without performing file or network
   I/O.
4. `render` projects chart models into self-contained SVG and export tables.
5. `locations` loads the bundled GeoNames snapshot, ranks alternate-name
   search, and resolves selected cities into canonical coordinates and IANA
   time zones.
6. `store` owns the SQLite archive. Stored inputs remain alongside engine
   version and calculation settings so charts remain auditable.
7. `document` owns the optional versioned `.meridian` file format.
8. `desktop` is an Iced widget application. It draws the interactive chart with
   an Iced canvas and calls the domain modules directly; there is no HTML, CSS,
   WebView, route, or network listener.

The `meridian-cli` binary reuses the calculation and export layers directly.
The provider boundary remains deliberately narrow: missing high-precision data
is an error, never an invitation to substitute Moshier or a remote service.
