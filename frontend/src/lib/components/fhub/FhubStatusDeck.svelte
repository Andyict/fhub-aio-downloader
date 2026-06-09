<script lang="ts">
  interface Props {
    speed?: string;
    active?: number;
    queued?: number;
    completed?: number;
  }

  let {
    speed = "0 B/s",
    active = 0,
    queued = 0,
    completed = 0,
  }: Props = $props();

  const cards = $derived([
    { label: "Tốc độ", value: speed, icon: "speed" },
    { label: "Đang chạy", value: String(active), icon: "play_circle" },
    { label: "Hàng đợi", value: String(queued), icon: "playlist_add_check" },
    { label: "Hoàn tất", value: String(completed), icon: "task_alt" },
  ]);
</script>

<section class="fhub-status-deck" aria-label="FHUB status deck">
  {#each cards as card}
    <article class="status-card">
      <span class="material-icons" aria-hidden="true">{card.icon}</span>
      <div>
        <small>{card.label}</small>
        <strong>{card.value}</strong>
      </div>
    </article>
  {/each}
</section>

<style>
  .fhub-status-deck {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.85rem;
  }

  .status-card {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    min-height: 86px;
    padding: 1rem;
    border: 1px solid rgba(245, 184, 92, 0.15);
    border-radius: 22px;
    background: linear-gradient(180deg, rgba(29, 18, 13, 0.86), rgba(7, 5, 4, 0.74));
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.32), inset 0 1px 0 rgba(255, 255, 255, 0.045);
  }

  .material-icons {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 14px;
    background: rgba(245, 184, 92, 0.12);
    color: #f5b85c;
  }

  small,
  strong {
    display: block;
  }

  small {
    color: #c7b5a2;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  strong {
    margin-top: 0.15rem;
    color: #fff8ee;
    font-size: 1.08rem;
    letter-spacing: -0.03em;
  }

  @media (max-width: 920px) {
    .fhub-status-deck {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .fhub-status-deck {
      grid-template-columns: 1fr;
    }
  }
</style>
