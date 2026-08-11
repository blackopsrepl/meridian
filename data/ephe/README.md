# Ephemeris data directory

Run `tools/fetch-ephemeris --current` for the 1800–2399 coefficient set or
`tools/fetch-ephemeris --all` for the complete long-range planetary and lunar
set. Downloads are pinned to Swiss Ephemeris revision
`3fd0f956d73898b91cc4f67cf18b21af656d1342`; binary data is intentionally not
stored in this Git repository.

Meridian requests the Swiss-file backend and verifies the backend flag returned
for every body. A missing out-of-range file therefore produces a calculation
error instead of silently returning a lower-precision analytical result.

