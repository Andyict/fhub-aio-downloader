<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
    title?: string;
    description?: string;
    eyebrow?: string;
  }

  let {
    children,
    title = "Đăng nhập FHUB",
    description = "Mở khóa workspace điện ảnh và quản lý hệ thống media trong một giao diện FHUB-native.",
    eyebrow = "FHUB ACCESS",
  }: Props = $props();
</script>

<section class="fhub-auth-shell" aria-label={title}>
  <div class="auth-backdrop" aria-hidden="true"></div>
  <article class="auth-card">
    {#if eyebrow || title || description}
      <header>
        {#if eyebrow}<span>{eyebrow}</span>{/if}
        {#if title}<h1>{title}</h1>{/if}
        {#if description}<p>{description}</p>{/if}
      </header>
    {/if}

    <div class="auth-content">
      {@render children()}
    </div>
  </article>
</section>

<style>
  .fhub-auth-shell {
    position: relative;
    min-height: 100svh;
    display: grid;
    place-items: center;
    overflow: auto;
    padding: clamp(0.85rem, 3vw, 1.4rem);
    background: #070a12;
  }

  .auth-backdrop {
    position: absolute;
    inset: 0;
    pointer-events: none;
    display: none;
  }

  .auth-card {
    position: relative;
    z-index: 1;
    width: min(100%, 430px);
    display: grid;
    gap: 0.72rem;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    backdrop-filter: none;
  }

  header {
    display: grid;
    gap: 0.16rem;
  }

  span {
    color: #f5b85c;
    font-size: 0.64rem;
    font-weight: 950;
    letter-spacing: 0.2em;
    text-transform: uppercase;
  }

  h1,
  p {
    margin: 0;
  }

  h1 {
    color: #fff8ee;
    font-size: clamp(1.75rem, 7vw, 2.8rem);
    line-height: 0.95;
    letter-spacing: -0.075em;
  }

  p {
    color: #c7b5a2;
    line-height: 1.35;
  }

  .auth-content {
    min-width: 0;
  }
</style>
