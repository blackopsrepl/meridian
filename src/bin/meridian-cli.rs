use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use meridian::astro::{ChartCalculator, ChartRequest, SwissEphemerisProvider};
use meridian::render::{WheelOptions, chart_csv, render_wheel};

#[derive(Debug, Parser)]
#[command(
    name = "meridian-cli",
    version,
    about = "Command-line chart calculation for Meridian"
)]
struct Cli {
    #[arg(value_name = "REQUEST.json")]
    request: PathBuf,
    #[arg(long, value_enum, default_value = "json")]
    format: OutputFormat,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long, env = "MERIDIAN_EPHE_PATH")]
    ephemeris: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Svg,
    Csv,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    calculate_file(
        cli.request,
        cli.format,
        cli.output,
        cli.ephemeris.unwrap_or_else(default_ephemeris_path),
    )
}

fn default_ephemeris_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(binary_dir) = executable.parent()
    {
        candidates.push(binary_dir.join("data/ephe"));
        if let Some(prefix) = binary_dir.parent() {
            candidates.push(prefix.join("Resources/data/ephe"));
            candidates.push(prefix.join("lib/Meridian/data/ephe"));
        }
    }
    candidates.push(PathBuf::from("data/ephe"));
    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| PathBuf::from("data/ephe"))
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
