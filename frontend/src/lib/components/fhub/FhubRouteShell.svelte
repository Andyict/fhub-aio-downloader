<script lang="ts">
  import type { Snippet } from "svelte";
  import FhubShellFrame from "./FhubShellFrame.svelte";

  interface Props {
    children: Snippet;
    title?: string;
    subtitle?: string;
    mode?: "dashboard" | "settings" | "auth" | "default";
  }

  let {
    children,
    title = "FHUB",
    subtitle = "Cinema NAS workspace",
    mode = "default",
  }: Props = $props();
</script>

{#if mode === "auth"}
  <section class="route-auth" aria-label={title}>
    {@render children()}
  </section>
{:else}
  <FhubShellFrame {title} {subtitle}>
    <div class={`fhub-route-shell mode-${mode}`}>
      {@render children()}
    </div>
  </FhubShellFrame>
{/if}

<style>
  .fhub-route-shell {
    min-width: 0;
    display: grid;
    gap: 1rem;
  }

  .mode-dashboard {
    gap: 1.1rem;
  }

  .mode-settings {
    gap: 0.9rem;
  }

  .route-auth {
    min-width: 0;
  }
</style>
