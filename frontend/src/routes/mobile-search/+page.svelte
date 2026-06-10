<script lang="ts">
  import { onMount, tick } from "svelte";

  let value = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);
  let embedded = $state(false);

  const fsharePattern = /https?:\/\/(www\.)?fshare\.vn\/(file|folder)\//i;

  onMount(() => {
    const params = new URLSearchParams(window.location.search);
    embedded = params.get("embed") === "1";
    value = params.get("q") || params.get("url") || "";
    void tick().then(() => inputEl?.focus());
  });

  function submit() {
    const clean = value.trim();
    if (!clean) return;
    const target = fsharePattern.test(clean)
      ? `/downloads?url=${encodeURIComponent(clean)}`
      : `/discover?q=${encodeURIComponent(clean)}`;
    if (embedded && window.parent) window.parent.location.href = target;
    else window.location.href = target;
  }
</script>

<svelte:head>
  <title>Nhập tìm kiếm mobile · FHub</title>
</svelte:head>

<main class:embedded class="mobile-search-page">
  <section class="card">
    {#if !embedded}
      <div class="head">
        <button type="button" onclick={() => history.back()} aria-label="Quay lại">
          <span class="material-icons">arrow_back</span>
        </button>
        <div>
          <p>FHub mobile</p>
          <h1>Dán link hoặc tìm phim</h1>
        </div>
      </div>
    {/if}

    <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
      <label for="mobile-search-input">Link FShare hoặc tên phim</label>
      <input
        id="mobile-search-input"
        bind:this={inputEl}
        bind:value
        type="text"
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck="false"
        inputmode="text"
        enterkeyhint="search"
        placeholder="Dán link FShare hoặc nhập tên phim..."
      />
      <button type="submit" disabled={!value.trim()}>
        <span class="material-icons">search</span>
        <span>Tiếp tục</span>
      </button>
    </form>

    {#if !embedded}
      <p class="hint">Link FShare sẽ mở tab Downloads. Tên phim sẽ quay về Discover để tìm.</p>
    {/if}
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    overscroll-behavior: auto !important;
    -webkit-user-select: text !important;
    user-select: text !important;
  }
  .mobile-search-page {
    min-height: 100vh;
    min-height: 100dvh;
    display: grid;
    place-items: start center;
    padding: max(20px, env(safe-area-inset-top, 0px)) 14px max(24px, env(safe-area-inset-bottom, 0px));
    background: #050816;
    color: #f8fafc;
  }
  .mobile-search-page.embedded {
    min-height: 82px;
    display: block;
    padding: 0;
    background: transparent;
  }
  .card {
    width: min(100%, 520px);
    margin-top: 8px;
    padding: 16px;
    border: 1px solid rgba(148, 163, 184, .18);
    border-radius: 24px;
    background: #0b1220;
    box-shadow: 0 18px 44px rgba(0,0,0,.45);
  }
  .embedded .card {
    width: 100%;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
  }
  .head {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    margin-bottom: 18px;
  }
  .head button {
    width: 48px;
    height: 48px;
    border: 1px solid rgba(148, 163, 184, .18);
    border-radius: 16px;
    color: #f8fafc;
    background: rgba(15, 23, 42, .92);
  }
  .head p {
    margin: 0 0 4px;
    color: #f8c14a;
    font-size: 12px;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: .12em;
  }
  .head h1 {
    margin: 0;
    font-size: 22px;
    line-height: 1.1;
    letter-spacing: -.04em;
  }
  form {
    display: grid;
    gap: 10px;
  }
  .embedded form {
    grid-template-columns: minmax(0, 1fr) 64px;
    gap: 10px;
  }
  label {
    color: rgba(226, 232, 240, .72);
    font-size: 13px;
    font-weight: 800;
  }
  .embedded label { display: none; }
  input {
    width: 100%;
    min-width: 0;
    height: 58px;
    box-sizing: border-box;
    padding: 0 14px;
    border: 1px solid rgba(248, 193, 74, .28);
    border-radius: 18px;
    outline: 0;
    color: #f8fafc;
    background: #050816;
    font-size: 16px;
    font-weight: 800;
    line-height: 58px;
    appearance: none;
    -webkit-appearance: none;
    -webkit-user-select: text;
    user-select: text;
    pointer-events: auto;
    touch-action: auto;
  }
  .embedded input {
    height: 76px;
    padding: 0 18px;
    border-radius: 22px;
    background: #080d18;
    font-size: 16px;
    font-weight: 900;
    line-height: 76px;
  }
  input:focus {
    border-color: rgba(248, 193, 74, .72);
    box-shadow: 0 0 0 4px rgba(248, 193, 74, .12);
  }
  input::placeholder {
    color: rgba(226, 232, 240, .42);
  }
  form button[type="submit"] {
    height: 56px;
    border: 0;
    border-radius: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #090d18;
    background: linear-gradient(135deg, #f8c14a, #a78bfa);
    font-size: 16px;
    font-weight: 950;
  }
  .embedded form button[type="submit"] {
    width: 64px;
    height: 76px;
    padding: 0;
    border-radius: 22px;
  }
  .embedded form button[type="submit"] span:last-child { display: none; }
  form button[type="submit"]:disabled {
    opacity: .45;
  }
  .hint {
    margin: 14px 2px 0;
    color: rgba(226, 232, 240, .58);
    font-size: 13px;
    line-height: 1.45;
  }
</style>
