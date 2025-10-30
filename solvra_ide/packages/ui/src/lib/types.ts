export interface ProjectNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: ProjectNode[];
}

export interface TaskOutcome {
  command: string;
  stdout: string;
  stderr: string;
  exit_code: number | null;
}

export interface RunTaskPayload {
  command: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  timeout?: number;
  shell?: boolean;
}

export interface EditorTabViewModel {
  id: string;
  title: string;
  content: string;
  path: string;
  dirty: boolean;
}

export interface CompletionItem {
  label: string;
  detail?: string;
  kind: 'variable' | 'function' | 'keyword';
}
