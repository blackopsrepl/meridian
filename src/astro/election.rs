use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    AccidentalDignity, AspectPhase, Calendar, Chart, ChartCalculator, ChartError, ChartPurpose,
    ChartRequest, CivilDateTime, Combustion, Coordinates, Planet, PointId, TimeZoneSpec,
    TraditionalHouseSystem, ZodiacSign, civil_from_julian_day,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectionTopic {
    General,
    Marriage,
    Commerce,
    Travel,
    Career,
    Home,
    Healing,
    Litigation,
}

impl ElectionTopic {
    pub const ALL: [Self; 8] = [
        Self::General,
        Self::Marriage,
        Self::Commerce,
        Self::Travel,
        Self::Career,
        Self::Home,
        Self::Healing,
        Self::Litigation,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::General => "General undertaking",
            Self::Marriage => "Marriage & union",
            Self::Commerce => "Commerce & contracts",
            Self::Travel => "Travel & journeys",
            Self::Career => "Career & honours",
            Self::Home => "Home & property",
            Self::Healing => "Healing & treatment",
            Self::Litigation => "Litigation & judgment",
        }
    }

    const fn natural_significator(self) -> Planet {
        match self {
            Self::General | Self::Litigation => Planet::Jupiter,
            Self::Marriage => Planet::Venus,
            Self::Commerce | Self::Travel => Planet::Mercury,
            Self::Career | Self::Healing => Planet::Sun,
            Self::Home => Planet::Moon,
        }
    }

    const fn topical_house(self) -> u8 {
        match self {
            Self::General | Self::Healing => 1,
            Self::Commerce => 2,
            Self::Home => 4,
            Self::Marriage | Self::Litigation => 7,
            Self::Travel => 9,
            Self::Career => 10,
        }
    }
}

impl std::fmt::Display for ElectionTopic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectionRequest {
    pub title: String,
    pub start_jd_ut: f64,
    pub end_jd_ut: f64,
    pub step_minutes: u16,
    pub location_name: String,
    pub coordinates: Coordinates,
    pub house_system: TraditionalHouseSystem,
    pub topic: ElectionTopic,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionScoreItem {
    pub label: String,
    pub score: i32,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectionCandidate {
    pub rank: usize,
    pub score: i32,
    pub chart: Chart,
    pub score_items: Vec<ElectionScoreItem>,
    pub advisories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectionSearchResult {
    pub request: ElectionRequest,
    pub evaluated_instants: usize,
    pub candidates: Vec<ElectionCandidate>,
}

#[derive(Debug, Error)]
pub enum ElectionError {
    #[error(transparent)]
    Chart(#[from] ChartError),
    #[error("invalid election search: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct ElectionSearch {
    calculator: ChartCalculator,
}

impl ElectionSearch {
    #[must_use]
    pub fn new(calculator: ChartCalculator) -> Self {
        Self { calculator }
    }

    pub fn search(&self, request: ElectionRequest) -> Result<ElectionSearchResult, ElectionError> {
        validate(&request)?;
        let step_days = f64::from(request.step_minutes) / 1_440.0;
        let mut evaluated_instants = 0_usize;
        let mut candidates = Vec::with_capacity(request.limit * 3);
        let mut jd_ut = request.start_jd_ut;
        while jd_ut <= request.end_jd_ut + 1e-7 {
            let chart = self.calculator.calculate(chart_request(&request, jd_ut))?;
            let (score, score_items, advisories) = score_chart(&chart, request.topic);
            candidates.push(ElectionCandidate {
                rank: 0,
                score,
                chart,
                score_items,
                advisories,
            });
            evaluated_instants += 1;
            jd_ut += step_days;
        }
        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.chart.moment.jd_ut.total_cmp(&right.chart.moment.jd_ut))
        });
        candidates.truncate(request.limit);
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.rank = index + 1;
        }
        Ok(ElectionSearchResult {
            request,
            evaluated_instants,
            candidates,
        })
    }
}

