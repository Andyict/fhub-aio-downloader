<script lang="ts">
  export let title = "FHub Cinema Hub";
  export let subtitle = "Trung tâm quản lý media cho NAS";
  export let eyebrow = "FHUB NATIVE";
  export let primaryLabel = "Mở hàng đợi";
  export let secondaryLabel = "Cài đặt";
</script>

<section class="fhub-hero-panel" aria-label="FHUB hero panel">
  <div class="hero-glow" aria-hidden="true"></div>
  <div class="hero-content">
    <span>{eyebrow}</span>
    <h1>{title}</h1>
    <p>{subtitle}</p>
    <div class="hero-actions">
      <slot name="primary">
        <button type="button" class="primary-action">{primaryLabel}</button>
      </slot>
      <slot name="secondary">
        <button type="button" class="secondary-action">{secondaryLabel}</button>
      </slot>
    </div>
  </div>
  <div class="hero-watermark" aria-hidden="true">FHUB</div>
</section>

<style>
  .fhub-hero-panel {
    position: relative;
    min-height: 240px;
    overflow: hidden;
    display: grid;
    align-items: end;
    padding: clamp(1.15rem, 3vw, 2rem);
    border: 1px solid rgba(245, 184, 92, 0.18);
    border-radius: 30px;
    background:
      linear-gradient(110deg, rgba(7, 5, 4, 0.92), rgba(32, 18, 12, 0.72)),
      radial-gradient(circle at 15% 10%, rgba(245, 184, 92, 0.18), transparent 28%),
      radial-gradient(circle at 92% 25%, rgba(225, 29, 72, 0.10), transparent 24%);
    box-shadow: 0 32px 90px rgba(0, 0, 0, 0.44), inset 0 1px 0 rgba(255, 255, 255, 0.055);
  }

  .hero-glow {
    position: absolute;
    inset: -30%;
    pointer-events: none;
    background: conic-gradient(from 120deg at 50% 50%, transparent, rgba(245, 184, 92, 0.18), transparent 38%);
    filter: blur(32px);
    opacity: 0.72;
  }

  .hero-content {
    position: relative;
    z-index: 1;
    max-width: 680px;
  }

  span {
    color: #f5b85c;
    font-size: 0.7rem;
    font-weight: 950;
    letter-spacing: 0.28em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0.35rem 0 0;
    color: #fff8ee;
    font-size: clamp(2.1rem, 6vw, 5.2rem);
    line-height: 0.92;
    letter-spacing: -0.08em;
  }

  p {
    max-width: 560px;
    margin: 0.8rem 0 0;
    color: #c7b5a2;
    line-height: 1.55;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.7rem;
    margin-top: 1.25rem;
  }

  button {
    min-height: 46px;
    border-radius: 999px;
    cursor: pointer;
    font-weight: 900;
    padding: 0 1.1rem;
  }

  .primary-action {
    border: 1px solid rgba(255, 214, 164, 0.34);
    background: linear-gradient(135deg, #f5b85c, #c2410c);
    color: #160b05;
  }

  .secondary-action {
    border: 1px solid rgba(255, 214, 164, 0.16);
    background: rgba(255, 247, 237, 0.06);
    color: #ffe8c2;
  }

  .hero-watermark {
    position: absolute;
    right: 1rem;
    bottom: 0.45rem;
    color: rgba(255, 232, 194, 0.12);
    font-size: clamp(3rem, 14vw, 11rem);
    font-weight: 950;
    letter-spacing: -0.12em;
    line-height: 0.8;
    pointer-events: none;
  }
</style>
