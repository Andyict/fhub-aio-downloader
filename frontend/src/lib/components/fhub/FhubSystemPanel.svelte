<script lang="ts">
  interface Props {
    title?: string;
    subtitle?: string;
    connection?: string;
    storage?: string;
    account?: string;
  }

  let {
    title = "Trạng thái hệ thống",
    subtitle = "Theo dõi các tín hiệu vận hành quan trọng của FHUB.",
    connection = "Sẵn sàng",
    storage = "Đang theo dõi",
    account = "Đã cấu hình",
  }: Props = $props();

  const signals = $derived([
    { label: "Kết nối", value: connection, icon: "hub" },
    { label: "Lưu trữ", value: storage, icon: "dns" },
    { label: "Tài khoản", value: account, icon: "verified_user" },
  ]);
</script>

<section class="fhub-system-panel" aria-label={title}>
  <header>
    <span>FHUB SYSTEM</span>
    <h3>{title}</h3>
    <p>{subtitle}</p>
  </header>

  <div class="signal-grid">
    {#each signals as signal}
      <article class="signal-card">
        <span class="material-icons" aria-hidden="true">{signal.icon}</span>
        <div>
          <small>{signal.label}</small>
          <strong>{signal.value}</strong>
        </div>
      </article>
    {/each}
  </div>
</section>

<style>
  .fhub-system-panel {
    display: grid;
    gap: 1rem;
  }

  header {
    display: grid;
    gap: 0.25rem;
  }

  header span {
    color: #f5b85c;
    font-size: 0.66rem;
    font-weight: 950;
    letter-spacing: 0.22em;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    color: #fff8ee;
    font-size: 1.2rem;
    letter-spacing: -0.04em;
  }

  p {
    color: #c7b5a2;
    line-height: 1.5;
  }

  .signal-grid {
    display: grid;
    gap: 0.7rem;
  }

  .signal-card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.85rem;
    border: 1px solid rgba(255, 214, 164, 0.12);
    border-radius: 18px;
    background: rgba(255, 247, 237, 0.04);
  }

  .material-icons {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: rgba(245, 184, 92, 0.11);
    color: #f5b85c;
  }

  small,
  strong {
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
    font-size: 0.92rem;
  }
</style>
