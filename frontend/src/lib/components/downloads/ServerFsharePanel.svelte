<script lang="ts">
  import { onMount, tick } from "svelte";
  import { accountStore } from "$lib/stores/account.svelte";
  import { toasts } from "$lib/stores/toasts";
  import { downloadStore } from "$lib/stores/downloads";

  type PreviewItem = {
    name: string;
    url: string;
    size: number;
    is_directory?: boolean;
    title?: string;
    year?: number;
    season?: number;
    episode?: number;
    quality?: string;
  };

  type PreviewResponse = {
    success: boolean;
    original_url: string;
    resolved_url: string;
    folder_name?: string;
    file_count?: number;
    total_size?: number;
    items: PreviewItem[];
  };

  let fshareUrl = $state("");
  let recursive = $state(true);
  let previewLoading = $state(false);
  let submitLoading = $state(false);
  let preview = $state<PreviewResponse | null>(null);
  let selectedUrls = $state<Set<string>>(new Set());
  let showConfirm = $state(false);
  let fshareEmail = $state("");
  let fsharePassword = $state("");
  let fshareLoginLoading = $state(false);
  let showPassword = $state(false);

  const activeAccount = $derived(accountStore.primaryFormatted);
  const hasFshareAccount = $derived(Boolean(accountStore.primary?.email));
  const needsVipRefresh = $derived(Boolean(accountStore.primary?.email) && (!accountStore.primary?.rank || accountStore.primary?.rank === "UNVERIFIED"));
  const downloadableItems = $derived((preview?.items || []).filter((item) => !item.is_directory));
  const selectedItems = $derived(downloadableItems.filter((item) => selectedUrls.has(item.url)));
  const selectedSize = $derived(selectedItems.reduce((sum, item) => sum + (item.size || 0), 0));

  async function autoRefreshVipStatus(showError = false) {
    const email = accountStore.primary?.email;
    if (!email) return false;

    for (let attempt = 1; attempt <= 3; attempt += 1) {
      const result = await accountStore.refresh(email);
      await accountStore.fetch();
      const rank = accountStore.primary?.rank;
      if (result?.success || (rank && rank !== "UNVERIFIED")) return true;
      if (attempt < 3) await new Promise((resolve) => setTimeout(resolve, 1200));
    }

    if (showError) {
      toasts.error("Không xác minh được VIP sau 3 lần thử. Vui lòng kiểm tra tài khoản FShare hoặc thử lại sau.");
    }
    return false;
  }

  onMount(async () => {
    await accountStore.fetch();
    const email = accountStore.primary?.email;
    const rank = accountStore.primary?.rank;
    if (email && (!rank || rank === "UNVERIFIED")) {
      await autoRefreshVipStatus(true);
    }
  });

  function formatBytes(bytes: number) {
    if (!bytes) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = bytes;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size.toFixed(size >= 10 || unit === 0 ? 0 : 1)} ${units[unit]}`;
  }

  function inferFileName(url: string) {
    const clean = url.split("?")[0].replace(/\/$/, "");
    const code = clean.split("/").pop() || "fshare-file";
    return `FShare-${code}`;
  }

  function normalizeUrl(url: string) {
    const trimmed = url.trim();
    if (!trimmed) return "";
    if (/^https?:\/\//i.test(trimmed)) return trimmed;
    return `https://${trimmed}`;
  }

  function isFolderUrl(url: string) {
    return /fshare\.vn\/folder\//i.test(url);
  }

  function selectAll() {
    selectedUrls = new Set(downloadableItems.map((item) => item.url));
  }

  function clearSelection() {
    selectedUrls = new Set();
  }

  function toggleItem(url: string) {
    const next = new Set(selectedUrls);
    if (next.has(url)) next.delete(url);
    else next.add(url);
    selectedUrls = next;
  }

  async function loginFshare() {
    if (!fshareEmail.trim() || !fsharePassword.trim()) {
      toasts.error("Nhập email và mật khẩu FShare trước");
      return false;
    }

    fshareLoginLoading = true;
    try {
      const ok = await accountStore.switchAccount(fshareEmail.trim(), fsharePassword);
      if (!ok) {
        toasts.error("Đăng nhập FShare thất bại");
        return false;
      }

      await accountStore.fetch();
      await tick();
      await new Promise((resolve) => setTimeout(resolve, 150));
      await accountStore.fetch();
      await tick();

      const verified = await autoRefreshVipStatus(false);
      await accountStore.fetch();
      await tick();
      fsharePassword = "";

      verified
        ? toasts.success("Đã đăng nhập và xác minh FShare")
        : toasts.success("Đã đăng nhập FShare. Nếu VIP chưa hiện ngay, hệ thống sẽ tự xác minh tiếp.");

      return true;
    } finally {
      fshareLoginLoading = false;
    }
  }

  async function inspectLink() {
    const url = normalizeUrl(fshareUrl);
    if (!url || !/fshare\.vn\/(file|folder)\//i.test(url)) {
      toasts.error("Dán link FShare file hoặc folder hợp lệ");
      return;
    }

    if (!hasFshareAccount) {
      toasts.error("Cần đăng nhập tài khoản FShare trước khi tải");
      return;
    }

    previewLoading = true;
    preview = null;
    selectedUrls = new Set();

    try {
      const response = await fetch("/api/downloads/preview-link", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url, recursive }),
      });
      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Không kiểm tra được link FShare");
      }
      preview = await response.json();
      selectAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không kiểm tra được link";
      toasts.error(message);
    } finally {
      previewLoading = false;
    }
  }

  async function confirmDownload() {
    if (!selectedItems.length || submitLoading) return;
    submitLoading = true;

    const batchId = crypto.randomUUID();
    const batchName = preview?.folder_name || "FShare external link";

    try {
      for (const item of selectedItems) {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 20000);
        let response: Response;
        try {
          response = await fetch("/api/downloads", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            signal: controller.signal,
            body: JSON.stringify({
              url: item.url,
              filename: item.name,
              category: "fshare",
              priority: "NORMAL",
              batch_id: batchId,
              batch_name: batchName,
            }),
          });
        } finally {
          clearTimeout(timeout);
        }
        if (!response.ok) {
          const message = await response.text();
          throw new Error(message || `Không thêm được ${item.name}`);
        }
      }

      toasts.success(`Đã thêm ${selectedItems.length} file vào hàng đợi tải về server`);
      showConfirm = false;
      await downloadStore.fetchAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thêm được tác vụ tải";
      toasts.error(message);
    } finally {
      submitLoading = false;
    }
  }
