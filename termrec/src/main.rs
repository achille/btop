use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cast;
mod chart;
mod extract;
mod metrics;
mod record;
mod replay;
mod vterm;

#[derive(Parser)]
#[command(name = "termrec", about = "Record, replay, and chart btop terminal sessions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record a terminal session to an asciicast v2 file
    Record {
        /// Output file path
        #[arg(short, long, default_value = "recording.cast")]
        output: PathBuf,
        /// Command to run (default: btop)
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Replay a recorded session with TUI playback controls
    Replay {
        /// Path to the .cast file
        file: PathBuf,
        /// Playback speed multiplier
        #[arg(long, default_value = "1.0")]
        speed: f64,
    },
    /// Extract btop metrics from a recording to JSON
    Extract {
        /// Path to the .cast file
        file: PathBuf,
        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate charts from a recording
    Chart {
        /// Path to the .cast file
        file: PathBuf,
        /// Output image file (PNG or SVG)
        #[arg(short, long, default_value = "chart.png")]
        output: PathBuf,
        /// Metric to chart: cpu, cpu-cores, mem, net, load, all
        #[arg(long, default_value = "all")]
        metric: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record { output, command } => {
            let cmd = if command.is_empty() {
                vec!["btop".to_string()]
            } else {
                command
            };
            eprintln!("Recording to {}... (run your session, exit to stop)", output.display());
            record::record(&cmd, &output)?;
            eprintln!("Recording saved to {}", output.display());
        }
        Commands::Replay { file, speed } => {
            replay::replay(&file, speed)?;
        }
        Commands::Extract { file, output } => {
            let frames = extract::extract_from_file(&file)?;
            let json = serde_json::to_string_pretty(&frames)?;
            if let Some(out_path) = output {
                std::fs::write(&out_path, &json)?;
                eprintln!("Extracted {} frames to {}", frames.len(), out_path.display());
            } else {
                println!("{}", json);
            }
        }
        Commands::Chart {
            file,
            output,
            metric,
        } => {
            let metric_kind = chart::MetricKind::from_str(&metric)
                .ok_or_else(|| anyhow::anyhow!("unknown metric: {}. Use: cpu, cpu-cores, mem, net, load, all", metric))?;
            let frames = extract::extract_from_file(&file)?;
            chart::generate_chart(&frames, &output, metric_kind)?;
            eprintln!("Chart saved to {}", output.display());
        }
    }

    Ok(())
}
