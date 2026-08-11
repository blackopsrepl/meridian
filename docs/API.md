# API reference

All endpoints are local by default, use JSON unless explicitly documented, and
return a structured `{ "error", "status" }` body on API failure.

## Charts

`POST /api/v1/calculate` accepts either a `ChartRequest` or
`{ "chart": ChartRequest, "orb_policy": OrbPolicy }` and returns a calculated
`Chart`. `POST /api/v1/charts` accepts the same payload and returns a persisted
`ChartRecord` with a UUIDv7 identifier. The archive endpoints are:

- `GET /api/v1/charts`
- `GET /api/v1/charts/{id}`
- `DELETE /api/v1/charts/{id}`

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
`synastry`, `composite`, or `davison`. The corresponding browser route can
download a standalone SVG from `/tools/relationships.svg` with the same query.

## Ephemeris and events

`GET /api/v1/ephemeris?start_jd={jd}&rows={count}&step={days}` returns a
septenary table. `GET /api/v1/events?start_jd={jd}&end_jd={jd}` returns exact
ingresses, stations, new/full moons, and solar/lunar eclipses. The browser
ephemeris route also exports tidy CSV.

## Elections

`POST /api/v1/elections` accepts `ElectionRequest`: a UT interval, step in
minutes, location, traditional house system, topic, and result limit. The
response retains the full chart and every scored testimony for each candidate.
Intervals are capped at 366 days and 50,000 evaluated charts.
