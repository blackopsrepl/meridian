use crate::astro::{Chart, EphemerisTable, PointId};

pub fn chart_csv(chart: &Chart) -> Result<Vec<u8>, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "record_type",
        "name",
        "longitude",
        "sign_or_aspect",
        "degree_or_orb",
        "house_or_phase",
        "motion",
        "extra",
    ])?;
    for position in &chart.positions {
        writer.write_record([
            "planet".to_owned(),
            position.planet.name().to_owned(),
            format!("{:.8}", position.longitude),
            position.sign.name().to_owned(),
            format!("{:.8}", position.degree_in_sign),
            position.house.to_string(),
            if position.retrograde {
                "retrograde".to_owned()
            } else {
                "direct".to_owned()
            },
            format!(
                "speed={:.8};declination={:.8}",
                position.speed_longitude, position.declination
            ),
        ])?;
    }
    for lot in &chart.lots {
        writer.write_record([
            "lot".to_owned(),
            lot.kind.name().to_owned(),
            format!("{:.8}", lot.longitude),
            lot.sign.name().to_owned(),
            format!("{:.8}", lot.degree_in_sign),
            lot.house.to_string(),
            String::new(),
            format!("ruler={}", lot.ruler.name()),
        ])?;
    }
    for aspect in &chart.aspects {
        writer.write_record([
            "aspect".to_owned(),
            format!("{} — {}", point_name(aspect.left), point_name(aspect.right)),
            String::new(),
            aspect.kind.name().to_owned(),
            format!("{:.8}", aspect.orb),
            format!("{:?}", aspect.phase).to_lowercase(),
            String::new(),
            if aspect.partile {
                "partile".to_owned()
            } else {
                String::new()
            },
        ])?;
    }
    writer.flush()?;
    writer
        .into_inner()
        .map_err(|error| error.into_error().into())
}

pub fn ephemeris_csv(table: &EphemerisTable) -> Result<Vec<u8>, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "jd_ut",
        "planet",
        "longitude",
        "sign",
        "degree_in_sign",
        "speed_longitude",
        "motion",
        "declination",
    ])?;
    for row in &table.rows {
        for position in &row.positions {
            writer.write_record([
                format!("{:.10}", row.jd_ut),
                position.planet.name().to_owned(),
                format!("{:.8}", position.longitude),
                position.sign.name().to_owned(),
                format!("{:.8}", position.degree_in_sign),
                format!("{:.8}", position.speed_longitude),
                if position.retrograde {
                    "retrograde".to_owned()
                } else {
                    "direct".to_owned()
                },
                format!("{:.8}", position.declination),
            ])?;
        }
    }
    writer.flush()?;
    writer
        .into_inner()
        .map_err(|error| error.into_error().into())
}

const fn point_name(point: PointId) -> &'static str {
    point.name()
}

#[cfg(test)]
mod tests {
    use super::{chart_csv, ephemeris_csv};
    use crate::astro::{
        Calendar, ChartCalculator, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
        EphemerisTable, SwissEphemerisProvider, TimeZoneSpec, TraditionalHouseSystem,
    };

    #[test]
    fn csv_contains_the_seven_planets() -> Result<(), Box<dyn std::error::Error>> {
        let chart = ChartCalculator::new(SwissEphemerisProvider::new("data/ephe")?).calculate(
            ChartRequest {
                title: "Export".to_owned(),
                purpose: ChartPurpose::Event,
                local_time: CivilDateTime {
                    year: 2000,
                    month: 1,
                    day: 1,
                    hour: 12,
                    minute: 0,
                    second: 0.0,
                    calendar: Calendar::Gregorian,
                },
                time_zone: TimeZoneSpec::FixedOffset {
                    minutes_east: 0,
                    label: None,
                },
                location_name: "Greenwich".to_owned(),
                coordinates: Coordinates {
                    latitude: 51.4779,
                    longitude: 0.0,
                    elevation_m: 46.0,
                },
                house_system: TraditionalHouseSystem::WholeSign,
            },
        )?;
        let csv = String::from_utf8(chart_csv(&chart)?)?;
        assert_eq!(
            csv.lines()
                .filter(|line| line.starts_with("planet,"))
                .count(),
            7
        );
        assert!(!csv.contains("Uranus"));
        Ok(())
    }

    #[test]
    fn ephemeris_csv_has_seven_rows_per_instant() -> Result<(), Box<dyn std::error::Error>> {
        let provider = SwissEphemerisProvider::new("data/ephe")?;
        let table = EphemerisTable::calculate(&provider, 2_451_545.0, 2, 1.0)?;
        let csv = String::from_utf8(ephemeris_csv(&table)?)?;
        assert_eq!(csv.lines().count(), 15);
        assert!(!csv.contains("Uranus"));
        Ok(())
    }
}
