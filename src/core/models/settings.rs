use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[schema(example = json!({
    "grid": {
        "enabled": true,
        "matrix": { "columns": 3, "rows": 2 },
        "display": { "screen_width": 1920, "screen_height": 1080, "offset_x": 0, "offset_y": 0, "margin": 8, "monitor_index": 0 },
        "behavior": { "auto_recycle_slots": true, "enforce_cdp_bounds": true, "scale_factor": 1.0 }
    },
    "browser": {
        "default_type": "chromium",
        "executable_path": null,
        "headless": false,
        "default_user_agent": null
    },
    "runner": {
        "max_concurrent_jobs": 5,
        "timeout_ms": 30000,
        "auto_clean_history_days": 30
    }
}))]
pub struct AppSettings {
    pub grid: GridSettings,
    pub browser: BrowserSettings,
    pub runner: RunnerSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            grid: GridSettings::default(),
            browser: BrowserSettings::default(),
            runner: RunnerSettings::default(),
        }
    }
}

// === NAMESPACE 1: GRID SYSTEM ===
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GridSettings {
    pub enabled: bool,
    pub matrix: GridMatrix,
    pub display: DisplaySettings,
    pub behavior: GridBehavior,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            matrix: GridMatrix::default(),
            display: DisplaySettings::default(),
            behavior: GridBehavior::default(),
        }
    }
}