fn validate(request: &ElectionRequest) -> Result<(), ElectionError> {
    request
        .coordinates
        .validate()
        .map_err(|message| ElectionError::Invalid(message.to_owned()))?;
    if !request.start_jd_ut.is_finite()
        || !request.end_jd_ut.is_finite()
        || request.end_jd_ut <= request.start_jd_ut
    {
        return Err(ElectionError::Invalid(
            "end must be later than start".to_owned(),
        ));
    }
    if request.end_jd_ut - request.start_jd_ut > 366.0 {
        return Err(ElectionError::Invalid(
            "search interval may not exceed 366 days".to_owned(),
        ));
    }
    if !(5..=1_440).contains(&request.step_minutes) {
        return Err(ElectionError::Invalid(
            "step must be between 5 and 1440 minutes".to_owned(),
        ));
    }
    let count =
        (request.end_jd_ut - request.start_jd_ut) * 1_440.0 / f64::from(request.step_minutes);
    if count > 50_000.0 {
        return Err(ElectionError::Invalid(
            "search would evaluate more than 50000 charts".to_owned(),
        ));
    }
    if !(1..=50).contains(&request.limit) {
        return Err(ElectionError::Invalid(
            "candidate limit must be between 1 and 50".to_owned(),
        ));
    }
    Ok(())
}

fn chart_request(request: &ElectionRequest, jd_ut: f64) -> ChartRequest {
    let local_time = civil_from_julian_day(jd_ut, Calendar::Gregorian);
    ChartRequest {
        title: format!("{} · candidate", request.title),
        purpose: ChartPurpose::Electional,
        local_time: CivilDateTime {
            calendar: Calendar::Gregorian,
            ..local_time
        },
        time_zone: TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC · election search".to_owned()),
        },
        location_name: request.location_name.clone(),
        coordinates: request.coordinates,
        house_system: request.house_system,
    }
}

fn score_chart(chart: &Chart, topic: ElectionTopic) -> (i32, Vec<ElectionScoreItem>, Vec<String>) {
    let ascendant_ruler = ZodiacSign::from_longitude(chart.houses.ascendant).ruler();
    let topical_cusp = chart.houses.cusps[usize::from(topic.topical_house() - 1)];
    let topical_ruler = ZodiacSign::from_longitude(topical_cusp).ruler();
    let natural = topic.natural_significator();
    let mut items = Vec::new();
    let mut advisories = Vec::new();

    add_condition(&mut items, chart, ascendant_ruler, "Ascendant ruler", 2);
    add_condition(&mut items, chart, topical_ruler, "Topical house ruler", 2);
    if natural != topical_ruler {
        add_condition(&mut items, chart, natural, "Natural significator", 1);
    }
    add_condition(&mut items, chart, Planet::Moon, "Moon", 2);

    for benefic in [Planet::Venus, Planet::Jupiter] {
        if chart
            .planet(benefic)
            .is_some_and(|value| value.house == 1 || value.house == 10)
        {
            items.push(ElectionScoreItem {
                label: format!("{} angular", benefic.name()),
                score: 5,
                rationale: "A benefic occupies the first or tenth house".to_owned(),
            });
        }
    }
    for malefic in [Planet::Mars, Planet::Saturn] {
        if chart
            .planet(malefic)
            .is_some_and(|value| value.house == 1 || value.house == 10)
        {
            items.push(ElectionScoreItem {
                label: format!("{} angular", malefic.name()),
                score: -7,
                rationale: "A malefic occupies the first or tenth house".to_owned(),
            });
            advisories.push(format!("{} is angular in a dominant place", malefic.name()));
        }
    }

    let moon = chart.planet(Planet::Moon);
    if moon.is_some_and(|value| (195.0..225.0).contains(&value.longitude)) {
        items.push(ElectionScoreItem {
            label: "Moon via combusta".to_owned(),
            score: -10,
            rationale: "The Moon is between 15° Libra and 15° Scorpio".to_owned(),
        });
        advisories.push("Moon is in the via combusta".to_owned());
    }

    let applying = chart
        .aspects
        .iter()
        .filter(|aspect| {
            matches!(aspect.phase, AspectPhase::Applying | AspectPhase::Exact)
                && (aspect.left == PointId::Planet(Planet::Moon)
                    || aspect.right == PointId::Planet(Planet::Moon))
                && matches!(aspect.left, PointId::Planet(_))
                && matches!(aspect.right, PointId::Planet(_))
        })
        .collect::<Vec<_>>();
    if applying.is_empty() {
        items.push(ElectionScoreItem {
            label: "Moon without application".to_owned(),
            score: -8,
            rationale: "No applying Ptolemaic aspect is present at this instant".to_owned(),
        });
        advisories.push("Moon has no applying major aspect in the chart".to_owned());
    } else {
        for aspect in applying {
            let other = if aspect.left == PointId::Planet(Planet::Moon) {
                aspect.right
            } else {
                aspect.left
            };
            let PointId::Planet(other) = other else {
                continue;
            };
            let value = match other {
                Planet::Venus | Planet::Jupiter => 5,
                Planet::Mars | Planet::Saturn => -5,
                Planet::Sun | Planet::Moon | Planet::Mercury => 1,
            };
            items.push(ElectionScoreItem {
                label: format!("Moon {} {}", aspect.kind.name(), other.name()),
                score: value,
                rationale: format!("Applying with {:.2}° orb", aspect.orb),
            });
        }
    }

    if ZodiacSign::from_longitude(chart.houses.ascendant).modality() == super::Modality::Fixed {
        items.push(ElectionScoreItem {
            label: "Fixed ascendant".to_owned(),
            score: 2,
            rationale: "The rising sign supports continuity and durability".to_owned(),
        });
    }

    let score = items.iter().map(|item| item.score).sum();
    (score, items, advisories)
}

