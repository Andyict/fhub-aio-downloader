<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children?: Snippet;
    eyebrow?: string;
    title?: string;
    description?: string;
  }

  let {
    children,
    eyebrow = "FHUB PANEL",
    title = "Không gian FHUB",
    description = "Một vùng giao diện native của FHUB.",
  }: Props = $props();
</script>

<section class="fhub-panel" aria-label={title}>
  <header>
    <span>{eyebrow}</span>
    <h3>{title}</h3>
    <p>{description}</p>
  </header>

  <div class="panel-content">
    {#if children}
      {@render children()}
    {/if}
  </div>
</section>

<style>
  .fhub-panel {
    display: grid;
    gap: 1rem;
    min-width: 0;
    padding: 1rem;
    border: 1px solid rgba(245, 184, 92, 0.15);
    border-radius: 26px;
    background:
      linear-gradient(180deg, rgba(29, 18, 13, 0.78), rgba(7, 5, 4, 0.68)),
      radial-gradient(circle at 100% 0%, rgba(245, 184, 92, 0.09), transparent 32%);
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.34), inset 0 1px 0 rgba(255, 255, 255, 0.045);
  }

  header,
  .panel-content {
    min-width: 0;
  }

  header {
    display: grid;
    gap: 0.25rem;
  }

  span {
    color: #f5b85c;
    font-size: 0.66rem;
    font-weight: 950;
    letter-spacing: 0.22em;
    text-transform: uppercase;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    color: #fff8ee;
    font-size: clamp(1.05rem, 2vw, 1.45rem);
    letter-spacing: -0.04em;
  }

  p {
    color: #c7b5a2;
    line-height: 1.5;
  }
</style>
