<script lang="ts">
import { createEventDispatcher } from 'svelte';
import type { ProjectNode } from '../types';

let currentDragSource: ProjectNode | null = null;

  export let node: ProjectNode;
  export let level = 0;
  export let expanded: string[] = [];

  const dispatch = createEventDispatcher();
  const isExpanded = () => expanded.includes(node.path);

  function onContextMenu(event: MouseEvent) {
    event.preventDefault();
    if (typeof window === 'undefined') return;
    const action = window.prompt(`Action for ${node.name} (new/rename/delete)`);
    if (!action) return;
    dispatch('action', { type: action, node });
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    const data = event.dataTransfer?.getData('application/json');
    const source = data ? (JSON.parse(data) as ProjectNode) : currentDragSource;
    if (!source || source.path === node.path) {
      return;
    }
    dispatch('move', { source, target: node });
    currentDragSource = null;
  }

  function onDragStart(event: DragEvent) {
    currentDragSource = node;
    event.dataTransfer?.setData('application/json', JSON.stringify(node));
  }

  function onDragOver(event: DragEvent) {
    event.preventDefault();
  }
</script>

<li
  class="entry"
  style={`padding-left: ${level * 16}px`}
  draggable
  on:contextmenu={onContextMenu}
  on:drop={onDrop}
  on:dragover={onDragOver}
  on:dragstart={onDragStart}
  data-testid={`tree-item-${node.name}`}
>
  <button class="toggle" on:click={() => dispatch('toggle', node)}>
    {#if node.is_dir}
      {#if isExpanded()}
        ▼
      {:else}
        ▶
      {/if}
    {:else}
      •
    {/if}
  </button>
  <button
    class="label"
    type="button"
    on:dblclick={() => dispatch('open', node)}
    on:keydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        dispatch('open', node);
      }
    }}
  >
    {node.name}
  </button>
</li>
{#if node.is_dir && isExpanded()}
  <ul class="children">
    {#each node.children as child}
      <svelte:self
        node={child}
        level={level + 1}
        {expanded}
        on:toggle
        on:open
        on:move
        on:action
      />
    {/each}
  </ul>
{/if}

<style>
  .entry {
    list-style: none;
    display: grid;
    grid-template-columns: 16px 1fr;
    gap: 8px;
    align-items: center;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    color: rgba(226, 232, 240, 0.92);
  }

  .entry:hover {
    background: rgba(59, 130, 246, 0.12);
  }

  .toggle {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 10px;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .label {
    font-size: 13px;
    user-select: none;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }

  .children {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
