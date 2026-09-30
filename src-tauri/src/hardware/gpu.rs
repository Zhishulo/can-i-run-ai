use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub model: String,
    pub is_unified_memory: bool,
    pub vram_gb: f64,
    /// Backend label shown in the UI: "Metal" on macOS, "CUDA" /
    /// "DirectX 12" on Windows, "Unknown" when detection fails.
    pub metal_support: String,
}
