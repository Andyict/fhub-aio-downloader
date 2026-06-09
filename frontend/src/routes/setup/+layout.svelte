<script lang="ts">
  import { FhubAuthShell } from "$lib/components";

  let { children } = $props();
</script>

<svelte:head>
  <title>FHUB Setup</title>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link
    rel="preconnect"
    href="https://fonts.gstatic.com"
    crossorigin="anonymous"
  />
  <link
    href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
    rel="stylesheet"
  />
  <link
    href="https://fonts.googleapis.com/icon?family=Material+Icons"
    rel="stylesheet"
  />
</svelte:head>

<FhubAuthShell title="" eyebrow="" description="">
  {@render children()}
</FhubAuthShell>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: auto;
  }
</style>
