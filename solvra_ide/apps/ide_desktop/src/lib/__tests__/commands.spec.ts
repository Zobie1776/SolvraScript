import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({})
}));

const { invoke } = await import('@tauri-apps/api/core');
const { openFile, runTask } = await import('../commands');

describe('Desktop command bindings', () => {
  it('wraps open_file Tauri command', async () => {
    await openFile('/tmp/demo');
    expect(invoke).toHaveBeenCalledWith('open_file', { path: '/tmp/demo' });
  });

  it('sends run_task payload', async () => {
    await runTask({ command: 'echo hi', args: [], shell: true });
    expect(invoke).toHaveBeenCalledWith('run_task', {
      payload: { command: 'echo hi', args: [], shell: true }
    });
  });
});
