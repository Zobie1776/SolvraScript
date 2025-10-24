<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ProjectExplorer,
    TabbedEditor,
    IntegratedTerminal,
    SettingsDrawer,
    TaskRunnerPanel,
    type ProjectNode,
    type EditorTabViewModel,
    type TaskOutcome,
    createKeybindingStore
  } from '@nova-ide/ui';
  import { NovaLspClient } from '@nova-ide/lsp-client';
  import {
    closeTab,
    markDirty,
    openTab,
    setActiveTab,
    setProjectTree,
    setRunningTask,
    setTheme,
    updateContent,
    workspace,
    type WorkspaceState
  } from '$lib/stores/workspace';
  import {
    configureFailSafe,
    openFile,
    readProjectTree,
    runTask,
    saveFile,
    showError,
    verifyFailSafe
  } from '$lib/commands';

  const keybindings = createKeybindingStore();
  const lsp = new NovaLspClient();

  let projectRoot = '';
  let tabs: EditorTabViewModel[] = [];

  const updateTabs = (state: WorkspaceState) => {
    tabs = state.tabs.map((tab) => ({
      id: tab.id,
      title: tab.title,
      content: tab.content,
      dirty: tab.dirty,
      path: tab.path
    }));
  };

  onMount(() => {
    const unsubscribe = workspace.subscribe((state) => updateTabs(state));

    (async () => {
      try {
        const response = await readProjectTree('.');
        setProjectTree(response as ProjectNode);
        projectRoot = response.path;
      } catch (error) {
        await showError(`Unable to load project: ${error}`);
      }
    })();

    return () => {
      unsubscribe();
    };
  });

  async function handleOpen(path: string) {
    try {
      const { content } = await openFile(path);
      openTab({ id: path, title: path.split('/').at(-1) ?? path, content, path, dirty: false });
    } catch (error) {
      await showError(`Failed to open file: ${error}`);
    }
  }

  async function handleSave(tab: EditorTabViewModel) {
    try {
      await saveFile(tab.path, tab.content);
      markDirty(tab.id, false);
    } catch (error) {
      await showError(`Failed to save file: ${error}`);
    }
  }

  async function handleRun(command: string) {
    try {
      const outcome = (await runTask({ command, args: [], cwd: projectRoot, shell: true })) as TaskOutcome;
      setRunningTask(outcome);
    } catch (error) {
      await showError(`Task failed: ${error}`);
    }
  }

  async function handlePublish(command: string) {
    const passphrase = window.prompt('Enter publish passphrase');
    if (!passphrase) return;
    try {
      await verifyFailSafe(passphrase);
    } catch (error) {
      if (String(error).includes('no passphrase')) {
        await configureFailSafe(passphrase);
      } else {
        await showError(`Publish blocked: ${error}`);
        return;
      }
    }
    await handleRun(command);
  }
</script>

<div class="layout">
  <aside class="sidebar">
    <ProjectExplorer project={$workspace.root} on:open={(event) => handleOpen(event.detail)} />
  </aside>
  <main class="editor">
    <TabbedEditor
      {tabs}
      theme={$workspace.theme}
      completionProvider={(source, position) => lsp.completions(source, position)}
      on:save={(event) => handleSave(event.detail)}
      on:close={(event) => closeTab(event.detail)}
      on:dirty={(event) => markDirty(event.detail.id, event.detail.dirty)}
      on:update={(event) => updateContent(event.detail.id, event.detail.content)}
      on:activate={(event) => setActiveTab(event.detail.id)}
    />
    <TaskRunnerPanel
      lastOutcome={$workspace.runningTask}
      on:run={(event) => handleRun(event.detail)}
      on:publish={(event) => handlePublish(event.detail)}
    />
    <IntegratedTerminal {projectRoot} />
  </main>
  <SettingsDrawer
    {keybindings}
    theme={$workspace.theme}
    on:theme={(event) => setTheme(event.detail)}
  />
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 280px 1fr auto;
    height: 100vh;
    background: var(--background-primary);
  }

  .sidebar {
    border-right: 1px solid rgba(148, 163, 184, 0.2);
    background: rgba(15, 23, 42, 0.35);
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
  }
</style>
