// src/lib/api/custom-assets.ts — Tauri wrappers for custom checkpoint & pose management
import { invoke } from '@tauri-apps/api/core';
import type { CustomCheckpoint, CustomPose } from '$lib/types';

// ── Checkpoints ──

export async function scanAvailableCheckpoints(): Promise<string[]> {
  return invoke('scan_available_checkpoints');
}

export async function listCustomCheckpoints(): Promise<CustomCheckpoint[]> {
  return invoke('list_custom_checkpoints');
}

export async function addCustomCheckpoint(displayName: string, filename: string): Promise<CustomCheckpoint> {
  return invoke('add_custom_checkpoint', { displayName, filename });
}

export async function deleteCustomCheckpoint(id: number): Promise<void> {
  return invoke('delete_custom_checkpoint', { id });
}

export async function importCheckpointFile(sourcePath: string, targetFilename: string): Promise<string> {
  return invoke('import_checkpoint_file', { sourcePath, targetFilename });
}

// ── Poses ──

export async function scanAvailablePoses(): Promise<string[]> {
  return invoke('scan_available_poses');
}

export async function listCustomPoses(): Promise<CustomPose[]> {
  return invoke('list_custom_poses');
}

export async function addCustomPose(displayName: string, filename: string): Promise<CustomPose> {
  return invoke('add_custom_pose', { displayName, filename });
}

export async function deleteCustomPose(id: number): Promise<void> {
  return invoke('delete_custom_pose', { id });
}

export async function importPoseFile(sourcePath: string, targetFilename: string): Promise<string> {
  return invoke('import_pose_file', { sourcePath, targetFilename });
}
