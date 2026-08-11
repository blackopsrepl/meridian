use axum::extract::{Form, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::astro::{
    AnnualProfection, Calendar, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
    ElectionRequest, ElectionSearch, ElectionTopic, EphemerisTable, FirdariaPeriod, OrbPolicy,
    Planet, RelationshipCalculator, SkyEventSearch, TimeZoneSpec, TimingCalculator,
    TraditionalHouseSystem, resolve_moment,
};
use crate::render::{
    WheelOptions, chart_csv, ephemeris_csv, render_composite_wheel, render_synastry_wheel,
    render_wheel,
};

use super::AppState;
use super::error::WebError;
use super::forms::NewChartForm;
use super::views::{
    ElectionFormValues, RelationshipOutput, TimingOutput, chart_page, dashboard_page,
    elections_page, ephemeris_page, new_chart_page, relationships_page, timing_page,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/charts/new", get(new_chart))
        .route("/charts", post(create_chart))
        .route("/charts/{id}", get(show_chart))
        .route("/charts/{id}/wheel.svg", get(chart_wheel))
        .route("/charts/{id}/data.csv", get(chart_data_csv))
        .route("/charts/{id}/delete", post(delete_chart_form))
        .route("/tools/ephemeris", get(ephemeris_tool))
        .route("/tools/ephemeris.csv", get(ephemeris_csv_tool))
        .route("/tools/timing", get(timing_tool))
        .route("/tools/relationships", get(relationships_tool))
        .route("/tools/relationships.svg", get(relationship_svg))
        .route("/tools/elections", get(elections_tool))
        .route("/assets/app.css", get(stylesheet))
        .route("/assets/app.js", get(javascript))
        .route("/api/v1/health", get(api_health))
        .route("/api/v1/calculate", post(api_calculate))
        .route("/api/v1/ephemeris", get(api_ephemeris))
        .route("/api/v1/events", get(api_events))
        .route("/api/v1/charts/{id}/timing", get(api_chart_timing))
        .route("/api/v1/relationships", get(api_relationships))
        .route("/api/v1/elections", post(api_elections))
        .route(
            "/api/v1/charts",
            get(api_list_charts).post(api_create_chart),
        )
        .route(
            "/api/v1/charts/{id}",
            get(api_get_chart).delete(api_delete_chart),
        )
        .with_state(state)
}

async fn dashboard(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let current = state.calculator.calculate(current_sky_request())?;
    let recent = state.store.list_charts(12)?;
    Ok(Html(dashboard_page(&current, &recent).into_string()))
}

#[derive(Debug, Deserialize)]
struct PurposeQuery {
    purpose: Option<String>,
}

async fn new_chart(Query(query): Query<PurposeQuery>) -> Html<String> {
    let purpose = query.purpose.as_deref().unwrap_or("natal");
    Html(new_chart_page(purpose).into_string())
}

async fn create_chart(
    State(state): State<AppState>,
    Form(form): Form<NewChartForm>,
) -> Result<Redirect, WebError> {
    let (request, orb_policy) = form.into_calculation()?;
    let chart = state
        .calculator
        .calculate_with_orb_policy(request, orb_policy)?;
    let record = state.store.insert_chart(&chart)?;
    Ok(Redirect::to(&format!("/charts/{}", record.id)))
}

async fn show_chart(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> Result<Html<String>, WebError> {
    let record = state
        .store
        .get_chart(parse_id(&raw_id)?)?
        .ok_or(WebError::NotFound)?;
    Ok(Html(chart_page(&record).into_string()))
}

async fn chart_wheel(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> Result<Response, WebError> {
    let record = state
        .store
        .get_chart(parse_id(&raw_id)?)?
        .ok_or(WebError::NotFound)?;
    let svg = render_wheel(&record.chart, WheelOptions::default());
    Ok((
        [
            (CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=meridian-chart.svg",
            ),
        ],
        svg,
    )
        .into_response())
}

async fn chart_data_csv(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> Result<Response, WebError> {
    let record = state
        .store
        .get_chart(parse_id(&raw_id)?)?
        .ok_or(WebError::NotFound)?;
    let bytes = chart_csv(&record.chart)
        .map_err(|error| WebError::BadRequest(format!("CSV export failed: {error}")))?;
    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=meridian-chart.csv",
            ),
        ],
        bytes,
    )
        .into_response())
}