impl GridSettings {
    /// Computes (left, top, width, height) for a given slot index
    pub fn calculate_slot_bounds(&self, slot_index: u32) -> (i32, i32, u32, u32) {
        let cols = self.matrix.columns.max(1);
        let rows = self.matrix.rows.max(1);
        let total_slots = cols * rows;
        let effective_index = if self.behavior.auto_recycle_slots {
            slot_index % total_slots
        } else {
            slot_index.min(total_slots - 1)
        };

        let col = effective_index % cols;
        let row = effective_index / cols;

        let margin = self.display.margin;
        let total_margin_x = margin * (cols + 1);
        let total_margin_y = margin * (rows + 1);

        let available_w = self.display.screen_width.saturating_sub(total_margin_x);
        let available_h = self.display.screen_height.saturating_sub(total_margin_y);

        let slot_w = (available_w / cols).max(200);
        let slot_h = (available_h / rows).max(150);

        let pos_x = self.display.offset_x + (margin + col * (slot_w + margin)) as i32;
        let pos_y = self.display.offset_y + (margin + row * (slot_h + margin)) as i32;

        (pos_x, pos_y, slot_w, slot_h)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GridMatrix {
    pub columns: u32,
    pub rows: u32,
}

impl Default for GridMatrix {
    fn default() -> Self {
        Self {
            columns: 3,
            rows: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct DisplaySettings {
    #[serde(alias = "screenWidth")]
    pub screen_width: u32,
    #[serde(alias = "screenHeight")]
    pub screen_height: u32,
    #[serde(alias = "offsetX")]
    pub offset_x: i32,
    #[serde(alias = "offsetY")]
    pub offset_y: i32,
    pub margin: u32,
    #[serde(alias = "monitorIndex")]
    pub monitor_index: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            screen_width: 1920,
            screen_height: 1080,
            offset_x: 0,
            offset_y: 0,
            margin: 8,
            monitor_index: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GridBehavior {
    #[serde(alias = "autoRecycleSlots")]
    pub auto_recycle_slots: bool,
    #[serde(alias = "enforceCdpBounds")]
    pub enforce_cdp_bounds: bool,
    #[serde(alias = "scaleFactor")]
    pub scale_factor: f64,
}

impl Default for GridBehavior {
    fn default() -> Self {
        Self {
            auto_recycle_slots: true,
            enforce_cdp_bounds: true,
            scale_factor: 1.0,
        }
    }
}

// === NAMESPACE 2: BROWSER ENGINE ===
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct BrowserSettings {
    #[serde(alias = "defaultType")]
    pub default_type: String,
    #[serde(alias = "defaultProfileId", alias = "defaultBrowser", alias = "default_browser")]
    pub default_profile_id: Option<String>,
    #[serde(alias = "executablePath")]
    pub executable_path: Option<String>,
    pub headless: bool,
    #[serde(alias = "defaultUserAgent")]
    pub default_user_agent: Option<String>,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            default_type: "chromium".to_string(),
            default_profile_id: None,
            executable_path: None,
            headless: false,
            default_user_agent: None,
        }
    }
}

// === NAMESPACE 3: WORKFLOW RUNNER ===
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct RunnerSettings {
    #[serde(alias = "maxConcurrentJobs")]
    pub max_concurrent_jobs: u32,
    #[serde(alias = "timeoutMs")]
    pub timeout_ms: u64,
    #[serde(alias = "autoCleanHistoryDays")]
    pub auto_clean_history_days: u32,
}

impl Default for RunnerSettings {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 5,
            timeout_ms: 30000,
            auto_clean_history_days: 30,
        }
    }
}

// === PARTIAL UPDATE REQUEST DTO (For PATCH) ===
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAppSettingsRequest {
    pub grid: Option<UpdateGridSettingsRequest>,
    pub browser: Option<UpdateBrowserSettingsRequest>,
    pub runner: Option<UpdateRunnerSettingsRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateGridSettingsRequest {
    pub enabled: Option<bool>,
    pub matrix: Option<UpdateGridMatrixRequest>,
    pub display: Option<UpdateDisplaySettingsRequest>,
    pub behavior: Option<UpdateGridBehaviorRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateGridMatrixRequest {
    pub columns: Option<u32>,
    pub rows: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDisplaySettingsRequest {
    #[serde(alias = "screenWidth")]
    pub screen_width: Option<u32>,
    #[serde(alias = "screenHeight")]
    pub screen_height: Option<u32>,
    #[serde(alias = "offsetX")]
    pub offset_x: Option<i32>,
    #[serde(alias = "offsetY")]
    pub offset_y: Option<i32>,
    pub margin: Option<u32>,
    #[serde(alias = "monitorIndex")]
    pub monitor_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateGridBehaviorRequest {
    #[serde(alias = "autoRecycleSlots")]
    pub auto_recycle_slots: Option<bool>,
    #[serde(alias = "enforceCdpBounds")]
    pub enforce_cdp_bounds: Option<bool>,
    #[serde(alias = "scaleFactor")]
    pub scale_factor: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBrowserSettingsRequest {
    #[serde(alias = "defaultType")]
    pub default_type: Option<String>,
    #[serde(alias = "defaultProfileId", alias = "defaultBrowser", alias = "default_browser")]
    pub default_profile_id: Option<String>,
    #[serde(alias = "executablePath")]
    pub executable_path: Option<String>,
    pub headless: Option<bool>,
    #[serde(alias = "defaultUserAgent")]
    pub default_user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRunnerSettingsRequest {
    #[serde(alias = "maxConcurrentJobs")]
    pub max_concurrent_jobs: Option<u32>,
    #[serde(alias = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    #[serde(alias = "autoCleanHistoryDays")]
    pub auto_clean_history_days: Option<u32>,
}

impl AppSettings {
    pub fn apply_patch(&mut self, patch: UpdateAppSettingsRequest) {
        if let Some(grid_patch) = patch.grid {
            if let Some(val) = grid_patch.enabled {
                self.grid.enabled = val;
            }
            if let Some(m) = grid_patch.matrix {
                if let Some(cols) = m.columns {
                    self.grid.matrix.columns = cols;
                }
                if let Some(rows) = m.rows {
                    self.grid.matrix.rows = rows;
                }
            }
            if let Some(d) = grid_patch.display {
                if let Some(w) = d.screen_width {
                    self.grid.display.screen_width = w;
                }
                if let Some(h) = d.screen_height {
                    self.grid.display.screen_height = h;
                }
                if let Some(ox) = d.offset_x {
                    self.grid.display.offset_x = ox;
                }
                if let Some(oy) = d.offset_y {
                    self.grid.display.offset_y = oy;
                }
                if let Some(m) = d.margin {
                    self.grid.display.margin = m;
                }
                if let Some(idx) = d.monitor_index {
                    self.grid.display.monitor_index = idx;
                }
            }
            if let Some(b) = grid_patch.behavior {
                if let Some(val) = b.auto_recycle_slots {
                    self.grid.behavior.auto_recycle_slots = val;
                }
                if let Some(val) = b.enforce_cdp_bounds {
                    self.grid.behavior.enforce_cdp_bounds = val;
                }
                if let Some(val) = b.scale_factor {
                    self.grid.behavior.scale_factor = val;
                }
            }
        }

        if let Some(b_patch) = patch.browser {
            if let Some(dt) = b_patch.default_type {
                self.browser.default_type = dt;
            }
            if b_patch.default_profile_id.is_some() {
                self.browser.default_profile_id = b_patch.default_profile_id;
            }
            if b_patch.executable_path.is_some() {
                self.browser.executable_path = b_patch.executable_path;
            }
            if let Some(hl) = b_patch.headless {
                self.browser.headless = hl;
            }
            if b_patch.default_user_agent.is_some() {
                self.browser.default_user_agent = b_patch.default_user_agent;
            }
        }

        if let Some(r_patch) = patch.runner {
            if let Some(mcj) = r_patch.max_concurrent_jobs {
                self.runner.max_concurrent_jobs = mcj;
            }
            if let Some(tms) = r_patch.timeout_ms {
                self.runner.timeout_ms = tms;
            }
            if let Some(chd) = r_patch.auto_clean_history_days {
                self.runner.auto_clean_history_days = chd;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_slot_bounds() {
        let grid = GridSettings {
            enabled: true,
            matrix: GridMatrix { columns: 3, rows: 2 },
            display: DisplaySettings {
                screen_width: 1920,
                screen_height: 1080,
                offset_x: 0,
                offset_y: 0,
                margin: 8,
                monitor_index: 0,
            },
            behavior: GridBehavior {
                auto_recycle_slots: true,
                enforce_cdp_bounds: true,
                scale_factor: 1.0,
            },
        };

        // Slot 0 (col 0, row 0)
        let (x0, y0, w0, h0) = grid.calculate_slot_bounds(0);
        assert_eq!(x0, 8);
        assert_eq!(y0, 8);
        assert_eq!(w0, 629);
        assert_eq!(h0, 528);

        // Slot 1 (col 1, row 0)
        let (x1, y1, w1, h1) = grid.calculate_slot_bounds(1);
        assert_eq!(x1, 8 + 629 + 8);
        assert_eq!(y1, 8);
        assert_eq!(w1, 629);
        assert_eq!(h1, 528);

        // Slot 3 (col 0, row 1)
        let (x3, y3, w3, h3) = grid.calculate_slot_bounds(3);
        assert_eq!(x3, 8);
        assert_eq!(y3, 8 + 528 + 8);
        assert_eq!(w3, 629);
        assert_eq!(h3, 528);

        // Slot 6 (recycles back to slot 0)
        let (x6, y6, _, _) = grid.calculate_slot_bounds(6);
        assert_eq!(x6, x0);
        assert_eq!(y6, y0);
    }

    #[test]
    fn test_calculate_slot_bounds_zero_dimensions_no_panic() {
        let grid = GridSettings {
            enabled: true,
            matrix: GridMatrix { columns: 0, rows: 0 },
            display: DisplaySettings {
                screen_width: 0,
                screen_height: 0,
                offset_x: -10,
                offset_y: -20,
                margin: 50,
                monitor_index: 0,
            },
            behavior: GridBehavior {
                auto_recycle_slots: true,
                enforce_cdp_bounds: true,
                scale_factor: 1.0,
            },
        };

        // Must not panic on zero dimensions
        let (x, y, w, h) = grid.calculate_slot_bounds(999);
        assert!(w >= 200);
        assert!(h >= 150);
        assert_eq!(x, -10 + 50);
        assert_eq!(y, -20 + 50);
    }

    #[test]
    fn test_calculate_slot_bounds_no_recycle_caps_at_max() {
        let grid = GridSettings {
            enabled: true,
            matrix: GridMatrix { columns: 2, rows: 2 },
            display: DisplaySettings::default(),
            behavior: GridBehavior {
                auto_recycle_slots: false,
                enforce_cdp_bounds: true,
                scale_factor: 1.0,
            },
        };

        let (x_last, y_last, _, _) = grid.calculate_slot_bounds(3);
        let (x_huge, y_huge, _, _) = grid.calculate_slot_bounds(9999);
        assert_eq!(x_last, x_huge);
        assert_eq!(y_last, y_huge);
    }
}

