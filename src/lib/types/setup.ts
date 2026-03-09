// src/lib/types/setup.ts — mirrors Rust types in src-tauri/src/services/setup.rs

export interface DependencyStatus {
  name: string;
  installed: boolean;
  version?: string;
  path?: string;
  error?: string;
}

export interface SetupProgress {
  step: string;
  progress_pct: number; // 0.0 - 1.0
  message: string;
  is_error: boolean;
}

export interface GpuInfo {
  /** "nvidia" | "amd" | "intel" | "unknown" */
  vendor: string;
  name: string;
}

export interface SetupStatus {
  ollama: DependencyStatus;
  ollama_model: DependencyStatus;
  comfyui: DependencyStatus;
  comfyui_torch: DependencyStatus;
  checkpoints: DependencyStatus[];
  custom_nodes: DependencyStatus[];
  gpu_info: GpuInfo;
  all_ready: boolean;
}
