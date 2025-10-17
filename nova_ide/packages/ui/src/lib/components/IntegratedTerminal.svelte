<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { Terminal } from 'xterm';
  import 'xterm/css/xterm.css';

  export let projectRoot = '.';

  let container: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let dispose: (() => void) | null = null;

  type ShellChild = {
    stdout: { on: (event: 'data', handler: (line: string) => void) => void };
    stderr: { on: (event: 'data', handler: (line: string) => void) => void };
    write: (data: string) => Promise<void>;
    kill: () => void;
  };

  async function spawnShell() {
    if (!container) return;
    terminal = new Terminal({
      convertEol: true,
      fontFamily: 'Fira Code, monospace',
      theme: {
        background: '#0f172a',
        foreground: '#e2e8f0'
      }
    });
    terminal.open(container);

    try {
      const { Command } = await import('@tauri-apps/plugin-shell');
      const command = await Command.create('nova', ['repl'], {
        cwd: projectRoot
      });
      const child = (await command.spawn()) as unknown as ShellChild;

      child.stdout.on('data', (line: string) => terminal?.write(line));
      child.stderr.on('data', (line: string) => terminal?.write(`\r\n${line}`));
      terminal.onData((data: string) => {
        child.write(data).catch((error: string) => terminal?.write(`\r\n${error}`));
      });
      dispose = () => child.kill();
    } catch (error) {
      console.error(error);
      terminal.writeln('NovaCLI unavailable, falling back to system shell.');
      try {
        const { Command } = await import('@tauri-apps/plugin-shell');
        const command = await Command.create('bash', ['-l'], {
          cwd: projectRoot
        });
        const child = (await command.spawn()) as unknown as ShellChild;
        child.stdout.on('data', (line: string) => terminal?.write(line));
        child.stderr.on('data', (line: string) => terminal?.write(`\r\n${line}`));
        terminal.onData((data: string) => {
          child.write(data).catch((err: string) => terminal?.write(`\r\n${err}`));
        });
        dispose = () => child.kill();
      } catch (shellError) {
        terminal.writeln(`Unable to start shell: ${shellError}`);
      }
    }
  }

  onMount(() => {
    spawnShell();
  });

  onDestroy(() => {
    dispose?.();
    terminal?.dispose();
  });
</script>

<div class="terminal" bind:this={container} />

<style>
  .terminal {
    height: 240px;
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.2);
    overflow: hidden;
    background: #0f172a;
  }
</style>
