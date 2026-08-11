# Local city atlas

Run `tools/fetch-geonames` to install the GeoNames `cities500` world catalog,
country names, and first-level administrative names. The catalog contains all
cities with more than 500 inhabitants plus administrative seats down to PPLA4.
It is downloaded during `make setup` and intentionally excluded from Git.

Meridian uses the local snapshot for autocomplete and copies the resolved name,
WGS84 coordinates, elevation, and IANA time zone into every chart request. No
GeoNames web-service call occurs while the application is running.

GeoNames data is licensed under Creative Commons Attribution 4.0. See `NOTICE`.
