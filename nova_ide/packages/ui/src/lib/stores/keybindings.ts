import { writable } from 'svelte/store';

export interface Keybinding {
  id: string;
  command: string;
  keys: string;
  description: string;
}

const defaults: Keybinding[] = [
  { id: 'open', command: 'workspace.open', keys: 'Ctrl+O', description: 'Open file' },
  { id: 'save', command: 'workspace.save', keys: 'Ctrl+S', description: 'Save active tab' },
  { id: 'run', command: 'task.run', keys: 'Ctrl+R', description: 'Run default task' }
];

export function createKeybindingStore(initial: Keybinding[] = defaults) {
  const store = writable(initial);

  function updateBinding(id: string, keys: string) {
    store.update((items) =>
      items.map((binding) => (binding.id === id ? { ...binding, keys } : binding))
    );
  }

  function reset() {
    store.set(defaults);
  }

  return {
    subscribe: store.subscribe,
    updateBinding,
    reset
  };
}
