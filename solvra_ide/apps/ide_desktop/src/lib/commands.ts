import { invoke } from '@tauri-apps/api/core';
import type { ProjectNode, RunTaskPayload, TaskOutcome } from '@solvra-ide/ui';

export interface OpenFileResponse {
  path: string;
  content: string;
}

export async function openFile(path: string): Promise<OpenFileResponse> {
  return invoke('open_file', { path });
}

export async function saveFile(path: string, content: string): Promise<void> {
  await invoke('save_file', { path, content });
}

export async function runTask(payload: RunTaskPayload): Promise<TaskOutcome> {
  return invoke('run_task', { payload });
}

export async function readProjectTree(root: string): Promise<ProjectNode> {
  return invoke('read_project_tree', { root });
}

export async function showError(message: string): Promise<void> {
  await invoke('show_error', { message });
}

export async function configureFailSafe(passphrase: string): Promise<void> {
  await invoke('configure_fail_safe', { passphrase });
}

export async function verifyFailSafe(passphrase: string): Promise<void> {
  await invoke('verify_publish_passphrase', { passphrase });
}