async fn delete_chart_form(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> Result<Redirect, WebError> {
    if !state.store.delete_chart(parse_id(&raw_id)?)? {
        return Err(WebError::NotFound);
    }
    Ok(Redirect::to("/"))
}

#[derive(Debug, Deserialize)]
struct EphemerisQuery {
    start: Option<String>,
    days: Option<usize>,
    step: Option<f64>,
}

async fn ephemeris_tool(
    State(state): State<AppState>,
    Query(query): Query<EphemerisQuery>,
) -> Result<Html<String>, WebError> {
    let start_date = query.start.unwrap_or_else(today_string);
    let days = query.days.unwrap_or(31).clamp(1, 366);
    let step = query.step.unwrap_or(1.0);
    validate_ephemeris_interval(days, step)?;
    let start_jd = parse_utc_date(&start_date)?;
    let table = EphemerisTable::calculate(state.calculator.provider(), start_jd, days, step)
        .map_err(|error| WebError::BadRequest(error.to_string()))?;
    let events = SkyEventSearch::new(state.calculator.provider().clone())
        .search(start_jd, start_jd + step * days as f64)
        .map_err(|error| WebError::BadRequest(error.to_string()))?;
    Ok(Html(
        ephemeris_page(&table, &events, &start_date, days, step).into_string(),
    ))
}

async fn ephemeris_csv_tool(
    State(state): State<AppState>,
    Query(query): Query<EphemerisQuery>,
) -> Result<Response, WebError> {
    let start_date = query.start.unwrap_or_else(today_string);
    let days = query.days.unwrap_or(31).clamp(1, 366);
    let step = query.step.unwrap_or(1.0);
    validate_ephemeris_interval(days, step)?;
    let table = EphemerisTable::calculate(
        state.calculator.provider(),
        parse_utc_date(&start_date)?,
        days,
        step,
    )
    .map_err(|error| WebError::BadRequest(error.to_string()))?;
    let bytes = ephemeris_csv(&table)
        .map_err(|error| WebError::BadRequest(format!("CSV export failed: {error}")))?;
    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=meridian-ephemeris.csv",
            ),
        ],
        bytes,
    )
        .into_response())
}

