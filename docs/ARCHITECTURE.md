# Architecture

Meridian keeps astronomy, astrological doctrine, presentation, and persistence
as explicit boundaries:

1. `astro::ephemeris` converts civil instants and locations into high-precision
   apparent positions and house cusps. It is the only module allowed to depend
   on `swisseph-rs`.
2. `astro::chart` builds immutable septenary charts from those positions.
3. Doctrine modules derive aspects, dignities, lots, receptions, and timing
   techniques without performing file or network I/O.
4. `render` turns chart models into SVG and export tables.
5. `locations` loads the local GeoNames snapshot, ranks alternate-name search,
   and resolves browser-selected identifiers into canonical coordinates and
   IANA time zones.
6. `store` owns SQLite persistence. Stored inputs are retained alongside engine
   version and calculation settings so charts remain auditable.
7. `web` exposes HTML and versioned JSON endpoints over the same application
   service used by the CLI.

The provider boundary is intentionally narrow. Missing high-precision data is
an error, never an invitation to substitute Moshier or a remote service.
