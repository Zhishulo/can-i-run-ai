use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::ollama::BenchmarkResult;

/// Single streaming benchmark run against a local Ollama instance.
///
/// `stream: true` is what makes a real TTFT measurement possible:
/// the clock stops on the first chunk the server emits. Generation
/// and prompt speeds come from Ollama's own final summary frame
/// (`eval_count` / `eval_duration` and `prompt_eval_*`), which the
/// server measures internally. Warm-up and median-of-N runs are
/// planned for M1 (docs/technical-route.md §D4).
pub async fn run_benchmark(model_name: &str) -> Result<BenchmarkResult, String> {
    let prompt = "Explain in three concise bullet points why local AI inference is beneficial for privacy, speed, and offline access.";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let start = Instant::now();
    let mut res = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": model_name,
            "prompt": prompt,
            "stream": true,
            "options": {
                "num_predict": 128,
                "temperature": 0.0,
                "seed": 42
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Ollama ({e}). Is `ollama serve` running?"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Ollama returned HTTP {status}: {body}"));
    }

    let mut ttft_sec: f64 = 0.0;
    let mut sample_output = String::new();
    let mut final_frame: Option<serde_json::Value> = None;
    let mut buf = String::new();

    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        if ttft_sec == 0.0 {
            ttft_sec = start.elapsed().as_secs_f64();
        }
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // Ollama streams newline-delimited JSON; a chunk boundary can
        // split a line, so only complete lines are consumed here.
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(tok) = v["response"].as_str() {
                    sample_output.push_str(tok);
                }
                if v["done"].as_bool() == Some(true) {
                    final_frame = Some(v);
                }
            }
        }
    }

    if let Some(rest) = buf.lines().last() {
        let rest = rest.trim();
        if !rest.is_empty() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) {
                if let Some(tok) = v["response"].as_str() {
                    sample_output.push_str(tok);
                }
                if v["done"].as_bool() == Some(true) {
                    final_frame = Some(v);
                }
            }
        }
    }

    if ttft_sec == 0.0 {
        return Err("Ollama closed the stream without generating any tokens.".to_string());
    }
    let final_frame = final_frame
        .ok_or_else(|| "Ollama stream ended without a summary frame (done=true).".to_string())?;

    let eval_count = final_frame["eval_count"].as_u64().unwrap_or(0);
    let eval_duration = final_frame["eval_duration"].as_f64().unwrap_or(0.0) / 1e9;
    let prompt_count = final_frame["prompt_eval_count"].as_u64().unwrap_or(0);
    let prompt_duration = final_frame["prompt_eval_duration"].as_f64().unwrap_or(0.0) / 1e9;

    let generation_tok_per_sec = if eval_duration > 0.0 {
        eval_count as f64 / eval_duration
    } else {
        0.0
    };
    let prompt_eval_tok_per_sec = if prompt_duration > 0.0 {
        prompt_count as f64 / prompt_duration
    } else {
        0.0
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(BenchmarkResult {
        model: model_name.to_string(),
        ttft_sec: super::round2(ttft_sec),
        prompt_eval_tok_per_sec: super::round1(prompt_eval_tok_per_sec),
        generation_tok_per_sec: super::round1(generation_tok_per_sec),
        total_tokens: eval_count,
        total_duration_sec: super::round2(start.elapsed().as_secs_f64()),
        sample_output,
        timestamp,
    })
}
