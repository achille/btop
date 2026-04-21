use anyhow::{Context, Result};
use plotters::prelude::*;
use std::path::Path;

use crate::metrics::BtopFrame;

/// Which metric to chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Cpu,
    CpuCores,
    Mem,
    Net,
    Load,
    All,
}

impl MetricKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "cpu-cores" | "cpucores" | "cores" => Some(Self::CpuCores),
            "mem" | "memory" => Some(Self::Mem),
            "net" | "network" => Some(Self::Net),
            "load" => Some(Self::Load),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

/// Generate a chart from extracted btop frames.
pub fn generate_chart(
    frames: &[BtopFrame],
    output: &Path,
    metric: MetricKind,
) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("no frames to chart");
    }

    let is_svg = output
        .extension()
        .map_or(false, |e| e.eq_ignore_ascii_case("svg"));

    if metric == MetricKind::All {
        generate_all_chart(frames, output, is_svg)
    } else if is_svg {
        let root = SVGBackend::new(output, (1200, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        draw_single_metric(&root, frames, metric)?;
        root.present()?;
        Ok(())
    } else {
        let root = BitMapBackend::new(output, (1200, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        draw_single_metric(&root, frames, metric)?;
        root.present()?;
        Ok(())
    }
}

fn generate_all_chart(frames: &[BtopFrame], output: &Path, is_svg: bool) -> Result<()> {
    if is_svg {
        let root = SVGBackend::new(output, (1600, 1200)).into_drawing_area();
        root.fill(&WHITE)?;
        let areas = root.split_evenly((2, 2));
        draw_single_metric(&areas[0], frames, MetricKind::Cpu)?;
        draw_single_metric(&areas[1], frames, MetricKind::Mem)?;
        draw_single_metric(&areas[2], frames, MetricKind::Net)?;
        draw_single_metric(&areas[3], frames, MetricKind::Load)?;
        root.present()?;
    } else {
        let root = BitMapBackend::new(output, (1600, 1200)).into_drawing_area();
        root.fill(&WHITE)?;
        let areas = root.split_evenly((2, 2));
        draw_single_metric(&areas[0], frames, MetricKind::Cpu)?;
        draw_single_metric(&areas[1], frames, MetricKind::Mem)?;
        draw_single_metric(&areas[2], frames, MetricKind::Net)?;
        draw_single_metric(&areas[3], frames, MetricKind::Load)?;
        root.present()?;
    }
    Ok(())
}

fn draw_single_metric<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
    metric: MetricKind,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    match metric {
        MetricKind::Cpu => draw_cpu(area, frames),
        MetricKind::CpuCores => draw_cpu_cores(area, frames),
        MetricKind::Mem => draw_mem(area, frames),
        MetricKind::Net => draw_net(area, frames),
        MetricKind::Load => draw_load(area, frames),
        MetricKind::All => unreachable!("All handled separately"),
    }
}

fn time_range(frames: &[BtopFrame]) -> (f64, f64) {
    let start = frames.first().map(|f| f.timestamp).unwrap_or(0.0);
    let end = frames.last().map(|f| f.timestamp).unwrap_or(1.0);
    (start, if end <= start { start + 1.0 } else { end })
}

fn draw_cpu<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let (t_start, t_end) = time_range(frames);
    let mut chart = ChartBuilder::on(area)
        .caption("CPU Usage (%)", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(t_start..t_end, 0.0..100.0)
        .context("build cpu chart")?;

    chart.configure_mesh().x_desc("Time (s)").y_desc("%").draw()?;

    let data: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| f.cpu_total.map(|c| (f.timestamp, c)))
        .collect();

    chart
        .draw_series(LineSeries::new(data, &BLUE))?
        .label("CPU Total")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.configure_series_labels().draw()?;
    Ok(())
}

fn draw_cpu_cores<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let (t_start, t_end) = time_range(frames);
    let max_cores = frames.iter().map(|f| f.cpu_cores.len()).max().unwrap_or(0);

    let mut chart = ChartBuilder::on(area)
        .caption("CPU Cores (%)", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(t_start..t_end, 0.0..100.0)
        .context("build cpu cores chart")?;

    chart.configure_mesh().x_desc("Time (s)").y_desc("%").draw()?;

    let colors = [RED, BLUE, GREEN, MAGENTA, CYAN, BLACK];

    for core_idx in 0..max_cores {
        let data: Vec<(f64, f64)> = frames
            .iter()
            .filter_map(|f| f.cpu_cores.get(core_idx).map(|&v| (f.timestamp, v)))
            .collect();
        let color = &colors[core_idx % colors.len()];
        chart
            .draw_series(LineSeries::new(data, color))?
            .label(format!("Core {}", core_idx))
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 20, y)], colors[core_idx % colors.len()])
            });
    }

    chart.configure_series_labels().draw()?;
    Ok(())
}

