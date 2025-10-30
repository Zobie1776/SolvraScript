import { describe, expect, it, vi } from 'vitest';
import { SolvraLspClient } from '../index';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined)
}));

const { invoke } = await import('@tauri-apps/api/core');

describe('SolvraLspClient', () => {
  it('sends completion requests with cursor location', async () => {
    const client = new SolvraLspClient();
    await client.completions('let a = 1', { line: 1, character: 4 });
    expect(invoke).toHaveBeenCalledWith('lsp_complete', {
      source: 'let a = 1',
      line: 1,
      character: 4
    });
  });
});
