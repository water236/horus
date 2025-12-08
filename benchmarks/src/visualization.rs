//! Benchmark Visualization Module
//!
//! Provides automatic graph generation and analysis for HORUS benchmarks.
//! Uses the `plotters` crate to generate PNG charts without external dependencies.

use plotters::prelude::*;
use std::path::Path;

/// Benchmark data point for visualization
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
    pub error: Option<f64>,
}

/// Comparison data for two methods
#[derive(Debug, Clone)]
pub struct ComparisonData {
    pub name: String,
    pub method_a: DataPoint,
    pub method_b: DataPoint,
}

/// Latency distribution data
#[derive(Debug, Clone)]
pub struct LatencyData {
    pub name: String,
    pub latencies_ns: Vec<f64>,
}

/// Generate a bar chart comparing two methods (e.g., POD vs Standard)
pub fn draw_comparison_bar_chart(
    output_path: &str,
    title: &str,
    comparisons: &[ComparisonData],
    method_a_name: &str,
    method_b_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (800, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_value = comparisons
        .iter()
        .map(|c| c.method_a.value.max(c.method_b.value))
        .fold(0.0f64, |a, b| a.max(b))
        * 1.2;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(80)
        .build_cartesian_2d(0..comparisons.len(), 0f64..max_value)?;

    chart
        .configure_mesh()
        .x_labels(comparisons.len())
        .x_label_formatter(&|x| {
            comparisons
                .get(*x)
                .map(|c| c.name.clone())
                .unwrap_or_default()
        })
        .y_desc("Latency (ns)")
        .x_desc("Operation")
        .axis_desc_style(("sans-serif", 16))
        .draw()?;

    // Draw method A bars (blue)
    chart
        .draw_series(comparisons.iter().enumerate().map(|(i, c)| {
            let x0 = i as f64 + 0.1;
            let x1 = i as f64 + 0.4;
            Rectangle::new([(i, 0.0), (i, c.method_a.value)], BLUE.mix(0.8).filled())
        }))?
        .label(method_a_name)
        .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 20, y + 5)], BLUE.mix(0.8).filled()));

    // Draw method B bars (red)
    chart
        .draw_series(comparisons.iter().enumerate().map(|(i, c)| {
            Rectangle::new([(i, 0.0), (i, c.method_b.value)], RED.mix(0.8).filled())
        }))?
        .label(method_b_name)
        .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 20, y + 5)], RED.mix(0.8).filled()));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;
    println!("Chart saved to: {}", output_path);
    Ok(())
}

/// Generate a grouped bar chart for side-by-side comparison
pub fn draw_grouped_bar_chart(
    output_path: &str,
    title: &str,
    labels: &[&str],
    method_a_values: &[f64],
    method_b_values: &[f64],
    method_a_name: &str,
    method_b_name: &str,
    y_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (800, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_value = method_a_values
        .iter()
        .chain(method_b_values.iter())
        .fold(0.0f64, |a, b| a.max(*b))
        * 1.2;

    let n = labels.len();

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(80)
        .build_cartesian_2d(0f64..(n as f64 * 3.0), 0f64..max_value)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(n)
        .x_label_formatter(&|x| {
            let idx = (*x as usize) / 3;
            labels.get(idx).map(|s| s.to_string()).unwrap_or_default()
        })
        .y_desc(y_label)
        .axis_desc_style(("sans-serif", 16))
        .draw()?;

    // Draw method A bars (blue)
    chart
        .draw_series(method_a_values.iter().enumerate().map(|(i, &v)| {
            let x = i as f64 * 3.0 + 0.2;
            Rectangle::new([(x, 0.0), (x + 1.0, v)], BLUE.mix(0.8).filled())
        }))?
        .label(method_a_name)
        .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 20, y + 5)], BLUE.mix(0.8).filled()));

    // Draw method B bars (red)
    chart
        .draw_series(method_b_values.iter().enumerate().map(|(i, &v)| {
            let x = i as f64 * 3.0 + 1.4;
            Rectangle::new([(x, 0.0), (x + 1.0, v)], RED.mix(0.8).filled())
        }))?
        .label(method_b_name)
        .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 20, y + 5)], RED.mix(0.8).filled()));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;
    println!("Chart saved to: {}", output_path);
    Ok(())
}