fn draw_mem<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let (t_start, t_end) = time_range(frames);
    let mut chart = ChartBuilder::on(area)
        .caption("Memory Usage (%)", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(t_start..t_end, 0.0..100.0)
        .context("build mem chart")?;

    chart.configure_mesh().x_desc("Time (s)").y_desc("%").draw()?;

    let used: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| {
            f.mem
                .as_ref()
                .and_then(|m| m.used_pct.map(|v| (f.timestamp, v)))
        })
        .collect();

    let cached: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| {
            f.mem
                .as_ref()
                .and_then(|m| m.cached_pct.map(|v| (f.timestamp, v)))
        })
        .collect();

    if !used.is_empty() {
        chart
            .draw_series(LineSeries::new(used, &RED))?
            .label("Used")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    }
    if !cached.is_empty() {
        chart
            .draw_series(LineSeries::new(cached, &BLUE))?
            .label("Cached")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    }

    chart.configure_series_labels().draw()?;
    Ok(())
}

fn draw_net<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    // Parse net speed strings to numeric values (bytes/s).
    let parse_speed = |s: &str| -> f64 {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return 0.0;
        }
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        if parts.len() > 1 {
            let unit = parts[1].to_lowercase();
            if unit.starts_with("gib") || unit.starts_with("gb") {
                num * 1024.0 * 1024.0 * 1024.0
            } else if unit.starts_with("mib") || unit.starts_with("mb") {
                num * 1024.0 * 1024.0
            } else if unit.starts_with("kib") || unit.starts_with("kb") {
                num * 1024.0
            } else {
                num
            }
        } else {
            num
        }
    };

    let down_data: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| {
            f.net
                .as_ref()
                .and_then(|n| n.download.as_ref().map(|d| (f.timestamp, parse_speed(d))))
        })
        .collect();
    let up_data: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| {
            f.net
                .as_ref()
                .and_then(|n| n.upload.as_ref().map(|u| (f.timestamp, parse_speed(u))))
        })
        .collect();

    let max_val = down_data
        .iter()
        .chain(up_data.iter())
        .map(|(_, v)| *v)
        .fold(1.0f64, f64::max);

    let (t_start, t_end) = time_range(frames);

    let mut chart = ChartBuilder::on(area)
        .caption("Network (bytes/s)", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(50)
        .build_cartesian_2d(t_start..t_end, 0.0..max_val * 1.1)
        .context("build net chart")?;

    chart.configure_mesh().x_desc("Time (s)").y_desc("B/s").draw()?;

    if !down_data.is_empty() {
        chart
            .draw_series(LineSeries::new(down_data, &BLUE))?
            .label("Download")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    }
    if !up_data.is_empty() {
        chart
            .draw_series(LineSeries::new(up_data, &RED))?
            .label("Upload")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    }

    chart.configure_series_labels().draw()?;
    Ok(())
}

fn draw_load<DB: DrawingBackend>(
    area: &DrawingArea<DB, plotters::coord::Shift>,
    frames: &[BtopFrame],
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let data_1m: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| f.load_avg.map(|(a, _, _)| (f.timestamp, a)))
        .collect();
    let data_5m: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| f.load_avg.map(|(_, b, _)| (f.timestamp, b)))
        .collect();
    let data_15m: Vec<(f64, f64)> = frames
        .iter()
        .filter_map(|f| f.load_avg.map(|(_, _, c)| (f.timestamp, c)))
        .collect();

    let max_load = data_1m
        .iter()
        .chain(data_5m.iter())
        .chain(data_15m.iter())
        .map(|(_, v)| *v)
        .fold(1.0f64, f64::max);

    let (t_start, t_end) = time_range(frames);

    let mut chart = ChartBuilder::on(area)
        .caption("Load Average", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(t_start..t_end, 0.0..max_load * 1.2)
        .context("build load chart")?;

    chart.configure_mesh().x_desc("Time (s)").draw()?;

    if !data_1m.is_empty() {
        chart
            .draw_series(LineSeries::new(data_1m, &RED))?
            .label("1 min")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    }
    if !data_5m.is_empty() {
        chart
            .draw_series(LineSeries::new(data_5m, &BLUE))?
            .label("5 min")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    }
    if !data_15m.is_empty() {
        chart
            .draw_series(LineSeries::new(data_15m, &GREEN))?
            .label("15 min")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    }

    chart.configure_series_labels().draw()?;
    Ok(())
}
