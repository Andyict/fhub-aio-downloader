<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children?: Snippet;
    actions?: Snippet;
    title?: string;
    subtitle?: string;
  }

  let {
    children,
    actions,
    title = "FHUB",
    subtitle = "Cinema NAS workspace",
  }: Props = $props();
</script>

<section class="fhub-shell-frame" aria-label="FHUB workspace frame">
  <header class="frame-header">
    <div>
      <span>FHUB WORKSPACE</span>
      <h2>{title}</h2>
      <p>{subtitle}</p>
    </div>
    {#if actions}
      {@render actions()}
    {/if}
  </header>

  <div class="frame-body">
    {#if children}
      {@render children()}
    {/if}
  </div>
</section>

<style>
  .fhub-shell-frame {
    display: grid;
    gap: 1rem;
    min-width: 0;
  }

  .frame-header {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid rgba(245, 184, 92, 0.14);
    border-radius: 24px;
    background: linear-gradient(180deg, rgba(29, 18, 13, 0.72), rgba(7, 5, 4, 0.58));
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.045);
  }

  span {
    color: #f5b85c;
    font-size: 0.66rem;
    font-weight: 950;
    letter-spacing: 0.24em;
    text-transform: uppercase;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    margin-top: 0.2rem;
    color: #fff8ee;
    font-size: clamp(1.25rem, 3vw, 2.15rem);
    letter-spacing: -0.055em;
  }

  p {
    margin-top: 0.25rem;
    color: #c7b5a2;
  }

  .frame-body {
    min-width: 0;
  }

  @media (max-width: 760px) {
    .frame-header {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
