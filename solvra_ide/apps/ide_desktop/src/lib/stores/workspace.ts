import { derived, writable } from 'svelte/store';
import type { ProjectNode, TaskOutcome } from '@solvra-ide/ui';

export interface EditorTab {
  id: string;
  title: string;
  path: string;
  dirty: boolean;
  content: string;
}

export interface WorkspaceState {
  root?: ProjectNode;
  tabs: EditorTab[];
  activeTab?: string;
  theme: 'light' | 'dark' | 'solarized';
  runningTask?: TaskOutcome;
}

const defaultState: WorkspaceState = {
  tabs: [],
  theme: 'dark'
};

const workspaceStore = writable<WorkspaceState>(defaultState);

export const workspace = {
  subscribe: workspaceStore.subscribe,
  set: workspaceStore.set,
  update: workspaceStore.update
};

export const activeTab = derived(workspaceStore, ($workspace) =>
  $workspace.tabs.find((tab) => tab.id === $workspace.activeTab)
);

export function openTab(tab: EditorTab) {
  workspaceStore.update((state) => {
    const existing = state.tabs.find((item) => item.id === tab.id);
    const tabs = existing
      ? state.tabs.map((item) => (item.id === tab.id ? { ...item, ...tab } : item))
      : [...state.tabs, tab];
    return {
      ...state,
      tabs,
      activeTab: tab.id
    };
  });
}

export function closeTab(id: string) {
  workspaceStore.update((state) => {
    const tabs = state.tabs.filter((tab) => tab.id !== id);
    const activeTab = state.activeTab === id ? tabs.at(-1)?.id : state.activeTab;
    return { ...state, tabs, activeTab };
  });
}

export function setActiveTab(id: string) {
  workspaceStore.update((state) => ({ ...state, activeTab: id }));
}

export function markDirty(id: string, dirty: boolean) {
  workspaceStore.update((state) => ({
    ...state,
    tabs: state.tabs.map((tab) => (tab.id === id ? { ...tab, dirty } : tab))
  }));
}

export function updateContent(id: string, content: string) {
  workspaceStore.update((state) => ({
    ...state,
    tabs: state.tabs.map((tab) =>
      tab.id === id
        ? {
            ...tab,
            content
          }
        : tab
    )
  }));
}

export function setTheme(theme: WorkspaceState['theme']) {
  workspaceStore.update((state) => ({ ...state, theme }));
}

export function setProjectTree(root: ProjectNode) {
  workspaceStore.update((state) => ({ ...state, root }));
}

export function setRunningTask(task?: TaskOutcome) {
  workspaceStore.update((state) => ({ ...state, runningTask: task }));
}
