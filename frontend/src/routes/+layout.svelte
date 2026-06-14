<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { accountStore } from "$lib/stores/account.svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  type UiLanguage = "vi" | "en";
  type UiMode = "modern" | "classic";

  const navLabels: Record<UiLanguage, Array<{ href: string; label: string; icon: string }>> = {
    vi: [
      { href: "/discover", label: "Khám phá", icon: "explore" },
      { href: "/downloads", label: "Tải xuống", icon: "download" },
      { href: "/auto-track", label: "Auto Track", icon: "sync" },
      { href: "/favorites", label: "Đã lưu", icon: "bookmark" },
      { href: "/settings", label: "Cài đặt", icon: "settings" },
    ],
    en: [
      { href: "/discover", label: "Discovery", icon: "explore" },
      { href: "/downloads", label: "Download", icon: "download" },
      { href: "/auto-track", label: "Auto Track", icon: "sync" },
      { href: "/favorites", label: "Bookmark", icon: "bookmark" },
      { href: "/settings", label: "Setting", icon: "settings" },
    ],
  };

  const routeInfo: Record<UiLanguage, Record<string, { title: string; subtitle: string; icon: string }>> = {
    vi: {
      "/": { title: "Bảng điều khiển", subtitle: "Hub media nội bộ", icon: "dashboard" },
      "/discover": { title: "Khám phá", subtitle: "Tìm phim, series và link FShare", icon: "explore" },
      "/favorites": { title: "Đã lưu", subtitle: "Nội dung đã bookmark", icon: "bookmark" },
      "/history": { title: "Lịch sử", subtitle: "Lịch sử xem, tải và tìm kiếm", icon: "history" },
      "/downloads": { title: "Tải xuống", subtitle: "Hàng đợi tải và tiến trình", icon: "download_for_offline" },
      "/auto-track": { title: "Auto Track", subtitle: "Theo dõi phim bộ và tự tải tập mới", icon: "sync" },
      "/settings": { title: "Cài đặt", subtitle: "Cấu hình và tài khoản", icon: "admin_panel_settings" },
    },
    en: {
      "/": { title: "Media Dashboard", subtitle: "Internal media hub", icon: "dashboard" },
      "/discover": { title: "Discovery", subtitle: "Find movies, series and FShare links", icon: "explore" },
      "/favorites": { title: "Bookmark", subtitle: "Saved content", icon: "bookmark" },
      "/history": { title: "History", subtitle: "Watch, download and search history", icon: "history" },
      "/downloads": { title: "Downloads", subtitle: "Queue and progress", icon: "download_for_offline" },
      "/auto-track": { title: "Auto Track", subtitle: "Track TV folders and download new episodes", icon: "sync" },
      "/settings": { title: "Settings", subtitle: "Config and accounts", icon: "admin_panel_settings" },
    },
  };

  let uiLanguage = $state<UiLanguage>("vi");
  let uiMode = $state<UiMode>("modern");
  let currentRoute = $derived($page.url.pathname);
  let isAuthRoute = $derived(currentRoute.startsWith("/login") || currentRoute.startsWith("/setup"));
  let currentInfo = $derived(routeInfo[uiLanguage][currentRoute] ?? routeInfo[uiLanguage]["/"]);
  let primaryFshare = $derived(accountStore.primaryFormatted);
  let isFshareVip = $derived(accountStore.isVip);
  let hasFshareAccount = $derived(!!accountStore.primary);
  let isFshareUnverified = $derived(!!accountStore.primary && !accountStore.isVip && ["", "UNVERIFIED"].includes((accountStore.primary.rank || "").toUpperCase().trim()));
  let showFshareLogin = $state(false);
  let sidebarCollapsed = $state(false);
  let fshareEmail = $state("");
  let fsharePassword = $state("");
  let showFsharePassword = $state(false);
  let fshareLoginLoading = $state(false);
  let fshareLoginMessage = $state("");
  let downloadSpeed = $state("0 B/s");
  let downloadState = $state("Queue idle");
  let hasDownloadActivity = $state(false);
  let currentUserRole = $state<"admin" | "user" | null>(null);
  let currentUsername = $state("");
  let authChecked = $state(false);
  let isAdminUser = $derived(currentUserRole === "admin");
  let navItems = $derived(navLabels[uiLanguage].filter((item) => {
    if (!isAdminUser && item.href === "/settings") return false;
    if (uiMode === "classic") return item.href === "/downloads" || (isAdminUser && item.href === "/settings");
    return true;
  }));
  let bottomNavItems = $derived(navItems);
  let languageFlag = $derived(uiLanguage === "vi" ? "🇻🇳" : "🇺🇸");
  let nextLanguageLabel = $derived(uiLanguage === "vi" ? "Switch to English" : "Chuyển sang tiếng Việt");

  onMount(() => {
    syncLanguage();
    syncUiMode();
    syncSidebarPreference();
    const onUiModeChange = (event: Event) => {
      const next = (event as CustomEvent<UiMode>).detail;
      if (next === "modern" || next === "classic") uiMode = next;
    };
    window.addEventListener("fhub-ui-mode-change", onUiModeChange);
    void bootAuth();
    void syncFshareAccount();
    void refreshTopStats();
    const statsTimer = window.setInterval(refreshTopStats, 10000);
    if ($page.url.searchParams.get("fshare") === "login") {
      openFshareLogin();
    }
    return () => {
      window.clearInterval(statsTimer);
      window.removeEventListener("fhub-ui-mode-change", onUiModeChange);
    };
  });

  async function bootAuth() {
    if (isAuthRoute) {
      authChecked = true;
      return;
    }
    const setupOk = await enforceFirstRunSetup();
    if (!setupOk) return;
    await loadCurrentUser();
  }

  async function enforceFirstRunSetup() {
    if (isAuthRoute) return true;
    try {
      const response = await fetch("/api/auth/setup-status", { credentials: "include" });
      const status = response.ok ? await response.json() : { setup_required: true };
      if (status.setup_required) {
        authChecked = true;
        await goto("/setup", { replaceState: true });
        return false;
      }
      return true;
    } catch {
      authChecked = true;
      await goto("/setup", { replaceState: true });
      return false;
    }
  }

  async function loadCurrentUser() {
    if (isAuthRoute) return;
    try {
      const response = await fetch("/api/auth/me", { credentials: "include" });
      if (!response.ok) {
        currentUserRole = null;
        currentUsername = "";
        await goto("/login", { replaceState: true });
        return;
      }
      const payload = await response.json();
      if (!payload?.authenticated && !payload?.user) {
        currentUserRole = null;
        currentUsername = "";
        await goto("/login", { replaceState: true });
        return;
      }
      currentUserRole = payload?.user?.role === "admin" ? "admin" : "user";
      currentUsername = payload?.user?.username || "";
    } catch {
      currentUserRole = null;
      currentUsername = "";
      await goto("/login", { replaceState: true });
    } finally {
      authChecked = true;
    }
  }

  function openFshareLogin() {
    fshareEmail = accountStore.primary?.email || fshareEmail;
    fsharePassword = "";
    fshareLoginMessage = "";
    showFshareLogin = true;
  }

  function closeFshareLogin() {
    if (fshareLoginLoading) return;
    showFshareLogin = false;
    fsharePassword = "";
  }

  async function loginFshare() {
    if (!fshareEmail.trim() || !fsharePassword.trim()) {
      fshareLoginMessage = "Nhập email/username và mật khẩu FShare trước đã.";
      return;
    }

    fshareLoginLoading = true;
    fshareLoginMessage = "Đang đăng nhập FShare...";
    try {
      const result = await accountStore.switchAccount(fshareEmail.trim(), fsharePassword);
      if (!result || result.success === false) {
        throw new Error(result?.message || result?.error || "Đăng nhập FShare thất bại. Kiểm tra lại tài khoản hoặc phiên đăng nhập FHUB.");
      }
      await accountStore.fetch();
      fsharePassword = "";
      fshareLoginMessage = result.message || "Đã lưu tài khoản FShare.";
    } catch (error) {
      fshareLoginMessage = error instanceof Error ? error.message : "Đăng nhập FShare thất bại.";
    } finally {
      fshareLoginLoading = false;
    }
  }

  async function syncFshareAccount() {
    await accountStore.fetch();
    // Không tự retry FShare liên tục ở topbar: FShare có cơ chế khóa 10 phút
    // nếu sai mật khẩu nhiều lần. Nếu rank UNVERIFIED, để người dùng refresh lại trong Settings.

  }

  async function refreshTopStats() {
    if (isAuthRoute) return;
    try {
      const response = await fetch("/api/engine/stats", { credentials: "include" });
      if (!response.ok) return;
      const stats = await response.json();
      const speed = Number(stats.total_speed ?? 0);
      const active = Number(stats.active_downloads ?? 0);
      const queued = Number(stats.queued ?? 0);
      hasDownloadActivity = active > 0 || queued > 0 || speed > 0;
      downloadSpeed = `${formatBytes(speed)}/s`;
      downloadState = active > 0 ? `${active} đang tải` : queued > 0 ? `${queued} đang chờ` : "Queue idle";
    } catch {
      hasDownloadActivity = false;
      downloadSpeed = "0 B/s";
      downloadState = "Offline";
    }
  }

  function formatBytes(value: number) {
    if (!Number.isFinite(value) || value <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = value;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size >= 10 || unit === 0 ? size.toFixed(0) : size.toFixed(1)} ${units[unit]}`;
  }

  function syncLanguage() {
    try {
      const saved = localStorage.getItem("fhub-ui-language");
      if (saved === "vi" || saved === "en") uiLanguage = saved;
      document.documentElement.lang = uiLanguage;
    } catch {
      // localStorage can be unavailable in restricted contexts.
    }
  }

  function syncSidebarPreference() {
    try {
      sidebarCollapsed = localStorage.getItem("fhub-sidebar-collapsed") === "true";
    } catch {
      // Ignore storage failures.
    }
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    try {
      localStorage.setItem("fhub-sidebar-collapsed", String(sidebarCollapsed));
    } catch {
      // Ignore storage failures.
    }
  }

  function syncUiMode() {
    try {
      const saved = localStorage.getItem("fhub-ui-mode");
      if (saved === "modern" || saved === "classic") uiMode = saved;
    } catch {
      // Ignore storage failures.
    }
  }

  $effect(() => {
    if (!authChecked || isAuthRoute || uiMode !== "classic") return;
    if (currentRoute !== "/downloads" && currentRoute !== "/settings") {
      void goto("/downloads", { replaceState: true });
    }
  });

  function toggleLanguage() {
    uiLanguage = uiLanguage === "vi" ? "en" : "vi";
    try {
      localStorage.setItem("fhub-ui-language", uiLanguage);
      document.documentElement.lang = uiLanguage;
    } catch {
      // Ignore storage failures; current in-memory UI still changes.
    }
    window.dispatchEvent(new CustomEvent("fhub-language-change", { detail: uiLanguage }));
  }

  function isActive(href: string) {
    if (href === "/") return currentRoute === "/";
    return currentRoute.startsWith(href);
  }
</script>

<svelte:head>
  <title>FHUB</title>
  <meta name="theme-color" content="#080a12" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap" rel="stylesheet" />
  <link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet" />
</svelte:head>

{#if isAuthRoute}
  {@render children()}
{:else if !authChecked}
  <div class="auth-gate" aria-label="Đang kiểm tra đăng nhập">
    <div class="auth-sync-card">
      <img src="/images/fhub-icon.clean.png" alt="FHUB" />
      <span class="sync-ring" aria-hidden="true"></span>
      <strong>{uiLanguage === "vi" ? "Đang đồng bộ tài khoản" : "Syncing account"}</strong>
      <small>{uiLanguage === "vi" ? "Đang kiểm tra quyền admin/user..." : "Checking admin/user role..."}</small>
    </div>
  </div>
{:else}
  <div class="fhub-shell" class:classic-mode={uiMode === "classic"} class:sidebar-collapsed={sidebarCollapsed}>
    <aside class="fhub-sidebar" aria-label="FHUB navigation">
      <div class="sidebar-head">
        <a class="fhub-brand" href="/" aria-label="FHUB home">
          <img src="/images/logo5.png" alt="FHUB" />
          <span>FShare & Torrent tốc độ cao</span>
        </a>
        <button class="sidebar-toggle" type="button" aria-label={sidebarCollapsed ? "Mở rộng thanh tab" : "Thu gọn thanh tab"} title={sidebarCollapsed ? "Mở rộng" : "Thu gọn"} onclick={toggleSidebar}>
          <span class="material-icons" aria-hidden="true">{sidebarCollapsed ? "chevron_right" : "chevron_left"}</span>
        </button>
      </div>

      <nav class="fhub-nav">
        {#each navItems as item, index}
          {#if index === 0}
            <small class="nav-group">{uiMode === "classic" ? "Classic" : "Điều hướng"}</small>
          {:else if item.href === "/downloads"}
            <small class="nav-group">{uiMode === "classic" ? "Admin" : "Quản trị"}</small>
          {/if}
          <a href={item.href} class:active={isActive(item.href)}>
            <span class="material-icons" aria-hidden="true">{item.icon}</span>
            <span>{item.label}</span>
          </a>
        {/each}
      </nav>

      <a class="sign-out-panel" href="/login">
        <span class="material-icons">logout</span>
        <div>
          <strong>Sign out</strong>
          <small>Thoát tài khoản admin</small>
        </div>
      </a>
    </aside>

    <main class="fhub-main">
      <header class="fhub-topbar">
        <a class="fhub-mobile-brand" href="/" aria-label="FShare Hub home">
          <img src="/images/fhub-icon.clean.png" alt="FShare Hub" />
        </a>
        <div class="fhub-route-context">
          <span class="material-icons">{currentInfo.icon}</span>
          <div>
            <strong>{currentInfo.title}</strong>
            <small>{currentInfo.subtitle}</small>
          </div>
        </div>
        {#if uiMode !== "classic"}
        <form class="top-search" action="/discover" onsubmit={async (event) => {
          const form = event.currentTarget as HTMLFormElement;
          const input = form.elements.namedItem("q") as HTMLInputElement | null;
          const value = input?.value.trim();
          event.preventDefault();
          if (!value) return;
          const isFshare = /https?:\/\/(www\.)?fshare\.vn\/(file|folder)\//i.test(value);
          if (isFshare) {
            await goto(`/downloads?url=${encodeURIComponent(value)}`, { replaceState: false, invalidateAll: true, noScroll: true });
            return;
          }
          await goto(`/discover?q=${encodeURIComponent(value)}`, { replaceState: false, invalidateAll: true, noScroll: true });
        }}>
          <span class="material-icons">search</span>
          <input name="q" placeholder="Tìm phim, link FShare..." />
          <button type="submit" aria-label="Tìm kiếm"><span class="material-icons">arrow_forward</span></button>
        </form>
        {/if}
        <div class="fhub-top-actions">
          {#if hasDownloadActivity}
            <span class="top-status download-speed">
              <span class="material-icons">download_for_offline</span>
              <span><strong>{downloadSpeed}</strong><small>{downloadState}</small></span>
            </span>
          {/if}
          {#if isFshareVip}
            <a class="top-status fshare-vip" href={isAdminUser ? "/settings?tab=accounts" : "/downloads"} title={`FShare VIP: ${primaryFshare.email}`}>
              <span class="vip-logo">VIP</span>
              <span><strong>FShare</strong><small>{primaryFshare.rank}</small></span>
            </a>
          {:else if hasFshareAccount}
            <a class="top-status fshare-pending" href={isAdminUser ? "/settings?tab=accounts" : "/downloads"} title={`FShare: ${primaryFshare.email}`}>
              <span class="material-icons">sync</span>
              <span><strong>FShare</strong><small>{isFshareUnverified ? "Đang kiểm tra" : primaryFshare.rank}</small></span>
            </a>
          {:else}
            <button class="top-status fshare-login" type="button" title="Đăng nhập FShare" onclick={openFshareLogin}>
              <span class="material-icons">login</span>
              <span><strong>FShare</strong><small>Login</small></span>
            </button>
          {/if}
          <button class="lang-switch" type="button" aria-label={nextLanguageLabel} title={nextLanguageLabel} onclick={toggleLanguage}>
            <span class="lang-flag" aria-hidden="true">{languageFlag}</span>
            <span class="lang-code">{uiLanguage.toUpperCase()}</span>
          </button>
          {#if currentUsername}
            <a class="top-pill account" href={isAdminUser ? "/settings" : "/downloads"}>{currentUsername}</a>
          {/if}
        </div>
      </header>

      <section class="fhub-content">
        {@render children()}
      </section>
    </main>

    <nav class="fhub-bottom-nav" aria-label="FHUB mobile navigation">
      {#each bottomNavItems as item}
        <a href={item.href} class:active={isActive(item.href)}>
          <span class="material-icons" aria-hidden="true">{item.icon}</span>
          <span>{item.label}</span>
        </a>
      {/each}
    </nav>

    {#if showFshareLogin}
      <div class="fshare-modal-backdrop" role="button" tabindex="0" aria-label="Đóng popup đăng nhập FShare" onclick={closeFshareLogin} onkeydown={(event) => { if (event.key === "Enter" || event.key === " " || event.key === "Escape") closeFshareLogin(); }}>
        <section class="fshare-login-modal" role="dialog" aria-modal="true" aria-labelledby="fshare-login-title" onclick={(event) => event.stopPropagation()}>
          <button class="modal-close" type="button" aria-label="Đóng" onclick={closeFshareLogin} disabled={fshareLoginLoading}>
            <span class="material-icons">close</span>
          </button>
          <div class="modal-orb"><span class="material-icons">cloud_sync</span></div>
          <div class="modal-copy">
            <span class="eyebrow">FShare Login</span>
            <h2 id="fshare-login-title">Đăng nhập tài khoản FShare</h2>
            <p>FHUB sẽ lưu tài khoản trên NAS để kiểm tra VIP và tải link FShare trực tiếp.</p>
          </div>
          <form class="modal-form" onsubmit={(event) => { event.preventDefault(); loginFshare(); }}>
            <label>
              <span>Email / username</span>
              <div class="modal-input">
                <span class="material-icons">alternate_email</span>
                <input bind:value={fshareEmail} autocomplete="username" placeholder="email@fshare.vn" />
              </div>
            </label>
            <label>
              <span>Mật khẩu</span>
              <div class="modal-input password-input">
                <span class="material-icons">lock</span>
                <input type={showFsharePassword ? "text" : "password"} bind:value={fsharePassword} autocomplete="current-password" placeholder="Nhập mật khẩu FShare" />
                <button type="button" class="password-eye" aria-label={showFsharePassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showFsharePassword = !showFsharePassword}>
                  <span class="material-icons">{showFsharePassword ? "visibility_off" : "visibility"}</span>
                </button>
              </div>
            </label>
            {#if fshareLoginMessage}<p class="modal-message">{fshareLoginMessage}</p>{/if}
            <button class="modal-submit" type="submit" disabled={fshareLoginLoading}>
              <span class="material-icons">login</span>
              {fshareLoginLoading ? "Đang đăng nhập..." : "Đăng nhập FShare"}
            </button>
          </form>
        </section>
      </div>
    {/if}
  </div>
{/if}

<style>
  :global(html),
  :global(body) {
    margin: 0;
    min-height: 100%;
    background: #080a12;
    color: #f8fafc;
    font-family: Inter, system-ui, sans-serif;
  }

  .auth-gate {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: radial-gradient(circle at 50% -8%, rgba(244,181,68,.16), transparent 32%), linear-gradient(180deg, #050505, #0d0906 52%, #050505);
  }

  .auth-sync-card {
    width: min(100%, 320px);
    display: grid;
    justify-items: center;
    gap: .58rem;
    padding: 1.25rem;
    border-radius: 26px;
    color: #fff8eb;
    background: linear-gradient(180deg, rgba(25,22,18,.92), rgba(8,8,8,.9));
    border: 1px solid rgba(244,181,68,.16);
    box-shadow: 0 26px 80px rgba(0,0,0,.54);
  }

  .auth-sync-card img {
    width: 58px;
    height: 58px;
    object-fit: contain;
    filter: drop-shadow(0 0 22px rgba(244,181,68,.25));
  }

  .auth-sync-card strong {
    margin-top: .15rem;
    font-size: 1rem;
    font-weight: 950;
  }

  .auth-sync-card small {
    color: rgba(255,248,235,.62);
    font-size: .78rem;
    font-weight: 750;
  }

  .sync-ring {
    width: 34px;
    height: 34px;
    border-radius: 999px;
    border: 3px solid rgba(244,181,68,.18);
    border-top-color: #f4b544;
    animation: sync-spin .72s linear infinite;
  }

  @keyframes sync-spin { to { transform: rotate(360deg); } }

  .fhub-shell {
    --fhub-sidebar-width: 276px;
    height: 100dvh;
    overflow: hidden;
    display: grid;
    grid-template-columns: var(--fhub-sidebar-width) minmax(0, 1fr);
    background:
      radial-gradient(circle at 18% 0%, rgba(248, 193, 74, 0.1), transparent 25%),
      linear-gradient(135deg, rgba(124, 58, 237, 0.16), transparent 34%),
      linear-gradient(180deg, #080a12 0%, #0d111c 48%, #05070d 100%);
  }

  .fhub-shell.sidebar-collapsed {
    --fhub-sidebar-width: 92px;
  }

  .fhub-shell::before {
    content: "";
    position: fixed;
    inset: 0;
    pointer-events: none;
    opacity: 0.16;
    background-image:
      linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
    background-size: 32px 32px;
  }

  .fhub-sidebar {
    position: relative;
    top: 0;
    z-index: 20;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
    padding: 1.35rem 1.2rem;
    border-right: 1px solid rgba(148, 163, 184, 0.14);
    background: rgba(7, 10, 18, 0.84);
    backdrop-filter: blur(24px) saturate(150%);
  }

  .sidebar-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 38px;
    align-items: start;
    gap: 0.6rem;
  }

  .sidebar-toggle {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border: 1px solid rgba(167, 139, 250, 0.2);
    border-radius: 13px;
    color: rgba(226, 232, 240, 0.78);
    background: rgba(255, 255, 255, 0.055);
    cursor: pointer;
  }

  .sidebar-toggle:hover {
    color: #fff;
    background: rgba(167, 139, 250, 0.16);
  }

  .fhub-brand,
  .fhub-mobile-brand {
    display: inline-grid;
    align-items: center;
    gap: 0.25rem;
    color: inherit;
    text-decoration: none;
  }

  .fhub-brand img {
    width: 190px;
    max-width: 100%;
    height: auto;
    object-fit: contain;
  }

  .fhub-mobile-brand {
    display: none;
    grid-template-columns: auto;
    gap: 0.55rem;
  }

  .fhub-mobile-brand img {
    width: 40px;
    height: 40px;
    object-fit: contain;
  }

  .sidebar-collapsed .sidebar-head {
    grid-template-columns: 1fr;
    justify-items: center;
  }

  .sidebar-collapsed .fhub-brand {
    justify-items: center;
  }

  .sidebar-collapsed .fhub-brand img {
    width: 48px;
    content: url("/images/fhub-icon.clean.png");
  }

  .sidebar-collapsed .fhub-brand span,
  .sidebar-collapsed .nav-group,
  .sidebar-collapsed .fhub-nav a span:not(.material-icons),
  .sidebar-collapsed .sign-out-panel div {
    display: none;
  }

  .sidebar-collapsed .fhub-sidebar {
    gap: 1rem;
    padding: 1.15rem 0.85rem;
  }

  .sidebar-collapsed .fhub-nav a {
    justify-content: center;
    padding: 0;
  }

  .sidebar-collapsed .sign-out-panel {
    grid-template-columns: 1fr;
    justify-items: center;
    padding: 0.62rem;
  }

  .fhub-brand span,
  .fhub-mobile-brand span {
    color: rgba(226, 232, 240, 0.58);
    font-size: 0.6rem;
    font-weight: 800;
    letter-spacing: 0.36em;
  }

  .fhub-nav {
    display: grid;
    gap: 0.48rem;
  }

  .nav-group {
    margin: 0.45rem 0 0.1rem;
    color: rgba(226, 232, 240, 0.42);
    font-size: 0.58rem;
    font-weight: 950;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .fhub-nav a,
  .fhub-bottom-nav a {
    display: flex;
    align-items: center;
    gap: 0.78rem;
    color: rgba(226, 232, 240, 0.68);
    text-decoration: none;
    font-size: 0.92rem;
    font-weight: 800;
  }

  .fhub-nav a {
    min-height: 46px;
    padding: 0 1.05rem;
    border-radius: 15px;
    border: 1px solid transparent;
  }

  .fhub-nav a.active,
  .fhub-nav a:hover {
    color: #f8fafc;
    background: linear-gradient(135deg, rgba(124, 58, 237, 0.32), rgba(36, 26, 64, 0.9));
    border-color: rgba(167, 139, 250, 0.38);
    box-shadow: 0 16px 42px rgba(0, 0, 0, 0.28);
  }

  .sign-out-panel {
    margin-top: auto;
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    min-height: 58px;
    padding: 0.78rem;
    border-radius: 16px;
    color: inherit;
    text-decoration: none;
    border: 1px solid rgba(248, 113, 113, 0.18);
    background: rgba(127, 29, 29, 0.12);
  }

  .sign-out-panel > .material-icons {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    color: #fecaca;
    background: rgba(248, 113, 113, 0.12);
  }

  .sign-out-panel strong,
  .sign-out-panel small {
    display: block;
  }

  .sign-out-panel strong {
    color: #fee2e2;
    font-size: 0.9rem;
  }

  .sign-out-panel small {
    margin-top: 0.15rem;
    color: rgba(254, 202, 202, 0.62);
    font-size: 0.72rem;
  }

  .fhub-main {
    min-width: 0;
    height: 100dvh;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
  }

  .fhub-topbar {
    position: sticky;
    top: 0;
    z-index: 15;
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 76px;
    padding: 0 1.5rem;
    background: rgba(7, 10, 18, 0.74);
    border-bottom: 1px solid rgba(148, 163, 184, 0.14);
    backdrop-filter: blur(22px) saturate(150%);
  }

  .fhub-route-context {
    display: inline-grid;
    grid-template-columns: 42px minmax(0, 1fr);
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
  }

  .top-search {
    flex: 1 1 360px;
    max-width: 520px;
    min-height: 44px;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    margin: 0 1rem;
    padding: 0 0.45rem 0 0.85rem;
    border-radius: 14px;
    border: 1px solid rgba(148, 163, 184, 0.14);
    background: rgba(255, 255, 255, 0.045);
  }

  .top-search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    color: #f8fafc;
    background: transparent;
    font-size: 0.86rem;
  }

  .top-search input::placeholder {
    color: rgba(226, 232, 240, 0.42);
  }

  .top-search button {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 11px;
    color: #080a12;
    background: linear-gradient(135deg, #f8c14a, #a78bfa);
  }

  .top-search button .material-icons {
    color: #080a12;
  }

  .fhub-route-context > .material-icons {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    color: #f8c14a;
    background: linear-gradient(135deg, rgba(248, 193, 74, 0.16), rgba(124, 58, 237, 0.13));
    border: 1px solid rgba(248, 193, 74, 0.14);
  }

  .fhub-route-context strong,
  .fhub-route-context small {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fhub-route-context strong {
    color: #ffffff;
    font-size: 0.96rem;
  }

  .fhub-route-context small {
    margin-top: 0.12rem;
    color: rgba(226, 232, 240, 0.56);
    font-size: 0.78rem;
  }

  .fhub-top-actions {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .top-status {
    display: inline-grid;
    grid-template-columns: 10px minmax(0, auto);
    align-items: center;
    gap: 0.55rem;
    min-height: 42px;
    padding: 0 0.78rem;
    border-radius: 14px;
    border: 1px solid rgba(148, 163, 184, 0.14);
    background: rgba(255, 255, 255, 0.045);
  }

  a.top-status {
    color: inherit;
    text-decoration: none;
  }

  .top-status.fshare-login {
    grid-template-columns: 18px minmax(0, auto);
    border-color: rgba(248, 193, 74, 0.22);
    background: linear-gradient(135deg, rgba(248, 193, 74, 0.12), rgba(124, 58, 237, 0.1));
  }

  .top-status.fshare-vip {
    grid-template-columns: 34px minmax(0, auto);
    border-color: rgba(248, 193, 74, 0.42);
    background:
      linear-gradient(135deg, rgba(248, 193, 74, 0.18), rgba(167, 139, 250, 0.13)),
      rgba(255, 255, 255, 0.05);
    box-shadow: 0 0 24px rgba(248, 193, 74, 0.1);
  }

  .vip-logo {
    width: 34px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    color: #080a12;
    font-size: 0.64rem;
    font-weight: 950;
    letter-spacing: 0.08em;
    background: linear-gradient(135deg, #f8c14a, #fef3c7 52%, #a78bfa);
    box-shadow: 0 0 18px rgba(248, 193, 74, 0.28);
  }

  .top-status > .material-icons {
    width: 18px;
    color: #f8c14a;
  }

  .top-status strong,
  .top-status small {
    display: block;
    line-height: 1.1;
    white-space: nowrap;
  }

  .top-status strong {
    color: #f8fafc;
    font-size: 0.78rem;
  }

  .top-status small {
    margin-top: 0.15rem;
    color: rgba(226, 232, 240, 0.5);
    font-size: 0.66rem;
  }

  .top-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 36px;
    padding: 0 0.85rem;
    border-radius: 12px;
    color: #e9d5ff;
    text-decoration: none;
    font-size: 0.78rem;
    font-weight: 850;
    background: rgba(124, 58, 237, 0.14);
    border: 1px solid rgba(167, 139, 250, 0.18);
  }

  .lang-switch {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 36px;
    padding: 0 0.78rem;
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.16);
    color: rgba(226, 232, 240, 0.82);
    background: rgba(255, 255, 255, 0.045);
    font-size: 0.78rem;
    font-weight: 900;
  }

  .fhub-content {
    position: relative;
    z-index: 1;
    padding: 1.55rem;
  }

  .fhub-bottom-nav {
    display: none;
  }

  .fshare-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(2, 6, 23, 0.72);
    backdrop-filter: blur(18px) saturate(140%);
  }

  .fshare-login-modal {
    position: relative;
    width: min(94vw, 460px);
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border: 1px solid rgba(167, 139, 250, 0.28);
    border-radius: 26px;
    background: radial-gradient(circle at 0% 0%, rgba(248, 193, 74, 0.18), transparent 34%), radial-gradient(circle at 100% 0%, rgba(56, 189, 248, 0.14), transparent 34%), linear-gradient(180deg, rgba(20, 26, 42, 0.98), rgba(7, 10, 18, 0.96));
    box-shadow: 0 30px 90px rgba(0, 0, 0, 0.5), 0 0 50px rgba(167, 139, 250, 0.12);
  }

  .modal-close { position: absolute; top: 0.85rem; right: 0.85rem; width: 42px; height: 42px; display: grid; place-items: center; border: 1px solid rgba(255,255,255,.1); border-radius: 14px; color: rgba(226,232,240,.78); background: rgba(255,255,255,.055); }
  .modal-close:hover { color: #fff; background: rgba(255,255,255,.1); }
  .modal-orb { width: 58px; height: 58px; display: grid; place-items: center; border-radius: 18px; color: #080a12; background: linear-gradient(135deg, #f8c14a, #a78bfa); }
  .modal-orb .material-icons { color: #080a12; font-size: 1.85rem; }
  .modal-copy h2 { margin: .25rem 0 0; color: #fff; font-size: 1.55rem; letter-spacing: -.04em; }
  .modal-copy p { margin: .5rem 0 0; color: rgba(226,232,240,.66); line-height: 1.5; }
  .modal-form, .modal-form label { display: grid; gap: .75rem; }
  .modal-form label > span { color: rgba(226,232,240,.72); font-size: .78rem; font-weight: 850; letter-spacing: .08em; text-transform: uppercase; }
  .modal-input { display: grid; grid-template-columns: 24px minmax(0,1fr) auto; align-items: center; gap: .72rem; min-height: 58px; padding: 0 .85rem; border: 1px solid rgba(148,163,184,.16); border-radius: 17px; background: rgba(3,6,14,.62); }
  .modal-input:focus-within { border-color: rgba(167,139,250,.42); box-shadow: 0 0 0 4px rgba(124,58,237,.12); }
  .modal-input .material-icons { color: #c4b5fd; font-size: 1.2rem; }
  .modal-input input { width: 100%; min-height: 56px; padding: 0; border: 0; outline: 0; color: #f8fafc; background: transparent; font-weight: 850; }
  .password-eye { width: 42px; height: 42px; display: grid; place-items: center; border: 1px solid rgba(167,139,250,.18); border-radius: 13px; color: #c4b5fd; background: rgba(255,255,255,.055); }
  .password-eye:hover { color: #fff; background: rgba(167,139,250,.16); }
  .modal-message { margin: -.15rem 0 0; padding: .72rem .82rem; border: 1px solid rgba(248,193,74,.18); border-radius: 14px; color: rgba(255,247,237,.84); background: rgba(248,193,74,.08); font-size: .84rem; font-weight: 780; line-height: 1.42; }
  .modal-submit { width: 100%; min-height: 54px; display: inline-flex; align-items: center; justify-content: center; gap: .45rem; border: 0; border-radius: 17px; color: #080a12; font-weight: 950; background: linear-gradient(100deg,#f8c14a 0%,#d6a8c9 50%,#a78bfa 100%); box-shadow: 0 18px 38px rgba(167,139,250,.13); }
  .modal-submit .material-icons { color: #080a12; }
  .modal-submit:disabled { opacity: .62; cursor: wait; }

  @media (max-width: 900px) {
    .fhub-shell {
      display: block;
      height: auto;
      min-height: 100dvh;
      overflow: visible;
      padding-bottom: 88px;
    }

    .fhub-sidebar {
      display: none;
    }

    .fhub-main {
      height: auto;
      min-height: 100dvh;
      overflow: visible;
    }

    .fhub-topbar {
      min-height: 64px;
      padding: 0 1rem;
    }

    .fhub-route-context,
    .top-search,
    .online-pill,
    .top-pill.account,
    .top-status.fshare-login,
    .top-status.fshare-vip span:not(.vip-logo),
    .lang-code {
      display: none;
    }

    .fhub-top-actions {
      gap: 0.42rem;
      margin-left: auto;
      min-width: 0;
    }

    .top-status {
      display: inline-grid;
      min-height: 38px;
      padding: 0 0.58rem;
      border-radius: 13px;
      gap: 0.42rem;
    }

    .top-status strong { font-size: 0.68rem; }
    .top-status small { font-size: 0.56rem; }
    .top-status.fshare-vip { grid-template-columns: auto; padding: 0 0.5rem; }
    .top-status.download-speed { grid-template-columns: 16px minmax(0, auto); }
    .top-status > .material-icons { width: 16px; font-size: 1.05rem; }
    .vip-logo { width: 38px; height: 26px; border-radius: 9px; font-size: 0.7rem; }
    .lang-switch { min-height: 38px; width: 46px; padding: 0; justify-content: center; }
    .lang-flag { font-size: 1.25rem; line-height: 1; }

    .fhub-mobile-brand {
      display: inline-grid;
    }

    .fhub-content {
      padding: 1rem;
    }

    .fhub-bottom-nav {
      position: fixed;
      left: 12px;
      right: 12px;
      bottom: 10px;
      z-index: 80;
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
      grid-auto-flow: column;
      grid-auto-columns: minmax(0, 1fr);
      gap: 4px;
      min-height: 58px;
      padding: 6px;
      border-radius: 18px;
      background: rgba(7, 10, 18, 0.92);
      border: 1px solid rgba(148, 163, 184, 0.16);
      box-shadow: 0 24px 70px rgba(0, 0, 0, 0.68), inset 0 1px 0 rgba(255, 255, 255, 0.04);
      backdrop-filter: blur(24px) saturate(150%);
    }

    .fhub-bottom-nav a {
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 2px;
      min-width: 0;
      min-height: 46px;
      padding: 0 1px;
      border-radius: 11px;
      font-size: clamp(0.46rem, 1.85vw, 0.58rem);
      line-height: 1;
      text-align: center;
      white-space: nowrap;
    }

    .fhub-bottom-nav a .material-icons {
      font-size: 1.18rem;
      line-height: 1;
    }

    .classic-mode {
      padding-bottom: 78px;
    }

    .classic-mode .fhub-bottom-nav {
      left: 50%;
      right: auto;
      bottom: 12px;
      width: min(360px, calc(100vw - 32px));
      min-height: 62px;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      padding: 7px;
      border-radius: 22px;
      transform: translateX(-50%);
    }

    .classic-mode .fhub-bottom-nav a {
      min-height: 48px;
      flex-direction: row;
      gap: 0.45rem;
      padding: 0 0.75rem;
      border-radius: 16px;
      font-size: 0.78rem;
    }

    .classic-mode .fhub-bottom-nav a .material-icons {
      font-size: 1.25rem;
    }

    .classic-mode .fhub-bottom-nav a.active {
      color: #080a12;
      background: linear-gradient(135deg, #f8c14a, #d6a8c9 52%, #a78bfa);
      box-shadow: 0 10px 30px rgba(167, 139, 250, 0.25);
    }

    .classic-mode .fhub-bottom-nav a.active .material-icons {
      color: #080a12;
    }

    .fhub-bottom-nav a.active {
      color: #f8fafc;
      background: rgba(124, 58, 237, 0.22);
    }
  }
</style>
