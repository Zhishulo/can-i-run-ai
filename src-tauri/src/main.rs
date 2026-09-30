// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use can_i_run_ai::{benchmark, engine, hardware, models};

use hardware::HardwareInfo;

#[tauri::command]
fn get_hardware_info() -> HardwareInfo {
    hardware::detect_hardware()
}

#[tauri::command]
fn list_models() -> Result<models::ModelDatabase, String> {
    models::load()
}

/// Compatibility reports for every model at every supported
/// quantization, computed against this machine's hardware. One IPC
/// call keeps quantization switching purely a frontend concern.
#[tauri::command]
fn evaluate_models(context_length: usize) -> Result<Vec<engine::ModelEvaluation>, String> {
    let db = models::load()?;
    let hw = hardware::detect_hardware();
    Ok(engine::evaluate_all(&db.models, &hw, context_length))
}

#[tauri::command]
async fn check_ollama_status() -> bool {
    benchmark::ollama::check_ollama_status().await
}

#[tauri::command]
async fn list_ollama_models() -> Result<serde_json::Value, String> {
    benchmark::ollama::list_ollama_models().await
}

#[tauri::command]
async fn run_ollama_benchmark(model: String) -> Result<benchmark::ollama::BenchmarkResult, String> {
    benchmark::runner::run_benchmark(&model).await
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_hardware_info,
            list_models,
            evaluate_models,
            check_ollama_status,
            list_ollama_models,
            run_ollama_benchmark
        ])
        .run(tauri::generate_context!())
        .expect("error while running Can I Run AI application");
}
