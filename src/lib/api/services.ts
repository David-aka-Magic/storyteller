// src/lib/api/services.ts — Tauri command wrappers for service management
import { invoke } from '@tauri-apps/api/core';

export interface ServiceStatusResponse {
  ollama_running: boolean;
  sd_running: boolean;
  ollama_error: string | null;
  sd_error: string | null;
}

export async function checkServicesStatus(): Promise<ServiceStatusResponse> {
  return invoke('check_services_status');
}

export async function startServices(): Promise<ServiceStatusResponse> {
  return invoke('start_services');
}

export async function stopServices(): Promise<void> {
  return invoke('stop_services');
}
