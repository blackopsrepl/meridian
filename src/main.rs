use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use meridian::astro::{ChartCalculator, ChartRequest, SwissEphemerisProvider};
use meridian::locations::CityIndex;
use meridian::render::{WheelOptions, chart_csv, render_wheel};
use meridian::store::Store;
use meridian::web::{AppState, app};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "meridian",
    version,
    about = "Classical septenary astrology workbench"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the local web application and JSON API.
    Serve {
        #[arg(long, env = "MERIDIAN_BIND", default_value = "127.0.0.1:3001")]
        bind: SocketAddr,
        #[arg(
            long,
            env = "MERIDIAN_DATABASE",
            default_value = "data/meridian.sqlite3"
        )]
        database: PathBuf,
        #[arg(long, env = "MERIDIAN_EPHE_PATH", default_value = "data/ephe")]
        ephemeris: PathBuf,
        #[arg(long, env = "MERIDIAN_CITY_PATH", default_value = "data/geonames")]
        cities: PathBuf,
    },
    /// Calculate one chart request from a JSON file.
    Chart {
        #[arg(value_name = "REQUEST.json")]
        request: PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, env = "MERIDIAN_EPHE_PATH", default_value = "data/ephe")]
        ephemeris: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Svg,
    Csv,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("meridian=info")),
        )
        .compact()
        .init();
    match Cli::parse().command {
        Command::Serve {
            bind,
            database,
            ephemeris,
            cities,
        } => serve(bind, database, ephemeris, cities).await,
        Command::Chart {
            request,
            format,
            output,
            ephemeris,
        } => calculate_file(request, format, output, ephemeris),
    }
}

async fn serve(
    bind: SocketAddr,
    database: PathBuf,
    ephemeris: PathBuf,
    cities: PathBuf,
) -> Result<()> {
    let provider = SwissEphemerisProvider::new(&ephemeris)
        .with_context(|| format!("could not open ephemeris at {}", ephemeris.display()))?;
    let store = Store::open(&database)
        .with_context(|| format!("could not open database at {}", database.display()))?;
    let city_index = CityIndex::load(&cities).with_context(|| {
        format!(
            "could not open the city atlas at {}; run `make data-cities`",
            cities.display()
        )
    })?;
    let city_count = city_index.len();
    let state = AppState::new(ChartCalculator::new(provider), store, city_index);
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("could not bind {bind}"))?;
    info!(address = %bind, city_count, "Meridian observatory ready");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server stopped unexpectedly")
}

fn calculate_file(
    request_path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
    ephemeris: PathBuf,
) -> Result<()> {
    let input = std::fs::read_to_string(&request_path)
        .with_context(|| format!("could not read {}", request_path.display()))?;
    let request: ChartRequest = serde_json::from_str(&input)
        .with_context(|| format!("invalid request JSON in {}", request_path.display()))?;
    let chart =
        ChartCalculator::new(SwissEphemerisProvider::new(&ephemeris)?).calculate(request)?;
    let bytes = match format {
        OutputFormat::Json => serde_json::to_vec_pretty(&chart)?,
        OutputFormat::Svg => render_wheel(&chart, WheelOptions::default()).into_bytes(),
        OutputFormat::Csv => chart_csv(&chart)?,
    };
    if let Some(path) = output {
        std::fs::write(&path, bytes)
            .with_context(|| format!("could not write {}", path.display()))?;
    } else {
        use std::io::Write as _;
        std::io::stdout().write_all(&bytes)?;
        std::io::stdout().write_all(b"\n")?;
    }
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl-C handler");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to install termination handler"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
}
