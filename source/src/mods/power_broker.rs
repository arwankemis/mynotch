use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DailyPowerBucket {
    pub date: String,
    pub energy_kwh: f32,
    pub cost_idr: u32,
    pub samples_count: u32,
    pub observed_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MonthlyPowerHistory {
    pub year: i32,
    pub month: u32,
    pub name: String,
    pub total_kwh: f32,
    pub total_cost_idr: u32,
    pub provenance: String, // "measured", "estimated", "legacy_estimated_monthly"
    pub days: u32,
    #[serde(default)]
    pub daily_buckets: Vec<DailyPowerBucket>,
}

#[derive(Clone, Debug)]
pub struct EnergyIntegrator {
    pub last_power_w: Option<f32>,
    pub last_sample_time_ms: Option<u64>,
    pub today_kwh: f32,
    pub max_gap_seconds: f64,
}

impl Default for EnergyIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyIntegrator {
    pub fn new() -> Self {
        Self {
            last_power_w: None,
            last_sample_time_ms: None,
            today_kwh: 0.0,
            max_gap_seconds: 10.0,
        }
    }

    /// Trapezoidal energy integration:
    /// dE[kWh] = ((P1 + P2) / 2) * (dt / 3600000)
    /// Pauses during gaps longer than `max_gap_seconds` (sleep/suspend).
    pub fn add_sample(&mut self, power_w: f32, monotonic_secs: f64) -> f32 {
        let mut delta_kwh = 0.0;
        if let (Some(p0), Some(t0_ms)) = (self.last_power_w, self.last_sample_time_ms) {
            let dt = monotonic_secs - (t0_ms as f64 / 1000.0);
            if dt > 0.0 && dt <= self.max_gap_seconds && power_w.is_finite() && p0.is_finite() {
                let avg_w = (p0 + power_w) / 2.0;
                delta_kwh = (avg_w / 1000.0) * (dt as f32 / 3600.0);
                self.today_kwh += delta_kwh;
            }
        }
        self.last_power_w = Some(power_w);
        self.last_sample_time_ms = Some((monotonic_secs * 1000.0) as u64);
        delta_kwh
    }

    pub fn today_kwh(&self) -> f32 {
        self.today_kwh
    }

    #[allow(dead_code)]
    pub fn reset_day(&mut self) {
        self.today_kwh = 0.0;
    }
}

/// Directory: `data/mod-data/daya/history/<year>/<month>.json`
pub fn get_history_dir(root: &Path) -> PathBuf {
    root.join("mod-data").join("daya").join("history")
}

pub fn load_monthly_history(root: &Path, year: i32) -> Vec<MonthlyPowerHistory> {
    let year_dir = get_history_dir(root).join(year.to_string());
    let mut list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&year_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(hist) = serde_json::from_slice::<MonthlyPowerHistory>(&bytes) {
                        list.push(hist);
                    }
                }
            }
        }
    }
    list.sort_by(|a, b| b.month.cmp(&a.month));
    list
}

pub fn save_monthly_history(root: &Path, history: &MonthlyPowerHistory) -> Result<(), String> {
    let year_dir = get_history_dir(root).join(history.year.to_string());
    std::fs::create_dir_all(&year_dir).map_err(|e| e.to_string())?;
    let file_path = year_dir.join(format!("{:02}.json", history.month));
    let json = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    std::fs::write(file_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Idempotent migration from legacy State fields into MonthlyPowerHistory
pub fn migrate_legacy_power_data(root: &Path, state: &mut crate::core::State) -> bool {
    if state.monthly_kwh <= 0.0 && state.power_month.is_empty() {
        return false;
    }

    let (year, month) = if let Some((y_str, m_str)) = state.power_month.split_once('-') {
        let y = y_str.parse::<i32>().unwrap_or(2026);
        let m = m_str.parse::<u32>().unwrap_or(10);
        (y, m)
    } else {
        (2026, 10)
    };

    let year_dir = get_history_dir(root).join(year.to_string());
    let file_path = year_dir.join(format!("{:02}.json", month));

    if !file_path.exists() && state.monthly_kwh > 0.0 {
        let history = MonthlyPowerHistory {
            year,
            month,
            name: format!("Bulan {:02}", month),
            total_kwh: state.monthly_kwh,
            total_cost_idr: state.monthly_cost_idr,
            provenance: "legacy_estimated_monthly".to_string(),
            days: state.power_days_active.len().max(1) as u32,
            daily_buckets: Vec::new(),
        };
        let _ = save_monthly_history(root, &history);
    }

    state.monthly_kwh = 0.0;
    state.monthly_cost_idr = 0;
    state.power_month.clear();
    state.power_days_active.clear();

    true
}