/// Generate a latency histogram
pub fn draw_latency_histogram(
    output_path: &str,
    title: &str,
    latencies_ns: &[f64],
    bins: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (800, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let min_val = latencies_ns.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = latencies_ns
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let range = max_val - min_val;
    let bin_width = range / bins as f64;

    // Calculate histogram
    let mut histogram = vec![0u32; bins];
    for &v in latencies_ns {
        let bin = ((v - min_val) / bin_width).floor() as usize;
        let bin = bin.min(bins - 1);
        histogram[bin] += 1;
    }

    let max_count = *histogram.iter().max().unwrap_or(&1);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(60)
        .build_cartesian_2d(min_val..max_val, 0u32..(max_count as f64 * 1.1) as u32)?;

    chart
        .configure_mesh()
        .y_desc("Frequency")
        .x_desc("Latency (ns)")
        .axis_desc_style(("sans-serif", 16))
        .draw()?;

    chart.draw_series(histogram.iter().enumerate().map(|(i, &count)| {
        let x0 = min_val + i as f64 * bin_width;
        let x1 = x0 + bin_width;
        Rectangle::new([(x0, 0), (x1, count)], BLUE.mix(0.8).filled())
    }))?;

    // Draw percentile lines
    let mut sorted = latencies_ns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = sorted[sorted.len() / 2];
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
    let p99 = sorted[(sorted.len() as f64 * 0.99) as usize];

    // P50 line (green)
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(p50, 0), (p50, max_count)],
        GREEN.stroke_width(2),
    )))?;

    // P95 line (orange)
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(p95, 0), (p95, max_count)],
        RGBColor(255, 165, 0).stroke_width(2),
    )))?;

    // P99 line (red)
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(p99, 0), (p99, max_count)],
        RED.stroke_width(2),
    )))?;

    root.present()?;
    println!("Histogram saved to: {}", output_path);
    Ok(())
}

/// Generate a speedup bar chart
pub fn draw_speedup_chart(
    output_path: &str,
    title: &str,
    labels: &[&str],
    speedups: &[f64],
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (800, 400)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_speedup = speedups.iter().fold(0.0f64, |a, &b| a.max(b)) * 1.2;
    let n = labels.len();

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..(n as f64 * 2.0), 0f64..max_speedup)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(n)
        .x_label_formatter(&|x| {
            let idx = (*x as usize) / 2;
            labels.get(idx).map(|s| s.to_string()).unwrap_or_default()
        })
        .y_desc("Speedup (x)")
        .axis_desc_style(("sans-serif", 16))
        .draw()?;

    // Draw reference line at 1.0x
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0.0, 1.0), (n as f64 * 2.0, 1.0)],
        BLACK.stroke_width(1),
    )))?;

    // Draw bars with color based on speedup
    chart.draw_series(speedups.iter().enumerate().map(|(i, &v)| {
        let x = i as f64 * 2.0 + 0.3;
        let color = if v >= 5.0 {
            GREEN.mix(0.8)
        } else if v >= 2.0 {
            BLUE.mix(0.8)
        } else if v >= 1.0 {
            RGBColor(255, 165, 0).mix(0.8) // Orange
        } else {
            RED.mix(0.8)
        };
        Rectangle::new([(x, 0.0), (x + 1.4, v)], color.filled())
    }))?;

    // Add value labels on bars
    for (i, &v) in speedups.iter().enumerate() {
        let x = i as f64 * 2.0 + 1.0;
        chart.draw_series(std::iter::once(Text::new(
            format!("{:.1}x", v),
            (x, v + max_speedup * 0.03),
            ("sans-serif", 14).into_font(),
        )))?;
    }

    root.present()?;
    println!("Speedup chart saved to: {}", output_path);
    Ok(())
}

/// Generate a line chart for latency over time/iterations
pub fn draw_latency_timeline(
    output_path: &str,
    title: &str,
    latencies: &[f64],
    window_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (1000, 400)).into_drawing_area();
    root.fill(&WHITE)?;

    // Calculate moving average
    let moving_avg: Vec<f64> = latencies
        .windows(window_size)
        .map(|w| w.iter().sum::<f64>() / w.len() as f64)
        .collect();

    let max_val = latencies.iter().fold(0.0f64, |a, &b| a.max(b)) * 1.1;
    let n = latencies.len();

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24).into_font())
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(80)
        .build_cartesian_2d(0..n, 0f64..max_val)?;

    chart
        .configure_mesh()
        .y_desc("Latency (ns)")
        .x_desc("Iteration")
        .axis_desc_style(("sans-serif", 14))
        .draw()?;

    // Draw raw data points (light)
    chart.draw_series(
        latencies
            .iter()
            .enumerate()
            .map(|(i, &v)| Circle::new((i, v), 1, BLUE.mix(0.2).filled())),
    )?;

    // Draw moving average (darker line)
    chart
        .draw_series(LineSeries::new(
            moving_avg
                .iter()
                .enumerate()
                .map(|(i, &v)| (i + window_size / 2, v)),
            BLUE.stroke_width(2),
        ))?
        .label("Moving Average")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;
    println!("Timeline chart saved to: {}", output_path);
    Ok(())
}