fn validate_ephemeris_interval(days: usize, step: f64) -> Result<(), WebError> {
    if !step.is_finite() || !(1.0 / 24.0..=31.0).contains(&step) || step * days as f64 > 3660.0 {
        Err(WebError::BadRequest(
            "ephemeris interval is outside the supported range".to_owned(),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct TimingQuery {
    chart: Option<String>,
    technique: Option<String>,
    target: Option<String>,
    end: Option<String>,
    age: Option<f64>,
    harmonic: Option<u16>,
    location: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    houses: Option<String>,
}

async fn timing_tool(
    State(state): State<AppState>,
    Query(query): Query<TimingQuery>,
) -> Result<Html<String>, WebError> {
    let target = query.target.unwrap_or_else(today_string);
    let end = query.end.unwrap_or_else(|| {
        (Utc::now().date_naive() + Duration::days(90))
            .format("%Y-%m-%d")
            .to_string()
    });
    let technique = query.technique.unwrap_or_else(|| "transits".to_owned());
    let age = query.age.unwrap_or(30.0);
    let harmonic = query.harmonic.unwrap_or(9);
    validate_age(age)?;
    if !(1..=360).contains(&harmonic) {
        return Err(WebError::BadRequest(
            "harmonic must be between 1 and 360".to_owned(),
        ));
    }

    let charts = state.store.list_charts(500)?;
    let output = if let Some(raw_id) = query.chart.as_deref() {
        let record = state
            .store
            .get_chart(parse_id(raw_id)?)?
            .ok_or(WebError::NotFound)?;
        Some(calculate_timing_output(
            &state,
            &record.chart,
            &technique,
            &target,
            &end,
            age,
            harmonic,
            query.location.as_deref(),
            query.latitude,
            query.longitude,
            query.elevation,
            query.houses.as_deref(),
        )?)
    } else {
        None
    };

    Ok(Html(
        timing_page(
            &charts,
            query.chart.as_deref(),
            &technique,
            &target,
            &end,
            age,
            harmonic,
            query.location.as_deref(),
            query.latitude,
            query.longitude,
            query.elevation,
            parse_optional_house_system(query.houses.as_deref())?,
            output.as_ref(),
        )
        .into_string(),
    ))
}

#[allow(
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn calculate_timing_output(
    state: &AppState,
    natal: &crate::astro::Chart,
    technique: &str,
    target: &str,
    end: &str,
    age: f64,
    harmonic: u16,
    location: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    house_system: Option<&str>,
) -> Result<TimingOutput, WebError> {
    let calculator = TimingCalculator::from_chart_calculator(&state.calculator);
    let target_jd = parse_utc_date(target)?;
    let end_jd = parse_utc_date(end)?;
    let result = match technique {
        "transits" => TimingOutput::Transits(calculator.transits(natal, target_jd, end_jd)?),
        "secondary" => {
            TimingOutput::Technique(calculator.secondary_progressions(natal, target_jd)?)
        }
        "solar_arc" => TimingOutput::Technique(calculator.solar_arc(natal, target_jd)?),
        "harmonic" => TimingOutput::Technique(calculator.harmonic(natal, harmonic)?),
        "profection" => TimingOutput::Profection(AnnualProfection::at_age(natal, age as u32)),
        "firdaria" => TimingOutput::Firdaria(FirdariaPeriod::at_age(natal.sect, age)),
        "solar_return" | "lunar_return" => {
            let (location_name, coordinates, return_houses) = timing_location(
                natal,
                location,
                latitude,
                longitude,
                elevation,
                house_system,
            )?;
            let (planet, maximum_days, label) = if technique == "solar_return" {
                (Planet::Sun, 370.0, "Solar return")
            } else {
                (Planet::Moon, 35.0, "Lunar return")
            };
            let chart = calculator.return_chart(
                &state.calculator,
                natal,
                planet,
                target_jd,
                target_jd + maximum_days,
                format!("{} · {label}", natal.request.title),
                location_name,
                coordinates,
                return_houses,
            )?;
            TimingOutput::Return(Box::new(chart))
        }
        "planetary_hours" => {
            let (_, coordinates, _) = timing_location(
                natal,
                location,
                latitude,
                longitude,
                elevation,
                house_system,
            )?;
            TimingOutput::PlanetaryHours(calculator.planetary_hours(target_jd, coordinates)?)
        }
        _ => {
            return Err(WebError::BadRequest(format!(
                "unknown timing technique: {technique}"
            )));
        }
    };
    Ok(result)
}

fn timing_location(
    natal: &crate::astro::Chart,
    location: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    house_system: Option<&str>,
) -> Result<(String, Coordinates, TraditionalHouseSystem), WebError> {
    let mut coordinates = natal.request.coordinates;
    match (latitude, longitude) {
        (Some(latitude), Some(longitude)) => {
            coordinates.latitude = latitude;
            coordinates.longitude = longitude;
        }
        (None, None) => {}
        _ => {
            return Err(WebError::BadRequest(
                "return latitude and longitude must be supplied together".to_owned(),
            ));
        }
    }
    if let Some(elevation) = elevation {
        coordinates.elevation_m = elevation;
    }
    coordinates
        .validate()
        .map_err(|error| WebError::BadRequest(error.to_owned()))?;
    let location_name = location
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&natal.request.location_name)
        .to_owned();
    let house_system =
        parse_optional_house_system(house_system)?.unwrap_or(natal.request.house_system);
    Ok((location_name, coordinates, house_system))
}

#[derive(Debug, Deserialize)]
struct RelationshipQuery {
    first: Option<String>,
    second: Option<String>,
    method: Option<String>,
}

async fn relationships_tool(
    State(state): State<AppState>,
    Query(query): Query<RelationshipQuery>,
) -> Result<Html<String>, WebError> {
    let charts = state.store.list_charts(500)?;
    let method = query.method.unwrap_or_else(|| "synastry".to_owned());
    let output = match (query.first.as_deref(), query.second.as_deref()) {
        (None, None) => None,
        (Some(first_id), Some(second_id)) => Some(calculate_relationship(
            &state, first_id, second_id, &method,
        )?),
        _ => {
            return Err(WebError::BadRequest(
                "both relationship charts must be selected".to_owned(),
            ));
        }
    };
    Ok(Html(
        relationships_page(
            &charts,
            query.first.as_deref(),
            query.second.as_deref(),
            &method,
            output.as_ref(),
        )
        .into_string(),
    ))
}

async fn relationship_svg(
    State(state): State<AppState>,
    Query(query): Query<RelationshipQuery>,
) -> Result<Response, WebError> {
    let first = query
        .first
        .as_deref()
        .ok_or_else(|| WebError::BadRequest("first chart is required".to_owned()))?;
    let second = query
        .second
        .as_deref()
        .ok_or_else(|| WebError::BadRequest("second chart is required".to_owned()))?;
    let output = calculate_relationship(
        &state,
        first,
        second,
        query.method.as_deref().unwrap_or("synastry"),
    )?;
    let svg = match output {
        RelationshipOutput::Synastry {
            report,
            first,
            second,
        } => render_synastry_wheel(&first, &second, &report, 960),
        RelationshipOutput::Composite(chart) => render_composite_wheel(&chart, 960),
        RelationshipOutput::Davison(chart) => render_wheel(
            &chart,
            WheelOptions {
                size: 960,
                ..WheelOptions::default()
            },
        ),
    };
    Ok((
        [
            (CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=meridian-relationship.svg",
            ),
        ],
        svg,
    )
        .into_response())
}

fn calculate_relationship(
    state: &AppState,
    first_id: &str,
    second_id: &str,
    method: &str,
) -> Result<RelationshipOutput, WebError> {
    let first_id = parse_id(first_id)?;
    let second_id = parse_id(second_id)?;
    if first_id == second_id {
        return Err(WebError::BadRequest(
            "relationship charts must be different".to_owned(),
        ));
    }
    let first = state
        .store
        .get_chart(first_id)?
        .ok_or(WebError::NotFound)?
        .chart;
    let second = state
        .store
        .get_chart(second_id)?
        .ok_or(WebError::NotFound)?
        .chart;
    let calculator = RelationshipCalculator::default();
    match method {
        "synastry" => Ok(RelationshipOutput::Synastry {
            report: calculator.synastry(&first, &second),
            first: Box::new(first),
            second: Box::new(second),
        }),
        "composite" => Ok(RelationshipOutput::Composite(calculator.composite(
            &first,
            &second,
            format!(
                "{} × {} · composite",
                first.request.title, second.request.title
            ),
        ))),
        "davison" => Ok(RelationshipOutput::Davison(Box::new(calculator.davison(
            &state.calculator,
            &first,
            &second,
            format!(
                "{} × {} · Davison",
                first.request.title, second.request.title
            ),
            first.request.house_system,
        )?))),
        _ => Err(WebError::BadRequest(format!(
            "unknown relationship method: {method}"
        ))),
    }
}

#[derive(Debug, Deserialize)]
struct ElectionQuery {
    title: Option<String>,
    start: Option<String>,
    end: Option<String>,
    step: Option<u16>,
    location: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    houses: Option<String>,
    topic: Option<String>,
    limit: Option<usize>,
}

async fn elections_tool(
    State(state): State<AppState>,
    Query(query): Query<ElectionQuery>,
) -> Result<Html<String>, WebError> {
    let now = Utc::now();
    let default_start = now.format("%Y-%m-%dT%H:00").to_string();
    let default_end = (now + Duration::days(7))
        .format("%Y-%m-%dT%H:00")
        .to_string();
    let should_search = query.start.is_some() || query.end.is_some();
    let values = ElectionFormValues {
        title: query
            .title
            .unwrap_or_else(|| "Chosen undertaking".to_owned()),
        start: query.start.unwrap_or(default_start),
        end: query.end.unwrap_or(default_end),
        step_minutes: query.step.unwrap_or(60),
        location: query.location.unwrap_or_else(|| "Greenwich".to_owned()),
        latitude: query.latitude.unwrap_or(51.4779),
        longitude: query.longitude.unwrap_or(0.0),
        elevation: query.elevation.unwrap_or(46.0),
        house_system: parse_house_system(query.houses.as_deref().unwrap_or("regiomontanus"))?,
        topic: parse_election_topic(query.topic.as_deref().unwrap_or("general"))?,
        limit: query.limit.unwrap_or(10),
    };
    let result = if should_search {
        let request = ElectionRequest {
            title: values.title.clone(),
            start_jd_ut: parse_utc_datetime(&values.start)?,
            end_jd_ut: parse_utc_datetime(&values.end)?,
            step_minutes: values.step_minutes,
            location_name: values.location.clone(),
            coordinates: Coordinates {
                latitude: values.latitude,
                longitude: values.longitude,
                elevation_m: values.elevation,
            },
            house_system: values.house_system,
            topic: values.topic,
            limit: values.limit,
        };
        Some(ElectionSearch::new(state.calculator.as_ref().clone()).search(request)?)
    } else {
        None
    };
    Ok(Html(elections_page(&values, result.as_ref()).into_string()))
}

async fn stylesheet() -> Response {
    static_asset(
        "text/css; charset=utf-8",
        include_str!("../../static/app.css"),
    )
}

async fn javascript() -> Response {
    static_asset(
        "text/javascript; charset=utf-8",
        include_str!("../../static/app.js"),
    )
}

fn static_asset(content_type: &'static str, content: &'static str) -> Response {
    let mut response = content.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    response
}

#[derive(Debug, Serialize)]
struct Health<'a> {
    status: &'a str,
    application: &'a str,
    version: &'a str,
    ephemeris: &'a str,
    planet_set: &'a str,
}

async fn api_health() -> Json<Health<'static>> {
    Json(Health {
        status: "ok",
        application: "meridian",
        version: env!("CARGO_PKG_VERSION"),
        ephemeris: "swisseph-rs/0.1.9 + DE441 .se1",
        planet_set: "classical_septenary",
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiCalculationRequest {
    Configured {
        chart: ChartRequest,
        orb_policy: OrbPolicy,
    },
    Simple(ChartRequest),
}

fn calculate_api_request(
    state: &AppState,
    request: ApiCalculationRequest,
) -> Result<crate::astro::Chart, crate::astro::ChartError> {
    match request {
        ApiCalculationRequest::Configured { chart, orb_policy } => state
            .calculator
            .calculate_with_orb_policy(chart, orb_policy),
        ApiCalculationRequest::Simple(chart) => state.calculator.calculate(chart),
    }
}

async fn api_calculate(
    State(state): State<AppState>,
    Json(request): Json<ApiCalculationRequest>,
) -> Response {
    match calculate_api_request(&state, request) {
        Ok(chart) => Json(chart).into_response(),
        Err(error) => WebError::Calculation(error).api_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ApiEphemerisQuery {
    start_jd: f64,
    rows: Option<usize>,
    step: Option<f64>,
}

async fn api_ephemeris(
    State(state): State<AppState>,
    Query(query): Query<ApiEphemerisQuery>,
) -> Response {
    match EphemerisTable::calculate(
        state.calculator.provider(),
        query.start_jd,
        query.rows.unwrap_or(31),
        query.step.unwrap_or(1.0),
    ) {
        Ok(table) => Json(table).into_response(),
        Err(error) => WebError::BadRequest(error.to_string()).api_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ApiEventQuery {
    start_jd: f64,
    end_jd: f64,
}

async fn api_events(State(state): State<AppState>, Query(query): Query<ApiEventQuery>) -> Response {
    match SkyEventSearch::new(state.calculator.provider().clone())
        .search(query.start_jd, query.end_jd)
    {
        Ok(events) => Json(events).into_response(),
        Err(error) => WebError::BadRequest(error.to_string()).api_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ApiTimingQuery {
    technique: String,
    target: String,
    end: Option<String>,
    age: Option<f64>,
    harmonic: Option<u16>,
    location: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    houses: Option<String>,
}

async fn api_chart_timing(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
    Query(query): Query<ApiTimingQuery>,
) -> Response {
    let result = async {
        let record = state
            .store
            .get_chart(parse_id(&raw_id)?)?
            .ok_or(WebError::NotFound)?;
        let age = query.age.unwrap_or(30.0);
        let harmonic = query.harmonic.unwrap_or(9);
        validate_age(age)?;
        let end = query.end.unwrap_or_else(|| query.target.clone());
        calculate_timing_output(
            &state,
            &record.chart,
            &query.technique,
            &query.target,
            &end,
            age,
            harmonic,
            query.location.as_deref(),
            query.latitude,
            query.longitude,
            query.elevation,
            query.houses.as_deref(),
        )
    }
    .await;

    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error.api_response(),
    }
}

async fn api_relationships(
    State(state): State<AppState>,
    Query(query): Query<RelationshipQuery>,
) -> Response {
    let result = query
        .first
        .as_deref()
        .zip(query.second.as_deref())
        .ok_or_else(|| {
            WebError::BadRequest("first and second chart identifiers are required".to_owned())
        })
        .and_then(|(first, second)| {
            calculate_relationship(
                &state,
                first,
                second,
                query.method.as_deref().unwrap_or("synastry"),
            )
        });

    match result {
        Ok(RelationshipOutput::Synastry { report, .. }) => Json(serde_json::json!({
            "method": "synastry",
            "result": report
        }))
        .into_response(),
        Ok(RelationshipOutput::Composite(chart)) => Json(serde_json::json!({
            "method": "composite",
            "result": chart
        }))
        .into_response(),
        Ok(RelationshipOutput::Davison(chart)) => Json(serde_json::json!({
            "method": "davison",
            "result": chart
        }))
        .into_response(),
        Err(error) => error.api_response(),
    }
}

async fn api_elections(
    State(state): State<AppState>,
    Json(request): Json<ElectionRequest>,
) -> Response {
    match ElectionSearch::new(state.calculator.as_ref().clone()).search(request) {
        Ok(result) => Json(result).into_response(),
        Err(error) => WebError::Election(error).api_response(),
    }
}

async fn api_create_chart(
    State(state): State<AppState>,
    Json(request): Json<ApiCalculationRequest>,
) -> Response {
    let result = calculate_api_request(&state, request)
        .map_err(WebError::Calculation)
        .and_then(|chart| state.store.insert_chart(&chart).map_err(WebError::Store));
    match result {
        Ok(record) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(error) => error.api_response(),
    }
}

async fn api_list_charts(State(state): State<AppState>) -> Response {
    match state.store.list_charts(100) {
        Ok(charts) => Json(charts).into_response(),
        Err(error) => WebError::Store(error).api_response(),
    }
}

async fn api_get_chart(State(state): State<AppState>, Path(raw_id): Path<String>) -> Response {
    let result = parse_id(&raw_id)
        .and_then(|id| state.store.get_chart(id).map_err(WebError::Store))
        .and_then(|record| record.ok_or(WebError::NotFound));
    match result {
        Ok(record) => Json(record).into_response(),
        Err(error) => error.api_response(),
    }
}

async fn api_delete_chart(State(state): State<AppState>, Path(raw_id): Path<String>) -> Response {
    let result = parse_id(&raw_id)
        .and_then(|id| state.store.delete_chart(id).map_err(WebError::Store))
        .and_then(|deleted| deleted.then_some(()).ok_or(WebError::NotFound));
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error.api_response(),
    }
}

fn parse_id(raw: &str) -> Result<Uuid, WebError> {
    Uuid::parse_str(raw).map_err(|_| WebError::BadRequest("invalid chart identifier".to_owned()))
}

fn validate_age(age: f64) -> Result<(), WebError> {
    if age.is_finite() && (0.0..=140.0).contains(&age) {
        Ok(())
    } else {
        Err(WebError::BadRequest(
            "age must be a finite number between 0 and 140".to_owned(),
        ))
    }
}

fn today_string() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn parse_utc_date(value: &str) -> Result<f64, WebError> {
    let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| WebError::BadRequest("date must use YYYY-MM-DD".to_owned()))?;
    let moment = resolve_moment(
        &CivilDateTime {
            year: date.year(),
            month: date.month() as u8,
            day: date.day() as u8,
            hour: 0,
            minute: 0,
            second: 0.0,
            calendar: Calendar::Gregorian,
        },
        &TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC".to_owned()),
        },
    )
    .map_err(|error| WebError::BadRequest(error.to_string()))?;
    Ok(moment.jd_ut)
}

fn parse_utc_datetime(value: &str) -> Result<f64, WebError> {
    let date_time = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| WebError::BadRequest("date-time must use YYYY-MM-DDTHH:MM".to_owned()))?;
    let moment = resolve_moment(
        &CivilDateTime {
            year: date_time.year(),
            month: date_time.month() as u8,
            day: date_time.day() as u8,
            hour: date_time.hour() as u8,
            minute: date_time.minute() as u8,
            second: 0.0,
            calendar: Calendar::Gregorian,
        },
        &TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC".to_owned()),
        },
    )
    .map_err(|error| WebError::BadRequest(error.to_string()))?;
    Ok(moment.jd_ut)
}

fn parse_house_system(value: &str) -> Result<TraditionalHouseSystem, WebError> {
    match value {
        "whole_sign" => Ok(TraditionalHouseSystem::WholeSign),
        "equal" => Ok(TraditionalHouseSystem::Equal),
        "porphyry" => Ok(TraditionalHouseSystem::Porphyry),
        "alcabitius" => Ok(TraditionalHouseSystem::Alcabitius),
        "placidus" => Ok(TraditionalHouseSystem::Placidus),
        "regiomontanus" => Ok(TraditionalHouseSystem::Regiomontanus),
        "campanus" => Ok(TraditionalHouseSystem::Campanus),
        "morinus" => Ok(TraditionalHouseSystem::Morinus),
        _ => Err(WebError::BadRequest(format!(
            "unknown traditional house system: {value}"
        ))),
    }
}

fn parse_optional_house_system(
    value: Option<&str>,
) -> Result<Option<TraditionalHouseSystem>, WebError> {
    value
        .filter(|value| !value.is_empty())
        .map(parse_house_system)
        .transpose()
}

fn parse_election_topic(value: &str) -> Result<ElectionTopic, WebError> {
    match value {
        "general" => Ok(ElectionTopic::General),
        "marriage" => Ok(ElectionTopic::Marriage),
        "commerce" => Ok(ElectionTopic::Commerce),
        "travel" => Ok(ElectionTopic::Travel),
        "career" => Ok(ElectionTopic::Career),
        "home" => Ok(ElectionTopic::Home),
        "healing" => Ok(ElectionTopic::Healing),
        "litigation" => Ok(ElectionTopic::Litigation),
        _ => Err(WebError::BadRequest(format!(
            "unknown election topic: {value}"
        ))),
    }
}

fn current_sky_request() -> ChartRequest {
    let now = Utc::now();
    ChartRequest {
        title: "Sky now".to_owned(),
        purpose: ChartPurpose::Event,
        local_time: CivilDateTime {
            year: now.year(),
            month: now.month() as u8,
            day: now.day() as u8,
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: f64::from(now.second()) + f64::from(now.nanosecond()) / 1_000_000_000.0,
            calendar: Calendar::Gregorian,
        },
        time_zone: TimeZoneSpec::FixedOffset {
            minutes_east: 0,
            label: Some("UTC".to_owned()),
        },
        location_name: "Greenwich".to_owned(),
        coordinates: Coordinates {
            latitude: 51.4779,
            longitude: 0.0,
            elevation_m: 46.0,
        },
        house_system: TraditionalHouseSystem::WholeSign,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::http::header::{CONTENT_TYPE, HeaderValue};
    use tower::ServiceExt;

    use super::router;
    use crate::astro::{
        Calendar, ChartCalculator, ChartPurpose, ChartRequest, CivilDateTime, Coordinates,
        ElectionRequest, ElectionTopic, SwissEphemerisProvider, TimeZoneSpec,
        TraditionalHouseSystem,
    };
    use crate::store::{ChartRecord, Store};
    use crate::web::AppState;

    fn test_app() -> Result<axum::Router, Box<dyn std::error::Error>> {
        Ok(router(AppState::new(
            ChartCalculator::new(SwissEphemerisProvider::new("data/ephe")?),
            Store::open(":memory:")?,
        )))
    }

    #[tokio::test]
    async fn health_reports_the_strict_planet_set() -> Result<(), Box<dyn std::error::Error>> {
        let response = test_app()?
            .oneshot(Request::get("/api/v1/health").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn dashboard_renders_live_ephemeris() -> Result<(), Box<dyn std::error::Error>> {
        let response = test_app()?
            .oneshot(Request::get("/").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/html; charset=utf-8"))
        );
        Ok(())
    }

    #[tokio::test]
    async fn every_browser_workspace_has_a_live_route() -> Result<(), Box<dyn std::error::Error>> {
        let app = test_app()?;
        for path in [
            "/tools/ephemeris?start=2026-08-11&days=2&step=1",
            "/tools/timing",
            "/tools/relationships",
            "/tools/elections",
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn saved_charts_feed_timing_and_relationship_apis()
    -> Result<(), Box<dyn std::error::Error>> {
        let app = test_app()?;
        let first = create_api_chart(&app, chart_request("First", 1990, 6, 15)).await?;
        let second = create_api_chart(&app, chart_request("Second", 1992, 9, 20)).await?;

        let relationship = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/v1/relationships?first={}&second={}&method=synastry",
                    first.id, second.id
                ))
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(relationship.status(), StatusCode::OK);

        let timing = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/v1/charts/{}/timing?technique=secondary&target=2026-08-11",
                    first.id
                ))
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(timing.status(), StatusCode::OK);

        let election_request = ElectionRequest {
            title: "API election".to_owned(),
            start_jd_ut: 2_461_043.5,
            end_jd_ut: 2_461_043.75,
            step_minutes: 120,
            location_name: "Greenwich".to_owned(),
            coordinates: Coordinates {
                latitude: 51.4779,
                longitude: 0.0,
                elevation_m: 46.0,
            },
            house_system: TraditionalHouseSystem::Regiomontanus,
            topic: ElectionTopic::General,
            limit: 2,
        };
        let election = app
            .oneshot(
                Request::post("/api/v1/elections")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&election_request)?))?,
            )
            .await?;
        assert_eq!(election.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn configured_api_calculation_retains_orb_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::json!({
            "chart": chart_request("Configured", 2000, 1, 1),
            "orb_policy": {
                "conjunction": 4.0,
                "sextile": 3.0,
                "square": 4.0,
                "trine": 4.0,
                "opposition": 5.0,
                "luminary_bonus": 1.0,
                "angle_orb": 2.5,
                "lot_orb": 1.5
            }
        });
        let response = test_app()?
            .oneshot(
                Request::post("/api/v1/calculate")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload)?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
        let chart: crate::astro::Chart = serde_json::from_slice(&body)?;
        assert!((chart.orb_policy.conjunction - 4.0).abs() < f64::EPSILON);
        assert!((chart.orb_policy.lot_orb - 1.5).abs() < f64::EPSILON);
        Ok(())
    }

    async fn create_api_chart(
        app: &axum::Router,
        request: ChartRequest,
    ) -> Result<ChartRecord, Box<dyn std::error::Error>> {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/charts")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&request)?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    fn chart_request(title: &str, year: i32, month: u8, day: u8) -> ChartRequest {
        ChartRequest {
            title: title.to_owned(),
            purpose: ChartPurpose::Natal,
            local_time: CivilDateTime {
                year,
                month,
                day,
                hour: 12,
                minute: 0,
                second: 0.0,
                calendar: Calendar::Gregorian,
            },
            time_zone: TimeZoneSpec::FixedOffset {
                minutes_east: 0,
                label: Some("UTC".to_owned()),
            },
            location_name: "Greenwich".to_owned(),
            coordinates: Coordinates {
                latitude: 51.4779,
                longitude: 0.0,
                elevation_m: 46.0,
            },
            house_system: TraditionalHouseSystem::WholeSign,
        }
    }
}
