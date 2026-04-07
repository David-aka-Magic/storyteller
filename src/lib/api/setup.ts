// src/lib/api/setup.ts — Tauri command wrappers for the dependency setup system
import { invoke } from '@tauri-apps/api/core';
import type { SetupStatus, GpuInfo, OllamaGpuStatus } from '$lib/types/setup';

export async function checkSetupStatus(): Promise<SetupStatus> {
  return invoke('check_setup_status');
}

export async function installDependency(name: string): Promise<void> {
  return invoke('install_dependency', { name });
}

export async function installAllDependencies(): Promise<void> {
  return invoke('install_all_dependencies');
}

export async function detectGpu(): Promise<GpuInfo> {
  return invoke('detect_gpu_info');
}

export async function checkOllamaGpuUsage(): Promise<OllamaGpuStatus> {
  return invoke('check_ollama_gpu_usage');
}
