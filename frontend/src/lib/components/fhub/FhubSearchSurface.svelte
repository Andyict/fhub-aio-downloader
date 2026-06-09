<script lang="ts">
  interface Props {
    query?: string;
    resultCount?: number;
    savedCount?: number;
    recentCount?: number;
    loading?: boolean;
    hasSearched?: boolean;
  }

  let {
    query = "",
    resultCount = 0,
    savedCount = 0,
    recentCount = 0,
    loading = false,
    hasSearched = false,
  }: Props = $props();

  const stateLabel = $derived(
    loading
      ? "Scanning FHUB sources"
      : hasSearched
        ? `${resultCount} assets ready`
        : "Ready for source discovery",
  );

  const signalItems = $derived([
    { label: "Saved", value: String(savedCount), icon: "star" },
    { label: "Recent", value: String(recentCount), icon: "history" },
    { label: "Results", value: String(resultCount), icon: "dataset" },
  ]);
</script>

<section class="fhub-search-surface" aria-label="FHUB search surface">
  <div class="surface-copy">
    <span class="eyebrow">FHUB Native Search</span>
    <h2>{query ? query : "Discover source assets"}</h2>
    <p>{stateLabel}</p>
  </div>

  <div class="surface-signals" aria-label="FHUB search signals">
    {#each signalItems as item}
      <article class="signal-card">
        <span class="material-icons" aria-hidden="true">{item.icon}</span>
        <div>
          <small>{item.label}</small>
          <strong>{item.value}</strong>
        </div>
      </article>
    {/each}
  </div>
</section>

<style>
  .fhub-search-surface {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(320px, 0.8fr);
    gap: 1rem;
    align-items: stretch;
    padding: 1.15rem;
    border: 1px solid rgba(245, 184, 92, 0.16);
    border-radius: 24px;
    background:
      radial-gradient(circle at 15% 20%, rgba(245, 184, 92, 0.16), transparent 34%),
      linear-gradient(180deg, rgba(28, 18, 12, 0.88), rgba(8, 6, 5, 0.78));
    box-shadow: 0 20px 70px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }

  .surface-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    justify-content: center;
    padding: 0.4rem 0.25rem;
  }

  .eyebrow {
    color: #f5b85c;
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  h2 {
    margin: 0.25rem 0 0;
    overflow: hidden;
    color: #fff8ee;
    font-size: clamp(1.25rem, 3vw, 2rem);
    letter-spacing: -0.045em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  p {
    margin: 0.35rem 0 0;
    color: #c7b5a2;
  }

  .surface-signals {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.65rem;
  }

  .signal-card {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
    padding: 0.85rem;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 18px;
    background: rgba(0, 0, 0, 0.2);
  }

  .material-icons {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 34px;
    height: 34px;
    border-radius: 12px;
    background: rgba(245, 184, 92, 0.12);
    color: #f5b85c;
    font-size: 19px;
  }

  small,
  strong {
    display: block;
  }

  small {
    color: #a89582;
    font-size: 0.64rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  strong {
    margin-top: 0.1rem;
    color: #fff8ee;
    font-family: var(--font-mono, monospace);
    font-size: 1rem;
  }

  @media (max-width: 900px) {
    .fhub-search-surface {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 560px) {
    .surface-signals {
      grid-template-columns: 1fr;
    }
  }
</style>
