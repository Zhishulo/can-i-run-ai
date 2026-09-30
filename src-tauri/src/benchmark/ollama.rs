use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub model: String,
    pub ttft_sec: f64,
    pub prompt_eval_tok_per_sec: f64,
    pub generation_tok_per_sec: f64,
    pub total_tokens: u64,
    pub total_duration_sec: f64,
    pub sample_output: String,
    /// Epoch milliseconds, matches `new Date(ts)` on the frontend.
    pub timestamp: u64,
}

pub async fn check_ollama_status() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Raw pass-through of Ollama's /api/tags payload
/// (`{ models: [...] }`); the frontend types the fields it uses.
pub async fn list_ollama_models() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| format!("Ollama is not reachable: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}
