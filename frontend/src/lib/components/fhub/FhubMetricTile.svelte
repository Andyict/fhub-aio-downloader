<script lang="ts">
  export let label = "Chỉ số";
  export let value = "0";
  export let icon = "analytics";
  export let note = "FHUB metric";
</script>

<article class="fhub-metric-tile" aria-label={label}>
  <span class="material-icons" aria-hidden="true">{icon}</span>
  <div>
    <small>{label}</small>
    <strong>{value}</strong>
    <em>{note}</em>
  </div>
</article>

<style>
  .fhub-metric-tile {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    min-height: 84px;
    padding: 1rem;
    border: 1px solid rgba(245, 184, 92, 0.15);
    border-radius: 22px;
    background: linear-gradient(180deg, rgba(29, 18, 13, 0.82), rgba(7, 5, 4, 0.68));
    box-shadow: 0 18px 52px rgba(0, 0, 0, 0.30), inset 0 1px 0 rgba(255, 255, 255, 0.045);
  }

  .material-icons {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border-radius: 15px;
    background: rgba(245, 184, 92, 0.12);
    color: #f5b85c;
  }

  small,
  strong,
  em {
    display: block;
  }

  small {
    color: #c7b5a2;
    font-size: 0.68rem;
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  strong {
    margin-top: 0.12rem;
    color: #fff8ee;
    font-size: 1.22rem;
    letter-spacing: -0.045em;
  }

  em {
    margin-top: 0.12rem;
    color: rgba(199, 181, 162, 0.78);
    font-size: 0.72rem;
    font-style: normal;
  }
</style>
