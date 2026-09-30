export interface CpuInfo {
  model: string;
  cores: number;
  arch: string;
  isAppleSilicon: boolean;
}

export interface MemoryInfo {
  totalBytes: number;
  totalGb: number;
  freeBytes: number;
  freeGb: number;
  isUnified: boolean;
}

export interface GpuInfo {
  model: string;
  isUnifiedMemory: boolean;
  metalSupport: string;
  vramGb: number;
  /** nvidia | amd | intel | apple | unknown */
  vendor: string;
  /** Driver version when the runtime exposes one (NVML / WMI). */
  driverVersion: string | null;
}

export interface HardwareSpecs {
  platform: string;
  arch: string;
  osName: string;
  osVersion: string;
  cpu: CpuInfo;
  memory: MemoryInfo;
  gpu: GpuInfo;
  backends: string[];
}

export type QuantizationType = 'Q2_K' | 'Q3_K_M' | 'Q4_K_M' | 'Q5_K_M' | 'Q6_K' | 'Q8_0' | 'FP16';

export interface AIModelDefinition {
  id: string;
  name: string;
  family: 'Qwen' | 'Llama' | 'DeepSeek' | 'Gemma' | 'Mistral' | 'Phi';
  parameterCountBillion: number; // e.g., 8 for 8B
  defaultQuantization: QuantizationType;
  supportedQuantizations: QuantizationType[];
  maxContextLength: number; // e.g., 32768
  layers: number;
  heads: number;
  kvHeads?: number | null; // Grouped-Query Attention (GQA)
  headDim: number;
  description: string;
  recommendedUse: string;
  ollamaName?: string | null;
}

export type CompatibilityStatus = 'recommended' | 'runnable' | 'not-recommended' | 'incompatible';

export interface CompatibilityReport {
  status: CompatibilityStatus;
  statusLabel: string;
  totalRequiredMemoryGb: number;
  weightsMemoryGb: number;
  kvCacheMemoryGb: number;
  overheadMemoryGb: number;
  buffersMemoryGb: number;
  usableMemoryGb: number;
  memoryUsagePercentage: number;
  estimatedTokensPerSecond: number;
  headline: string;
  advice: string;
  canOffloadToGpu: boolean;
  /** vram | ram | context | none — what actually limits this model. */
  bottleneck: string;
}

/**
 * One model (fields flattened by the Rust engine) plus a
 * CompatibilityReport per supported quantization.
 */
export interface ModelEvaluation extends AIModelDefinition {
  reports: Partial<Record<QuantizationType, CompatibilityReport>>;
}

export interface OllamaModelDetail {
  name: string;
  model: string;
  size: number;
  digest: string;
  details: {
    format: string;
    family: string;
    parameter_size: string;
    quantization_level: string;
    context_length?: number;
  };
}

export interface BenchmarkMetrics {
  model: string;
  ttftSec: number;
  promptEvalTokPerSec: number;
  generationTokPerSec: number;
  totalTokens: number;
  totalDurationSec: number;
  sampleOutput: string;
  /** Epoch milliseconds (rendered via `new Date(ts)`). */
  timestamp: number;
  /** Medians are computed over this many measured runs (warm-up excluded). */
  runsCompleted: number;
  /** Min/max across measured runs — variance the UI can surface. */
  ttftMinSec: number;
  ttftMaxSec: number;
  generationMinTokPerSec: number;
  generationMaxTokPerSec: number;
}