fn add_condition(
    items: &mut Vec<ElectionScoreItem>,
    chart: &Chart,
    planet: Planet,
    label: &str,
    weight: i32,
) {
    let Some(condition) = chart.conditions.iter().find(|value| value.planet == planet) else {
        return;
    };
    let score = i32::from(condition.total_score) * weight;
    let motion = if condition
        .accidental
        .contains(&AccidentalDignity::Retrograde)
    {
        "retrograde"
    } else {
        "direct"
    };
    let solar = match condition.combustion {
        Combustion::Cazimi => "cazimi",
        Combustion::Combust => "combust",
        Combustion::UnderBeams => "under the beams",
        Combustion::Free => "free of the beams",
    };
    items.push(ElectionScoreItem {
        label: format!("{label}: {}", planet.name()),
        score,
        rationale: format!("condition {} · {motion} · {solar}", condition.total_score),
    });
}

#[cfg(test)]
mod tests {
    use super::{ElectionRequest, ElectionSearch, ElectionTopic};
    use crate::astro::{
        ChartCalculator, Coordinates, SwissEphemerisProvider, TraditionalHouseSystem,
    };

    #[test]
    fn election_search_ranks_complete_septenary_charts() -> Result<(), Box<dyn std::error::Error>> {
        let search = ElectionSearch::new(ChartCalculator::new(SwissEphemerisProvider::new(
            "data/ephe",
        )?));
        let result = search.search(ElectionRequest {
            title: "Test election".to_owned(),
            start_jd_ut: 2_461_043.5,
            end_jd_ut: 2_461_044.0,
            step_minutes: 120,
            location_name: "Greenwich".to_owned(),
            coordinates: Coordinates {
                latitude: 51.4779,
                longitude: 0.0,
                elevation_m: 46.0,
            },
            house_system: TraditionalHouseSystem::Regiomontanus,
            topic: ElectionTopic::General,
            limit: 3,
        })?;
        assert_eq!(result.evaluated_instants, 7);
        assert_eq!(result.candidates.len(), 3);
        assert!(
            result
                .candidates
                .windows(2)
                .all(|pair| pair[0].score >= pair[1].score)
        );
        assert!(
            result
                .candidates
                .iter()
                .all(|candidate| candidate.chart.positions.len() == 7)
        );
        Ok(())
    }
}
