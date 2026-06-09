<script lang="ts">
  import type { Snippet } from "svelte";
  import FhubPanel from "./FhubPanel.svelte";

  interface Props {
    children: Snippet;
    title?: string;
    description?: string;
  }

  let {
    children,
    title = "Cấu hình FHUB",
    description = "Quản lý các lựa chọn vận hành trong một không gian native của FHUB.",
  }: Props = $props();
</script>

<section class="fhub-settings-shell" aria-label={title}>
  <header class="settings-hero">
    <span>FHUB CONTROL ROOM</span>
    <h2>{title}</h2>
    <p>{description}</p>
  </header>

  <FhubPanel
    eyebrow="FHUB SETTINGS"
    title="Không gian cấu hình"
    description="Các khối cấu hình có thể được ghép dần vào shell này mà không phụ thuộc vào layout cũ."
  >
    {@render children()}
  </FhubPanel>
</section>

<style>
  .fhub-settings-shell {
    display: grid;
    gap: 1rem;
    min-width: 0;
  }

  .settings-hero {
    display: grid;
    gap: 0.28rem;
    padding: clamp(1rem, 2vw, 1.35rem);
    border: 1px solid rgba(245, 184, 92, 0.15);
    border-radius: 26px;
    background:
      radial-gradient(circle at 0% 0%, rgba(245, 184, 92, 0.14), transparent 32%),
      linear-gradient(135deg, rgba(29, 18, 13, 0.84), rgba(7, 5, 4, 0.70));
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.30), inset 0 1px 0 rgba(255, 255, 255, 0.045);
  }

  span {
    color: #f5b85c;
    font-size: 0.68rem;
    font-weight: 950;
    letter-spacing: 0.24em;
    text-transform: uppercase;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: #fff8ee;
    font-size: clamp(1.45rem, 4vw, 2.6rem);
    letter-spacing: -0.06em;
  }

  p {
    max-width: 620px;
    color: #c7b5a2;
    line-height: 1.55;
  }
</style>
