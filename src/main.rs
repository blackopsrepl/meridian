use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, anyhow};
use axum::Router;
use axum::body::{Body, to_bytes};
use meridian::astro::{ChartCalculator, SwissEphemerisProvider};
use meridian::locations::CityIndex;
use meridian::store::Store;
use meridian::web::{AppState, app as desktop_router};
use tauri::Manager;
use tauri::path::BaseDirectory;
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;

const MAX_DESKTOP_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug)]
struct DesktopPaths {
    database: PathBuf,
    ephemeris: PathBuf,
    cities: PathBuf,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("meridian=info")),
        )
        .compact()
        .init();

    let router = Arc::new(OnceLock::<Router>::new());
    let protocol_router = Arc::clone(&router);
    let setup_router = Arc::clone(&router);

    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(
            "meridian",
            move |_context, request, responder| {
                let Some(router) = protocol_router.get().cloned() else {
                    responder.respond(
                        tauri::http::Response::builder()
                            .status(503)
                            .header("content-type", "text/plain; charset=utf-8")
                            .body(b"Meridian is still starting".to_vec())
                            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new())),
                    );
                    return;
                };
                tauri::async_runtime::spawn(async move {
                    let request = request.map(Body::from);
                    let response = match router.oneshot(request).await {
                        Ok(response) => response,
                        Err(error) => match error {},
                    };
                    let (parts, body) = response.into_parts();
                    let response = match to_bytes(body, MAX_DESKTOP_RESPONSE_BYTES).await {
                        Ok(bytes) => tauri::http::Response::from_parts(parts, bytes.to_vec()),
                        Err(error) => tauri::http::Response::builder()
                            .status(500)
                            .header("content-type", "text/plain; charset=utf-8")
                            .body(
                                format!("could not render desktop response: {error}").into_bytes(),
                            )
                            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new())),
                    };
                    responder.respond(response);
                });
            },
        )
        .setup(move |application| {
            let paths = DesktopPaths::resolve(application.handle())?;
            let router = build_router(&paths)?;
            setup_router
                .set(router)
                .map_err(|_| anyhow!("desktop router was initialized more than once"))?;

            let window = application
                .get_webview_window("main")
                .context("the main Meridian window is not configured")?;
            window.navigate("meridian://localhost/".parse()?)?;
            window.show()?;
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}

impl DesktopPaths {
    fn resolve(application: &tauri::AppHandle) -> Result<Self> {
        let database = match std::env::var_os("MERIDIAN_DATABASE") {
            Some(path) => PathBuf::from(path),
            None => application
                .path()
                .app_data_dir()
                .context("could not resolve the Meridian application-data directory")?
                .join("meridian.sqlite3"),
        };
        let ephemeris = resource_or_override(application, "MERIDIAN_EPHE_PATH", "data/ephe")?;
        let cities = resource_or_override(application, "MERIDIAN_CITY_PATH", "data/geonames")?;
        Ok(Self {
            database,
            ephemeris,
            cities,
        })
    }
}

fn resource_or_override(
    application: &tauri::AppHandle,
    variable: &str,
    resource: &str,
) -> Result<PathBuf> {
    std::env::var_os(variable).map_or_else(
        || {
            application
                .path()
                .resolve(resource, BaseDirectory::Resource)
                .with_context(|| format!("could not resolve bundled resource {resource}"))
        },
        |path: OsString| Ok(PathBuf::from(path)),
    )
}

fn build_router(paths: &DesktopPaths) -> Result<Router> {
    let provider = SwissEphemerisProvider::new(&paths.ephemeris).with_context(|| {
        format!(
            "could not open the bundled ephemeris at {}",
            paths.ephemeris.display()
        )
    })?;
    let store = Store::open(&paths.database).with_context(|| {
        format!(
            "could not open the chart archive at {}",
            paths.database.display()
        )
    })?;
    let city_index = CityIndex::load(&paths.cities).with_context(|| {
        format!(
            "could not open the bundled city atlas at {}",
            paths.cities.display()
        )
    })?;
    Ok(desktop_router(AppState::new(
        ChartCalculator::new(provider),
        store,
        city_index,
    )))
}
