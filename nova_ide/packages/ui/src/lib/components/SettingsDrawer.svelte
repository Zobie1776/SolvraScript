<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import type { Keybinding } from '../stores/keybindings';

  export let keybindings: {
    subscribe: (run: (value: Keybinding[]) => void) => () => void;
    updateBinding: (id: string, keys: string) => void;
    reset: () => void;
  };
  export let theme: 'light' | 'dark' | 'solarized' = 'dark';

  const dispatch = createEventDispatcher();
  const themeOptions: Array<typeof theme> = ['light', 'dark', 'solarized'];
  let bindings: Keybinding[] = [];
  let search = '';
  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    unsubscribe = keybindings.subscribe((value) => {
      bindings = value;
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });

  $: filtered = bindings.filter((item) =>
    item.description.toLowerCase().includes(search.toLowerCase()) ||
    item.command.toLowerCase().includes(search.toLowerCase())
  );

  function changeTheme(value: typeof theme) {
    dispatch('theme', value);
  }

  function updateBinding(id: string, keys: string) {
    keybindings.updateBinding(id, keys);
    dispatch('keybinding', { id, keys });
  }
</script>

<aside class="drawer" aria-label="Settings">
  <section>
    <h2>Theme</h2>
    <div class="theme-grid">
      {#each themeOptions as option}
        <button
          type="button"
          class:selected={option === theme}
          on:click={() => changeTheme(option)}
        >
          {option}
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h2>Keybindings</h2>
    <input
      placeholder="Filter commands"
      bind:value={search}
      class="search"
      aria-label="Search keybindings"
    />
    <div class="keybinding-list">
      {#each filtered as binding}
        <label>
          <span>{binding.description}</span>
          <input
            value={binding.keys}
            on:change={(event) => updateBinding(binding.id, event.currentTarget.value)}
          />
        </label>
      {/each}
    </div>
    <button type="button" class="command" on:click={() => keybindings.reset()}>
      Reset defaults
    </button>
  </section>
</aside>

<style>
  .drawer {
    width: 280px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    background: rgba(15, 23, 42, 0.55);
    border-left: 1px solid rgba(148, 163, 184, 0.18);
  }

  h2 {
    font-size: 14px;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  button {
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.25);
    background: transparent;
    color: inherit;
    padding: 8px;
    cursor: pointer;
    text-transform: capitalize;
  }

  button.selected {
    background: rgba(59, 130, 246, 0.2);
  }

  .search {
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.25);
    background: rgba(15, 23, 42, 0.6);
    color: inherit;
  }

  .keybinding-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 240px;
    overflow-y: auto;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  label input {
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.25);
    background: rgba(15, 23, 42, 0.6);
    color: inherit;
  }

  .command {
    margin-top: 12px;
    width: 100%;
  }
</style>