/// Statistical analysis summary
#[derive(Debug, Clone)]
pub struct AnalysisSummary {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
    pub cv: f64, // Coefficient of variation
}

impl AnalysisSummary {
    pub fn from_latencies(latencies: &[f64]) -> Self {
        let mut sorted = latencies.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len();
        let mean = sorted.iter().sum::<f64>() / n as f64;
        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };

        let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();

        Self {
            mean,
            median,
            std_dev,
            p50: sorted[n / 2],
            p95: sorted[(n as f64 * 0.95) as usize],
            p99: sorted[(n as f64 * 0.99) as usize],
            min: sorted[0],
            max: sorted[n - 1],
            cv: std_dev / mean * 100.0,
        }
    }

    pub fn print_report(&self, name: &str) {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  Statistical Analysis: {:^36} ║", name);
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║  Mean:         {:>12.1} ns                              ║",
            self.mean
        );
        println!(
            "║  Median:       {:>12.1} ns                              ║",
            self.median
        );
        println!(
            "║  Std Dev:      {:>12.1} ns                              ║",
            self.std_dev
        );
        println!(
            "║  CV:           {:>12.1} %                               ║",
            self.cv
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║  Min:          {:>12.1} ns                              ║",
            self.min
        );
        println!(
            "║  P50:          {:>12.1} ns                              ║",
            self.p50
        );
        println!(
            "║  P95:          {:>12.1} ns                              ║",
            self.p95
        );
        println!(
            "║  P99:          {:>12.1} ns                              ║",
            self.p99
        );
        println!(
            "║  Max:          {:>12.1} ns                              ║",
            self.max
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }
}

/// Generate a complete benchmark report with all charts
pub fn generate_full_report(
    output_dir: &str,
    report_name: &str,
    pod_stats: &BenchmarkStats,
    std_stats: &BenchmarkStats,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let prefix = format!("{}/{}_{}", output_dir, report_name, timestamp);

    // 1. Comparison bar chart
    draw_grouped_bar_chart(
        &format!("{}_comparison.png", prefix),
        "POD vs Standard Link Performance",
        &["Send", "Recv", "Roundtrip"],
        &[pod_stats.send_ns, pod_stats.recv_ns, pod_stats.roundtrip_ns],
        &[std_stats.send_ns, std_stats.recv_ns, std_stats.roundtrip_ns],
        "POD Link",
        "Standard Link",
        "Latency (ns)",
    )?;

    // 2. Speedup chart
    let speedups = [
        std_stats.send_ns / pod_stats.send_ns,
        std_stats.recv_ns / pod_stats.recv_ns,
        std_stats.roundtrip_ns / pod_stats.roundtrip_ns,
    ];
    draw_speedup_chart(
        &format!("{}_speedup.png", prefix),
        "POD Link Speedup vs Standard Link",
        &["Send", "Recv", "Roundtrip"],
        &speedups,
    )?;

    // 3. Latency histograms (if raw data available)
    if !pod_stats.raw_latencies.is_empty() {
        draw_latency_histogram(
            &format!("{}_pod_histogram.png", prefix),
            "POD Link Latency Distribution",
            &pod_stats.raw_latencies,
            50,
        )?;
    }

    if !std_stats.raw_latencies.is_empty() {
        draw_latency_histogram(
            &format!("{}_std_histogram.png", prefix),
            "Standard Link Latency Distribution",
            &std_stats.raw_latencies,
            50,
        )?;
    }

    // 4. Timeline charts (if raw data available)
    if pod_stats.raw_latencies.len() > 100 {
        draw_latency_timeline(
            &format!("{}_pod_timeline.png", prefix),
            "POD Link Latency Over Time",
            &pod_stats.raw_latencies,
            100,
        )?;
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Report generated: {}", prefix);
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
}

/// Benchmark statistics for visualization
#[derive(Debug, Clone, Default)]
pub struct BenchmarkStats {
    pub send_ns: f64,
    pub recv_ns: f64,
    pub roundtrip_ns: f64,
    pub raw_latencies: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_summary() {
        let data: Vec<f64> = (0..1000).map(|i| 100.0 + (i as f64 * 0.1)).collect();
        let summary = AnalysisSummary::from_latencies(&data);
        assert!(summary.mean > 0.0);
        assert!(summary.std_dev > 0.0);
    }
}
