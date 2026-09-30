import { invoke } from '@tauri-apps/api/core';
import { BenchmarkMetrics, HardwareSpecs, ModelEvaluation, OllamaModelDetail } from '../types';

/**
 * Thin wrappers over the Tauri IPC commands (src-tauri/src/main.rs).
 * All errors propagate to the caller so the UI can show explicit
 * loading / failure states instead of silently faking data.
 */
export async function fetchHardwareSpecs(): Promise<HardwareSpecs> {
  return invoke<HardwareSpecs>('get_hardware_info');
}

/**
 * Compatibility reports for every model at every supported
 * quantization, computed by the Rust engine on this machine.
 * Re-invoke whenever the context length changes.
 */
export async function evaluateModels(contextLength: number): Promise<ModelEvaluation[]> {
  return invoke<ModelEvaluation[]>('evaluate_models', { contextLength });
}

export async function fetchOllamaModels(): Promise<{ isRunning: boolean; models: OllamaModelDetail[] }> {
  try {
    const data = await invoke<{ models?: OllamaModelDetail[] }>('list_ollama_models');
    return { isRunning: true, models: data.models ?? [] };
  } catch (_) {
    return { isRunning: false, models: [] };
  }
}

export async function checkOllamaStatus(): Promise<boolean> {
  return invoke<boolean>('check_ollama_status');
}

export async function listModels(): Promise<unknown> {
  return invoke('list_models');
}

export async function runOllamaBenchmark(modelName: string): Promise<BenchmarkMetrics> {
  return invoke<BenchmarkMetrics>('run_ollama_benchmark', { model: modelName });
}
