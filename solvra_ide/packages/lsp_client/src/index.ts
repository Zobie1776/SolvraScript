import { invoke } from '@tauri-apps/api/core';

export interface TextPosition {
  line: number;
  character: number;
}

export interface CompletionItem {
  label: string;
  detail?: string;
  kind: 'variable' | 'function' | 'keyword';
}

export interface HoverResult {
  contents: string;
}

export interface Diagnostic {
  message: string;
  line: number;
  column: number;
}

export class SolvraLspClient {
  async completions(source: string, position: TextPosition): Promise<CompletionItem[]> {
    return invoke('lsp_complete', { source, line: position.line, character: position.character });
  }

  async hover(source: string, position: TextPosition): Promise<HoverResult | null> {
    const result = await invoke<HoverResult | null>('lsp_hover', {
      source,
      line: position.line,
      character: position.character
    });
    return result;
  }

  async gotoDefinition(source: string, symbol: string): Promise<TextPosition | null> {
    return invoke('lsp_goto_definition', { source, symbol });
  }

  async diagnostics(source: string): Promise<Diagnostic[]> {
    return invoke('lsp_diagnostics', { source });
  }
}