</script>

<section class="server-fshare-panel" aria-label="Tải link FShare về server">
  <div class="panel-hero">
    <div class="hero-copy">
      <span class="eyebrow">SERVER DOWNLOAD</span>
      <h2>Tải link FShare về NAS</h2>
      <p>
        Dán link file hoặc thư mục FShare. FHub sẽ tải trên server/container, không tải về điện thoại hay máy đang mở web.
      </p>
    </div>
    <div class="account-pill" class:connected={hasFshareAccount}>
      <span class="material-icons">{hasFshareAccount ? "verified_user" : "lock"}</span>
      <div>
        <strong>{hasFshareAccount ? activeAccount.email : "Chưa đăng nhập FShare"}</strong>
        <small>{hasFshareAccount ? activeAccount.rank : "Cần tài khoản FShare để tải"}</small>
        {#if needsVipRefresh}
          <small>Đang tự xác minh VIP...</small>
        {/if}
      </div>
    </div>
  </div>

  {#if !hasFshareAccount}
    <div class="fshare-login-card">
      <div class="login-copy">
        <strong>FShare Account</strong>
        <span>Đăng nhập tài khoản FShare để server lấy link tải tốc độ cao.</span>
      </div>
      <div class="login-fields">
        <div class="field-group">
          <label class="field-label" for="server-fshare-email">Email FShare</label>
          <div class="input-row">
            <span class="material-icons input-icon" aria-hidden="true">mail</span>
            <input
              id="server-fshare-email"
              bind:value={fshareEmail}
              type="email"
              placeholder="name@email.com"
              autocomplete="username"
            />
          </div>
        </div>
        <div class="field-group">
          <label class="field-label" for="server-fshare-password">Mật khẩu</label>
          <div class="input-row password-row">
            <span class="material-icons input-icon" aria-hidden="true">lock</span>
            <input
              id="server-fshare-password"
              bind:value={fsharePassword}
              type={showPassword ? "text" : "password"}
              placeholder="••••••••••••"
              autocomplete="current-password"
            />
            <button
              class="vis-toggle"
              type="button"
              onclick={() => (showPassword = !showPassword)}
              aria-label="Ẩn hiện mật khẩu"
            >
              <span class="material-icons">{showPassword ? "visibility_off" : "visibility"}</span>
            </button>
          </div>
        </div>
      </div>
      <button class="primary-btn login-btn" type="button" onclick={loginFshare} disabled={fshareLoginLoading}>
        {fshareLoginLoading ? "Đang đăng nhập..." : "Đăng nhập FShare"}
      </button>
    </div>
  {/if}

  <div class="link-console">
    <div class="link-input-wrap">
      <span class="material-icons">link</span>
      <input bind:value={fshareUrl} placeholder="https://www.fshare.vn/file/... hoặc /folder/..." />
    </div>
    <button class="primary-btn" type="button" onclick={inspectLink} disabled={previewLoading || !hasFshareAccount}>
      {previewLoading ? "Đang kiểm tra..." : "Kiểm tra link"}
    </button>
  </div>

  {#if preview}
    <div class="preview-card">
      <div class="preview-head">
        <div>
          <h3>{preview.folder_name || "FShare link"}</h3>
          <small class="preview-kind">{isFolderUrl(preview.resolved_url) ? "Thư mục FShare" : "File FShare"}</small>
          <p>
            {downloadableItems.length} file · {formatBytes(preview.total_size || selectedSize)}
          </p>
        </div>
        <div class="preview-actions">
          <button type="button" onclick={selectAll}>Tất cả</button>
          <button type="button" onclick={clearSelection}>Bỏ chọn</button>
        </div>
      </div>

      <div class="file-list">
        {#each downloadableItems as item}
          <label class="file-row">
            <input
              type="checkbox"
              checked={selectedUrls.has(item.url)}
              onchange={() => toggleItem(item.url)}
            />
            <span class="material-icons file-icon">{item.quality?.toLowerCase().includes("subtitle") ? "subtitles" : "movie"}</span>
            <span class="file-main">
              <strong>{item.name}</strong>
              <small>{item.quality || "FShare"} · {formatBytes(item.size)}</small>
            </span>
          </label>
        {/each}
      </div>

      <div class="confirm-strip">
        <span>{selectedItems.length} file đã chọn · {formatBytes(selectedSize)}</span>
        <button class="primary-btn" type="button" onclick={() => (showConfirm = true)} disabled={!selectedItems.length}>
          Xác nhận tải
        </button>
      </div>
    </div>
  {/if}
</section>

{#if showConfirm}
  <div class="confirm-backdrop" role="presentation">
    <div class="confirm-modal" role="dialog" aria-modal="true" aria-label="Xác nhận tải">
      <div class="confirm-icon">
        <span class="material-icons">download_for_offline</span>
      </div>
      <h3>Xác nhận tải về server?</h3>
      <p>
        FHub sẽ thêm {selectedItems.length} file vào hàng đợi tải của container. File sẽ lưu theo cấu hình thư mục tải hiện tại trên server/NAS.
      </p>
      <div class="confirm-summary">
        <span>Tổng dung lượng</span>
        <strong>{formatBytes(selectedSize)}</strong>
      </div>
      <div class="modal-actions">
        <button type="button" class="secondary-btn" onclick={() => (showConfirm = false)} disabled={submitLoading}>
          Hủy
        </button>
        <button type="button" class="primary-btn" onclick={confirmDownload} disabled={submitLoading}>
          {submitLoading ? "Đang thêm..." : "Xác nhận tải"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .server-fshare-panel {
    display: grid;
    gap: 1rem;
    margin-bottom: 1rem;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 24px;
    background:
      radial-gradient(circle at 0% 0%, rgba(129, 140, 248, 0.08), transparent 34%),
      linear-gradient(145deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.028));
  }

  .panel-hero,
  .link-console,
  .preview-head,
  .confirm-strip,
  .fshare-login-card {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .panel-hero,
  .preview-head,
  .confirm-strip {
    justify-content: space-between;
  }

  .eyebrow {
    color: #818cf8;
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.14em;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    margin-top: 0.2rem;
    color: #fff;
    font-size: clamp(1.28rem, 2vw, 2rem);
    letter-spacing: -0.03em;
  }

  .hero-copy p,
  .preview-head p,
  .login-copy span,
  .file-row small,
  .confirm-modal p {
    color: #8ea0b5;
    line-height: 1.45;
  }

  .account-pill {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 230px;
    padding: 0.75rem;
    border: 1px solid rgba(139, 92, 246, 0.14);
    border-radius: 16px;
    background: rgba(139, 92, 246, 0.05);
  }

  .account-pill.connected {
    border-color: rgba(100, 116, 139, 0.28);
    background: rgba(100, 116, 139, 0.08);
  }

  .account-pill .material-icons {
    color: #8b5cf6;
  }

  .account-pill.connected .material-icons {
    color: #64748b;
  }

  .account-pill strong,
  .account-pill small {
    display: block;
  }

  .account-pill strong {
    max-width: 190px;
    overflow: hidden;
    color: #e5eef7;
    font-size: 0.82rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-pill small {
    margin-top: 0.15rem;
    color: #8ea0b5;
    font-size: 0.72rem;
  }

  .fshare-login-card {
    display: grid;
    grid-template-columns: minmax(170px, 0.85fr) minmax(0, 1.9fr) auto;
    align-items: end;
    gap: 0.95rem;
    padding: 1rem;
    border: 1px solid rgba(139, 92, 246, 0.11);
    border-radius: 20px;
    background: linear-gradient(180deg, rgba(139, 92, 246, 0.06), rgba(139, 92, 246, 0.035));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .login-copy {
    display: grid;
    align-content: center;
    gap: 0.24rem;
    padding-bottom: 0.15rem;
  }

  .login-copy strong {
    color: #fff;
    font-size: 0.96rem;
  }

  .login-copy span {
    font-size: 0.78rem;
  }

  .login-fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
    min-width: 0;
  }

  .field-group {
    display: grid;
    gap: 0.42rem;
    min-width: 0;
  }

  .field-label {
    color: rgba(255, 255, 255, 0.58);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .input-row {
    min-height: 48px;
    display: flex;
    align-items: center;
    gap: 0.62rem;
    padding: 0 0.78rem;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.09);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.065), rgba(255, 255, 255, 0.03));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.045);
    transition: border-color 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
  }

  .input-row:focus-within {
    border-color: rgba(139, 92, 246, 0.24);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.085), rgba(255, 255, 255, 0.04));
    box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.10);
  }

  .input-icon {
    color: rgba(255, 255, 255, 0.42);
    font-size: 1.05rem;
    flex-shrink: 0;
  }

  .input-row input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: #eaf2fb;
    font-size: 0.92rem;
    padding: 0.78rem 0;
    font-family: inherit;
  }

  .input-row input::placeholder {
    color: rgba(255, 255, 255, 0.24);
  }

  .password-row {
    padding-right: 0.42rem;
  }

  .vis-toggle {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.045);
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
    transition: all 0.18s ease;
  }

  .vis-toggle:hover {
    color: rgba(255, 255, 255, 0.82);
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.12);
  }

  .vis-toggle .material-icons {
    font-size: 1rem;
  }

  .field-shell {
    position: relative;
    display: grid;
    gap: 0.38rem;
    min-width: 0;
  }

  .field-shell > span:first-child {
    color: #b9c7d7;
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .input-shell {
    padding: 0.72rem 0.78rem 0.78rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    background: rgba(7, 11, 18, 0.38);
    transition: border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
  }

  .input-shell:focus-within {
    border-color: rgba(129, 140, 248, 0.18);
    background: rgba(7, 11, 18, 0.55);
    box-shadow: 0 0 0 3px rgba(129, 140, 248, 0.06);
  }

  .field-icon {
    position: absolute;
    left: 0.88rem;
    bottom: 0.92rem;
    color: #7e93a8;
    font-size: 1.08rem;
    pointer-events: none;
  }

  .link-input-wrap,
  .password-field {
    position: relative;
    flex: 1;
  }

  .link-input-wrap .material-icons {
    position: absolute;
    left: 0.9rem;
    top: 50%;
    color: #818cf8;
    transform: translateY(-50%);
  }

  input {
    width: 100%;
    min-height: 46px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    background: rgba(7, 11, 18, 0.72);
    color: #e5eef7;
    outline: none;
    padding: 0 0.9rem;
  }

  .input-shell input {
    min-height: 42px;
    border: 0;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.035);
  }

  .link-input-wrap input {
    padding-left: 2.8rem;
  }

  input:focus {
    border-color: rgba(129, 140, 248, 0.24);
    box-shadow: 0 0 0 3px rgba(129, 140, 248, 0.08);
  }

  .input-shell input:focus {
    box-shadow: none;
  }

  .password-field input {
    padding-right: 3rem;
  }

  .has-leading-icon input {
    padding-left: 2.65rem;
  }

  .field-shell input {
    min-width: 0;
  }

  .password-field button {
    position: absolute;
    right: 0.28rem;
    bottom: 0.22rem;
    width: 38px;
    height: 38px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: #7f93a8;
    transition: color 0.2s ease, background 0.2s ease;
  }

  .password-field button:hover {
    color: #e5eef7;
    background: rgba(255, 255, 255, 0.07);
  }

  .login-btn {
    min-width: 118px;
    height: 46px;
    align-self: end;
    padding: 0 1.15rem;
    white-space: nowrap;
    box-shadow: 0 12px 24px rgba(129, 140, 248, 0.08);
  }

  .file-row input {
    width: auto;
    min-height: auto;
    accent-color: #818cf8;
  }

  .primary-btn,
  .secondary-btn,
  .preview-actions button {
    min-height: 44px;
    border-radius: 14px;
    border: 0;
    padding: 0 1rem;
    font-weight: 850;
    cursor: pointer;
  }

  .primary-btn {
    background: linear-gradient(135deg, #818cf8, #64748b);
    color: #061018;
  }

  .secondary-btn,
  .preview-actions button {
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.05);
    color: #dbe8f4;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preview-card {
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    background: rgba(7, 11, 18, 0.48);
  }

  .preview-head,
  .confirm-strip {
    padding: 0.9rem;
  }

  .preview-head {
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .preview-head h3 {
    color: #fff;
    font-size: 1rem;
  }

  .preview-actions {
    display: flex;
    gap: 0.5rem;
  }

  .file-list {
    max-height: 360px;
    overflow: auto;
    padding: 0.45rem;
  }

  .file-row {
    display: grid;
    grid-template-columns: auto 34px minmax(0, 1fr);
    gap: 0.65rem;
    align-items: center;
    min-height: 64px;
    padding: 0.65rem;
    border-radius: 14px;
  }

  .file-row:hover {
    background: rgba(255, 255, 255, 0.045);
  }

  .file-icon {
    color: #818cf8;
  }

  .file-main {
    min-width: 0;
  }

  .file-main strong,
  .file-main small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-main strong {
    color: #e5eef7;
    font-size: 0.86rem;
  }

  .confirm-strip {
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    color: #b8c7d8;
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10000;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.68);
    backdrop-filter: blur(12px);
  }

  .confirm-modal {
    width: min(100%, 430px);
    padding: 1.25rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 24px;
    background: #0b111c;
    box-shadow: 0 30px 90px rgba(0, 0, 0, 0.45);
  }

  @media (max-width: 1180px) {
    .fshare-login-card {
      grid-template-columns: 1fr;
      align-items: stretch;
    }

    .login-btn {
      width: 100%;
    }
  }

  @media (max-width: 760px) {
    .login-fields {
      grid-template-columns: 1fr;
    }
  }

  .confirm-icon {
    width: 54px;
    height: 54px;
    display: grid;
    place-items: center;
    margin-bottom: 0.9rem;
    border-radius: 16px;
    background: rgba(129, 140, 248, 0.08);
    color: #818cf8;
  }

  .confirm-modal h3 {
    margin-bottom: 0.5rem;
    color: #fff;
    font-size: 1.25rem;
  }

  .confirm-summary {
    display: flex;
    justify-content: space-between;
    margin: 1rem 0;
    padding: 0.8rem;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.05);
    color: #8ea0b5;
  }

  .confirm-summary strong {
    color: #fff;
  }

  .modal-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  @media (max-width: 760px) {
    .server-fshare-panel {
      padding: 0.85rem;
      border-radius: 20px;
    }

    .panel-hero,
    .link-console,
    .preview-head,
    .confirm-strip,
    .fshare-login-card {
      align-items: stretch;
      flex-direction: column;
    }

    .account-pill,
    .login-copy {
      width: 100%;
      min-width: 0;
    }

    .primary-btn,
    .secondary-btn {
      width: 100%;
    }

    .preview-actions,
    .modal-actions {
      width: 100%;
      grid-template-columns: 1fr 1fr;
      display: grid;
    }
  }
</style>
