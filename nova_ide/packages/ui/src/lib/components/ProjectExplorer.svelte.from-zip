<script lang="ts">
  import { onMount } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import TreeItem from './TreeItem.svelte';
  import type { ProjectNode } from '../types';

  export let project: ProjectNode | undefined;

  const dispatch = createEventDispatcher();
  let expanded: string[] = [];

  onMount(() => {
    if (project) {
      expanded = [project.path];
    }
  });

  $: if (project && expanded.length === 0) {
    expanded = [project.path];
  }

  function toggle(node: ProjectNode) {
    if (!node.is_dir) {
      dispatch('open', node.path);
      return;
    }
    if (expanded.includes(node.path)) {
      expanded = expanded.filter((path) => path !== node.path);
    } else {
      expanded = [...expanded, node.path];
    }
  }

  function openNode(node: ProjectNode) {
    dispatch('open', node.path);
  }

  function moveNode(source: ProjectNode, target: ProjectNode) {
    dispatch('move', { source, target });
  }

  function action(type: string, node: ProjectNode) {
    dispatch(type as 'new', { node });
  }
</script>

{#if project}
  <ul class="tree" data-testid="project-tree">
    <TreeItem
      node={project}
      level={0}
      {expanded}
      on:toggle={(event) => toggle(event.detail)}
      on:open={(event) => openNode(event.detail)}
      on:move={(event) => moveNode(event.detail.source, event.detail.target)}
      on:action={(event) => action(event.detail.type, event.detail.node)}
    />
  </ul>
{:else}
  <div class="empty">Open a workspace to explore files.</div>
{/if}

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty {
    padding: 24px;
    opacity: 0.7;
  }
</style>
