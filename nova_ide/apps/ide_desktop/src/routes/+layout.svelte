<script lang="ts">
  import '../lib/styles/global.css';
  import { setTheme, workspace } from '$lib/stores/workspace';
  import { onMount } from 'svelte';

  let systemTheme: 'light' | 'dark' = 'dark';

  onMount(() => {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
    systemTheme = prefersDark.matches ? 'dark' : 'light';
    prefersDark.addEventListener('change', (event) => {
      systemTheme = event.matches ? 'dark' : 'light';
      setTheme(systemTheme);
    });
    setTheme(systemTheme);
  });

  $: document.body.dataset.theme = $workspace.theme;
</script>

<svelte:head>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    href="https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;500;700&display=swap"
    rel="stylesheet"
  />
</svelte:head>

<slot />
