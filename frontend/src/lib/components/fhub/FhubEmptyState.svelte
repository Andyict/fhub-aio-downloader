<script lang="ts">
  export let icon = "movie_filter";
  export let title = "Chưa có dữ liệu";
  export let message = "FHUB sẽ hiển thị nội dung tại đây khi có dữ liệu mới.";
  export let actionLabel = "Làm mới";
  export let showAction = false;
</script>

<section class="fhub-empty-state" aria-label={title}>
  <div class="empty-icon">
    <span class="material-icons" aria-hidden="true">{icon}</span>
  </div>
  <h3>{title}</h3>
  <p>{message}</p>
  {#if showAction}
    <button type="button">
      {actionLabel}
    </button>
  {/if}
</section>

<style>
  .fhub-empty-state {
    display: grid;
    place-items: center;
    gap: 0.65rem;
    min-height: 240px;
    padding: 1.5rem;
    text-align: center;
    border: 1px dashed rgba(245, 184, 92, 0.22);
    border-radius: 26px;
    background:
      radial-gradient(circle at 50% 0%, rgba(245, 184, 92, 0.10), transparent 34%),
      rgba(255, 247, 237, 0.035);
  }

  .empty-icon {
    width: 64px;
    height: 64px;
    display: grid;
    place-items: center;
    border-radius: 22px;
    background: rgba(245, 184, 92, 0.12);
    color: #f5b85c;
  }

  .material-icons {
    font-size: 2rem;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    color: #fff8ee;
    font-size: 1.12rem;
    letter-spacing: -0.03em;
  }

  p {
    max-width: 440px;
    color: #c7b5a2;
    line-height: 1.55;
  }

  button {
    min-height: 42px;
    border: 1px solid rgba(255, 214, 164, 0.18);
    border-radius: 999px;
    background: rgba(255, 247, 237, 0.06);
    color: #ffe8c2;
    cursor: pointer;
    font-weight: 850;
    padding: 0 1rem;
  }
</style>
