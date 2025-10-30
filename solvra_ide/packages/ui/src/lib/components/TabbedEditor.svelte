<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import type { EditorTabViewModel } from '../types';

  export let tabs: EditorTabViewModel[] = [];
  export let theme: 'light' | 'dark' | 'solarized' = 'dark';
  type CompletionProvider = (
    source: string,
    position: { line: number; character: number }
  ) => Promise<
    { label: string; detail?: string; kind?: 'variable' | 'function' | 'keyword' }[]
  >;

  export let completionProvider: CompletionProvider | null = null;

  const dispatch = createEventDispatcher();

  let activeIndex = 0;
  let splitView = false;
  let primaryContainer: HTMLDivElement | null = null;
  let secondaryContainer: HTMLDivElement | null = null;
  let monaco: typeof import('monaco-editor') | null = null;
  let primaryEditor: import('monaco-editor').editor.IStandaloneCodeEditor | null = null;
  let secondaryEditor: import('monaco-editor').editor.IStandaloneCodeEditor | null = null;
  const models = new Map<string, import('monaco-editor').editor.ITextModel>();
  let completionDisposable: import('monaco-editor').IDisposable | null = null;

  const activeTab = () => tabs[activeIndex];

  onMount(async () => {
    monaco = await import('monaco-editor');
    defineThemes();
    createEditors();
    updateEditorModel();
  });

  onDestroy(() => {
    primaryEditor?.dispose();
    secondaryEditor?.dispose();
    models.forEach((model) => model.dispose());
    completionDisposable?.dispose();
  });

  $: if (monaco && primaryEditor) {
    applyTheme(theme);
  }

  $: if (monaco) {
    updateEditorModel();
    configureCompletions();
  }

  $: if (activeIndex >= tabs.length) {
    activeIndex = Math.max(0, tabs.length - 1);
    updateEditorModel();
  }

  function defineThemes() {
    if (!monaco) return;
    monaco.editor.defineTheme('solvra-solarized', {
      base: 'vs-dark',
      inherit: true,
      rules: [],
      colors: {
        'editor.background': '#002b36',
        'editor.foreground': '#fdf6e3',
        'editorCursor.foreground': '#b58900',
        'editor.lineHighlightBackground': '#073642',
        'editorLineNumber.foreground': '#657b83'
      }
    });
  }

  function applyTheme(value: typeof theme) {
    if (!monaco) return;
    const themeKey = value === 'solarized' ? 'solvra-solarized' : value === 'dark' ? 'vs-dark' : 'vs';
    monaco.editor.setTheme(themeKey);
  }

  function createEditors() {
    if (!monaco || !primaryContainer) return;
    primaryEditor = monaco.editor.create(primaryContainer, {
      value: '',
      language: 'solvrascript',
      automaticLayout: true,
      minimap: { enabled: false }
    });
    primaryEditor.onDidChangeModelContent(() => {
      const tab = activeTab();
      if (!tab) return;
      const value = primaryEditor?.getValue() ?? '';
      dispatch('update', { id: tab.id, content: value });
      dispatch('dirty', { id: tab.id, dirty: value !== tab.content });
    });
  }

  function ensureSecondary() {
    if (!monaco || !secondaryContainer || secondaryEditor) return;
    secondaryEditor = monaco.editor.create(secondaryContainer, {
      value: primaryEditor?.getValue() ?? '',
      language: 'solvrascript',
      automaticLayout: true,
      readOnly: false,
      minimap: { enabled: false }
    });
    secondaryEditor.onDidChangeModelContent(() => {
      const tab = activeTab();
      if (!tab) return;
      const value = secondaryEditor?.getValue() ?? '';
      dispatch('update', { id: tab.id, content: value });
      dispatch('dirty', { id: tab.id, dirty: value !== tab.content });
    });
  }

  function disposeSecondary() {
    secondaryEditor?.dispose();
    secondaryEditor = null;
  }

  function toggleSplit() {
    splitView = !splitView;
    if (splitView) {
      ensureSecondary();
      updateEditorModel();
    } else {
      disposeSecondary();
    }
  }

  function setActive(index: number) {
    activeIndex = index;
    updateEditorModel();
    const tab = activeTab();
    if (tab) {
      dispatch('activate', tab);
    }
  }

  function close(tab: EditorTabViewModel, event: Event) {
    event.stopPropagation();
    dispatch('close', tab.id);
  }

  function save() {
    const tab = activeTab();
    if (tab) {
      dispatch('save', tab);
    }
  }

  function updateEditorModel() {
    if (!monaco || !primaryEditor) return;
    const tab = activeTab();
    if (!tab) {
      primaryEditor.setModel(null);
      secondaryEditor?.setModel(null);
      return;
    }

    let model = models.get(tab.path);
    if (!model) {
      const uri = monaco.Uri.parse(`file://${tab.path}`);
      model = monaco.editor.createModel(tab.content, 'solvrascript', uri);
      models.set(tab.path, model);
    } else if (model.getValue() !== tab.content) {
      model.setValue(tab.content);
    }

    primaryEditor.setModel(model);
    if (splitView) {
      ensureSecondary();
      secondaryEditor?.setModel(model);
    }
  }

  function configureCompletions() {
    if (!monaco || !completionProvider) {
      completionDisposable?.dispose();
      completionDisposable = null;
      return;
    }

    completionDisposable?.dispose();
    completionDisposable = monaco.languages.registerCompletionItemProvider('solvrascript', {
      triggerCharacters: ['.', ':'],
      provideCompletionItems: async (
        model: import('monaco-editor').editor.ITextModel,
        position: import('monaco-editor').Position
      ) => {
        const monacoApi = monaco;
        if (!monacoApi) {
          return { suggestions: [] };
        }

        const items = await completionProvider(model.getValue(), {
          line: position.lineNumber,
          character: position.column
        });
        const word = model.getWordUntilPosition(position);
        const range = new monacoApi.Range(
          position.lineNumber,
          word.startColumn,
          position.lineNumber,
          word.endColumn
        );
        const suggestions = items.map((item) => ({
          label: item.label,
          detail: item.detail,
          kind:
            item.kind === 'function'
              ? monacoApi.languages.CompletionItemKind.Function
              : item.kind === 'keyword'
                ? monacoApi.languages.CompletionItemKind.Keyword
                : monacoApi.languages.CompletionItemKind.Variable,
          insertText: item.label,
          range
        }));
        return { suggestions };
      }
    });
  }

  function handleTabKey(event: KeyboardEvent, index: number) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      setActive(index);
    }
  }
