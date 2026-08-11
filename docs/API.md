# Internal desktop request contract

These routes are dispatched inside the Tauri process through the private
`meridian` custom protocol. Meridian does not bind a TCP socket, publish an
HTTP service, or accept requests from another process. The HTTP-shaped request
and response types remain an internal adapter because they allow the existing
Rust-rendered forms, exports, and focused route tests to share one contract.
JSON failures return a structured `{ "error", "status" }` body.

## Charts

`POST /api/v1/calculate` accepts either a `ChartRequest` or
`{ "chart": ChartRequest, "orb_policy": OrbPolicy }` and returns a calculated
`Chart`. `POST /api/v1/charts` accepts the same payload and returns a persisted
`ChartRecord` with a UUIDv7 identifier. The archive endpoints are:

- `GET /api/v1/charts`
- `GET /api/v1/charts/{id}`
- `DELETE /api/v1/charts/{id}`

Desktop chart creation resolves the submitted GeoNames `city_id` against the
bundled local atlas. Unless an explicit manual override is enabled, posted
coordinate or time-zone fields are ignored and the canonical atlas values are
stored in the chart request.

## Locations

`GET /api/v1/locations?q={name}&limit={count}` searches canonical, ASCII, and
alternate city names together with country and first-level administrative
names. Queries shorter than two characters return no results; `limit` is
capped at 20. Results include the stable GeoNames identifier, display name,
WGS84 coordinates, elevation, population, and IANA time zone.

## Timing

`GET /api/v1/charts/{id}/timing` requires `technique` and `target=YYYY-MM-DD`.
Supported technique keys are `transits`, `secondary`, `solar_arc`,
`solar_return`, `lunar_return`, `profection`, `firdaria`, `harmonic`, and
`planetary_hours`. Transits also require `end`. Optional parameters are `age`,
`harmonic`, `location`, `latitude`, `longitude`, `elevation`, and `houses`.
Coordinates and houses relocate return charts and planetary hours; omitted
values retain the radix place.

## Relationships

`GET /api/v1/relationships?first={id}&second={id}&method={method}` accepts
`synastry`, `composite`, or `davison`. The corresponding desktop workspace can
download a standalone SVG from `/tools/relationships.svg` with the same query.

## Ephemeris and events

`GET /api/v1/ephemeris?start_jd={jd}&rows={count}&step={days}` returns a
septenary table. `GET /api/v1/events?start_jd={jd}&end_jd={jd}` returns exact
ingresses, stations, new/full moons, and solar/lunar eclipses. The desktop
ephemeris route also exports tidy CSV.

## Elections

`POST /api/v1/elections` accepts `ElectionRequest`: a UT interval, step in
minutes, location, traditional house system, topic, and result limit. The
response retains the full chart and every scored testimony for each candidate.
Intervals are capped at 366 days and 50,000 evaluated charts.
