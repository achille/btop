use serde::{Deserialize, Serialize};

/// A single extracted btop frame's metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtopFrame {
    /// Timestamp in seconds from recording start.
    pub timestamp: f64,
    /// Overall CPU usage percentage (0-100).
    pub cpu_total: Option<f64>,
    /// Per-core CPU usage percentages.
    pub cpu_cores: Vec<f64>,
    /// Load averages: (1m, 5m, 15m).
    pub load_avg: Option<(f64, f64, f64)>,
    /// Memory metrics.
    pub mem: Option<MemMetrics>,
    /// Network metrics.
    pub net: Option<NetMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemMetrics {
    /// Total memory string (e.g. "15.6 GiB").
    pub total: String,
    /// Used percentage.
    pub used_pct: Option<f64>,
    /// Available percentage.
    pub available_pct: Option<f64>,
    /// Cached percentage.
    pub cached_pct: Option<f64>,
    /// Free percentage.
    pub free_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetMetrics {
    /// Download speed string (e.g. "1.2 MiB/s").
    pub download: Option<String>,
    /// Upload speed string (e.g. "256 KiB/s").
    pub upload: Option<String>,
}