</script>

<div class={`container ${splitView ? 'split' : ''}`}>
  <div class="tab-bar" role="tablist">
    {#each tabs as tab, index}
      <div
        class:active={index === activeIndex}
        class="tab"
        role="tab"
        tabindex="0"
        aria-selected={index === activeIndex}
        on:click={() => setActive(index)}
        on:keydown={(event) => handleTabKey(event, index)}
      >
        <span>{tab.title}</span>
        {#if tab.dirty}
          <span class="dirty" aria-label="Unsaved changes">●</span>
        {/if}
        <button
          type="button"
          class="close"
          aria-label={`Close ${tab.title}`}
          on:click={(event) => close(tab, event)}
        >
          ×
        </button>
      </div>
    {/each}
    <div class="spacer" />
    <button class="command" on:click={save}>Save</button>
    <button class="command" on:click={toggleSplit}>{splitView ? 'Single' : 'Split'}</button>
  </div>
  <div class="panes">
    <div class="editor" bind:this={primaryContainer} aria-label="Primary editor" />
    {#if splitView}
      <div class="editor" bind:this={secondaryContainer} aria-label="Secondary editor" />
    {/if}
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(15, 23, 42, 0.45);
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.15);
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px;
    background: rgba(15, 23, 42, 0.65);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: rgba(226, 232, 240, 0.86);
    cursor: pointer;
    font-family: inherit;
  }

  .tab.active {
    background: rgba(59, 130, 246, 0.24);
  }

  .tab:focus-visible {
    outline: 2px solid rgba(59, 130, 246, 0.4);
    outline-offset: 2px;
  }

  .close {
    font-size: 14px;
    cursor: pointer;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0;
  }

  .dirty {
    color: #f97316;
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .command {
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.3);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .panes {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    padding: 8px;
    min-height: 300px;
  }

  .container.split .panes {
    grid-template-columns: 1fr 1fr;
  }

  .editor {
    position: relative;
    min-height: 280px;
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.2);
    overflow: hidden;
  }
</style>
