<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { TaskOutcome } from '../types';

  export let lastOutcome: TaskOutcome | undefined;

  const dispatch = createEventDispatcher();
  let command = 'solvra build && solvra run';

  function run() {
    dispatch('run', command);
  }
</script>

<section class="panel" aria-label="Task runner">
  <div class="toolbar">
    <input bind:value={command} aria-label="Task command" />
    <button type="button" on:click={run}>Run</button>
    <button type="button" class="publish" on:click={() => dispatch('publish', command)}>
      Publish
    </button>
  </div>
  {#if lastOutcome}
    <div class="output" data-testid="task-output">
      <header>
        <strong>{lastOutcome.command}</strong>
        {#if lastOutcome.exit_code !== null}
          <span>exit {lastOutcome.exit_code}</span>
        {/if}
      </header>
      {#if lastOutcome.stdout}
        <pre>{lastOutcome.stdout}</pre>
      {/if}
      {#if lastOutcome.stderr}
        <pre class="error">{lastOutcome.stderr}</pre>
      {/if}
    </div>
  {:else}
    <p class="placeholder">Run a task to see logs.</p>
  {/if}
</section>

<style>
  .panel {
    background: rgba(15, 23, 42, 0.45);
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.18);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .toolbar {
    display: flex;
    gap: 8px;
  }

  input {
    flex: 1;
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.25);
    padding: 8px;
    background: rgba(15, 23, 42, 0.6);
    color: inherit;
  }

  button {
    border-radius: 8px;
    border: 1px solid rgba(148, 163, 184, 0.3);
    background: transparent;
    color: inherit;
    padding: 8px 16px;
    cursor: pointer;
  }

  .publish {
    border-color: rgba(34, 197, 94, 0.4);
  }

  .output {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(15, 23, 42, 0.5);
    border-radius: 8px;
    padding: 12px;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
  }

  .error {
    color: #f87171;
  }

  .placeholder {
    opacity: 0.65;
  }
</style>
