use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::ollama::BenchmarkResult;

/// Warm-up request token budget: just enough to force model load.
const WARMUP_NUM_PREDICT: u32 = 16;
/// Number of measured runs; medians are reported, min/max exposed
/// so the UI can show variance (docs/technical-route.md D4).
const MEASURED_RUNS: usize = 3;
const NUM_PREDICT: u32 = 128;
const PROMPT: &str = "Explain in three concise bullet points why local AI inference is beneficial for privacy, speed, and offline access.";

#[derive(Debug, Clone)]
struct RunMetrics {
    ttft_sec: f64,
    prompt_eval_tok_per_sec: f64,
    generation_tok_per_sec: f64,
    total_tokens: u64,
    total_duration_sec: f64,
    sample_output: String,
}

/// Single streaming request against a local Ollama instance.
///
/// `stream: true` is what makes a real TTFT measurement possible:
/// the clock stops on the first chunk the server emits. Generation
/// and prompt speeds come from Ollama's own final summary frame
/// (`eval_count` / `eval_duration` and `prompt_eval_*`), which the
/// server measures internally.
async fn stream_once(
    client: &reqwest::Client,
    model_name: &str,
    num_predict: u32,
) -> Result<RunMetrics, String> {
    let start = Instant::now();
    let mut res = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": model_name,
            "prompt": PROMPT,
            "stream": true,
            "options": {
                "num_predict": num_predict,
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

    Ok(RunMetrics {
        ttft_sec,
        prompt_eval_tok_per_sec: if prompt_duration > 0.0 {
            prompt_count as f64 / prompt_duration
        } else {
            0.0
        },
        generation_tok_per_sec: if eval_duration > 0.0 {
            eval_count as f64 / eval_duration
        } else {
            0.0
        },
        total_tokens: eval_count,
        total_duration_sec: start.elapsed().as_secs_f64(),
        sample_output,
    })
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// Warm-up (absorbs cold model load), then several measured runs;
/// medians are reported and variance is exposed via min/max.
pub async fn run_benchmark(model_name: &str) -> Result<BenchmarkResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    // Warm-up: discarded, but a failure here is a real error
    // (runtime down / model not pulled).
    stream_once(&client, model_name, WARMUP_NUM_PREDICT).await?;

    let mut runs = Vec::with_capacity(MEASURED_RUNS);
    for i in 0..MEASURED_RUNS {
        runs.push(stream_once(&client, model_name, NUM_PREDICT).await.map_err(|e| {
            format!("Measurement run {}/{} failed: {e}", i + 1, MEASURED_RUNS)
        })?);
    }

    let mut ttfts: Vec<f64> = runs.iter().map(|r| r.ttft_sec).collect();
    let mut prefills: Vec<f64> = runs.iter().map(|r| r.prompt_eval_tok_per_sec).collect();
    let mut gens: Vec<f64> = runs.iter().map(|r| r.generation_tok_per_sec).collect();
    let mut durations: Vec<f64> = runs.iter().map(|r| r.total_duration_sec).collect();

    let ttft_median = median(&mut ttfts);
    let generation_median = median(&mut gens);

    // Representative run = the one closest to the median generation
    // speed; its sample text and token count are shown in the UI.
    let median_run = runs
        .iter()
        .min_by(|a, b| {
            (a.generation_tok_per_sec - generation_median)
                .abs()
                .partial_cmp(&(b.generation_tok_per_sec - generation_median).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
        .expect("at least one measured run");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(BenchmarkResult {
        model: model_name.to_string(),
        ttft_sec: round2(ttft_median),
        prompt_eval_tok_per_sec: round1(median(&mut prefills)),
        generation_tok_per_sec: round1(generation_median),
        total_tokens: median_run.total_tokens,
        total_duration_sec: round2(median(&mut durations)),
        sample_output: median_run.sample_output,
        timestamp,
        runs_completed: runs.len() as u32,
        ttft_min_sec: round2(runs.iter().map(|r| r.ttft_sec).fold(f64::INFINITY, f64::min)),
        ttft_max_sec: round2(runs.iter().map(|r| r.ttft_sec).fold(0.0, f64::max)),
        generation_min_tok_per_sec: round1(runs.iter().map(|r| r.generation_tok_per_sec).fold(f64::INFINITY, f64::min)),
        generation_max_tok_per_sec: round1(runs.iter().map(|r| r.generation_tok_per_sec).fold(0.0, f64::max)),
    })
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
