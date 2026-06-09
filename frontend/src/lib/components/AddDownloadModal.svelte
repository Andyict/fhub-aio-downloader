<script lang="ts">
  import { downloadStore } from "$lib/stores/downloads";
  import type { AddDownloadRequest } from "$lib/stores/downloads";
  import Badge from "$lib/components/ui/Badge.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
  }

  let { isOpen = $bindable(false), onClose }: Props = $props();

  type PreviewFolderItem = {
    name: string;
    url: string;
    size: number;
    is_directory: boolean;
    title: string;
    year?: number;
    season?: number;
    episode?: number;
    quality: string;
  };

  type PreviewFolderResponse = {
    success: boolean;
    original_url: string;
    resolved_url: string;
    folder_code: string;
    folder_name: string;
    file_count: number;
    total_size: number;
    recursive: boolean;
    items: PreviewFolderItem[];
  };

  let url = $state("");
  let filename = $state("");
  let category = $state("movies");
  let priority = $state<"NORMAL" | "HIGH" | "LOW">("NORMAL");
  let error = $state("");
  let isSubmitting = $state(false);
  let detectedHost = $state("");
  let previewingFolder = $state(false);
  let folderPreview = $state<PreviewFolderResponse | null>(null);
  let selectedFolderItems = $state<string[]>([]);
  let recursivePreview = $state(false);
  let pendingDownloadConfirm = $state<{ mode: "single" | "selected" | "all"; title: string; subtitle: string; count: number; size?: number; items: PreviewFolderItem[] } | null>(null);

  // Detect host from URL
  function unwrapFacebookRedirect(input: string): string {
    const trimmed = input.trim();
    if (!trimmed.includes("l.facebook.com/l.php?")) return trimmed;
    try {
      const urlObj = new URL(trimmed);
      const raw = urlObj.searchParams.get("u");
      return raw ? decodeURIComponent(raw) : trimmed;
    } catch {
      return trimmed;
    }
  }

  function isFshareFolderUrl(input: string): boolean {
    return unwrapFacebookRedirect(input).includes("fshare.vn/folder/");
  }

  function detectHost(urlString: string): string {
    if (!urlString) return "";
    const normalized = unwrapFacebookRedirect(urlString);
    try {
      const urlObj = new URL(normalized);
      const hostname = urlObj.hostname.toLowerCase();

      if (hostname.includes("fshare.vn")) return isFshareFolderUrl(normalized) ? "Fshare Folder" : "Fshare";
      if (hostname.includes("drive.google.com")) return "Google Drive";
      if (hostname.includes("mega.nz")) return "MEGA";
      if (hostname.includes("mediafire.com")) return "MediaFire";
      if (hostname.includes("facebook.com") || hostname.includes("l.facebook.com")) return "Facebook Redirect";

      return "Unknown";
    } catch {
      return "";
    }
  }

  // Watch URL changes to detect host
  $effect(() => {
    detectedHost = detectHost(url);
  });

  // Validate URL
  function validateUrl(urlString: string): boolean {
    if (!urlString.trim()) {
      error = "URL is required";
      return false;
    }

    const normalized = unwrapFacebookRedirect(urlString);

    try {
      new URL(normalized);
    } catch {
      error = "Invalid URL format";
      return false;
    }

    const host = detectHost(urlString);
    if (host === "Unknown" || host === "") {
      error = "Unsupported host. Currently supported: Fshare";
      return false;
    }

    return true;
  }

  function requestDownloadConfirm(payload: { mode: "single" | "selected" | "all"; title: string; subtitle: string; count: number; size?: number; items: PreviewFolderItem[] }) {
    pendingDownloadConfirm = payload;
  }

  function cancelDownloadConfirm() {
    pendingDownloadConfirm = null;
  }

  async function confirmPendingDownload() {
    const pending = pendingDownloadConfirm;
    pendingDownloadConfirm = null;
    if (!pending || isSubmitting) return;

    try {
      if (pending.mode === "single") {
        await addSingleDownload();
        return;
      }

      await addFolderDownloads(pending.items);
    } finally {
      // Hard guard: never leave the confirm button stuck at "Đang thêm...".
      isSubmitting = false;
    }
  }

  async function addSingleDownload() {
    isSubmitting = true;
    error = "";

    try {
      const request: AddDownloadRequest = {
        url: unwrapFacebookRedirect(url),
        category: category || undefined,
        priority: priority,
      };

      if (filename.trim()) {
        request.filename = filename.trim();
      }

      const response = await downloadStore.addDownload(request);

      if (response.success) {
        closeModal();
        resetForm();
      } else {
        error = response.error || "Failed to add download";
      }
    } catch (e: any) {
      error = e.message || "An unexpected error occurred";
    } finally {
      isSubmitting = false;
    }
  }

  async function addFolderDownloads(targetItems: PreviewFolderItem[]) {
    if (!folderPreview || !targetItems.length) return;

    isSubmitting = true;
    error = "";

    try {
      const batchId = crypto.randomUUID();
      const batchName = folderPreview.folder_name;

      for (const item of targetItems) {
        const response = await downloadStore.addDownload({
          url: item.url,
          filename: item.name,
          category: category || undefined,
          priority,
          batch_id: batchId,
          batch_name: batchName,
        });

        if (!response.success) {
          throw new Error(response.error || `Không thêm được file: ${item.name}`);
        }
      }

      closeModal();
      resetForm();
    } catch (e: any) {
      error = e.message || "Không add được file từ folder";
    } finally {
      isSubmitting = false;
    }
  }

  // Handle form submission
  async function handleSubmit() {
    error = "";

    if (!validateUrl(url)) {
      return;
    }

    if (isFshareFolderUrl(url)) {
      await previewFolder();
      return;
    }

    requestDownloadConfirm({
      mode: "single",
      title: "Start this download?",
      subtitle: `${filename.trim() || "Fshare file"}${category ? ` • ${category}` : ""}`,
      count: 1,
      items: [],
    });
  }

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }
    return `${value >= 100 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
  }

  async function previewFolder() {
    previewingFolder = true;
    folderPreview = null;
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 20000);
      let response: Response;
      try {
        response = await fetch(`/api/downloads/preview-folder`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: url.trim(), recursive: recursivePreview }),
          signal: controller.signal,
        });
      } finally {
        clearTimeout(timeout);
      }
      const text = await response.text();
      let data: any = {};
      try {
        data = text ? JSON.parse(text) : {};
      } catch {
        data = { error: text };
      }
      if (!response.ok) {
        throw new Error(data.error || data.message || `HTTP ${response.status}`);
      }
      folderPreview = data;
      selectedFolderItems = data.items.filter((item: PreviewFolderItem) => !item.is_directory).map((item: PreviewFolderItem) => item.url);
    } catch (e: any) {
      error = e.message || "Không preview được folder";
    } finally {
      previewingFolder = false;
    }
  }

  function toggleFolderItem(itemUrl: string, checked: boolean) {
    if (checked) {
      if (!selectedFolderItems.includes(itemUrl)) {
        selectedFolderItems = [...selectedFolderItems, itemUrl];
      }
      return;
    }
    selectedFolderItems = selectedFolderItems.filter((url) => url !== itemUrl);
  }

  function selectAllFolderItems() {
    if (!folderPreview) return;
    selectedFolderItems = folderPreview.items.filter((item) => !item.is_directory).map((item) => item.url);
  }

  function clearFolderSelection() {
    selectedFolderItems = [];
  }

  async function downloadFolderItems(mode: "selected" | "all") {
    if (!folderPreview) return;

    const targetItems = (mode === "all"
      ? folderPreview.items.filter((item) => !item.is_directory)
      : folderPreview.items.filter((item) => !item.is_directory && selectedFolderItems.includes(item.url))
    );

    if (!targetItems.length) {
      error = "Chưa có file nào được chọn";
      return;
    }

    const totalSize = targetItems.reduce((sum, item) => sum + (item.size || 0), 0);

    requestDownloadConfirm({
      mode,
      title: mode === "all" ? "Download all files?" : "Download selected files?",
      subtitle: `${folderPreview.folder_name} • ${formatBytes(totalSize)}`,
      count: targetItems.length,
      size: totalSize,
      items: targetItems,
    });
  }

  // Close modal
  function closeModal() {
    isOpen = false;
    if (onClose) onClose();
  }

  // Reset form
  function resetForm() {
    url = "";
    filename = "";
    category = "movies";
    priority = "NORMAL";
    error = "";
    detectedHost = "";
    folderPreview = null;
    previewingFolder = false;
    selectedFolderItems = [];
    recursivePreview = false;
    pendingDownloadConfirm = null;
  }

  // Handle escape key
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isOpen) {
      closeModal();
    }
  }

  // Handle overlay click
  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      closeModal();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<Modal
  open={isOpen}
  onClose={closeModal}
  maxWidth="520px"
  accent="var(--color-primary, #ff8a1f)"
  ariaLabel="Add Download"
>
  {#snippet header()}
    <div class="modal-title-row">
      <span class="material-icons modal-icon">add_circle</span>
      <h2 id="modal-title">Add Download</h2>
    </div>
    <button class="close-btn" onclick={closeModal} aria-label="Close modal">
      <span class="material-icons">close</span>
    </button>
  {/snippet}

  {#snippet children()}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
      <div class="form-group">
        <label for="download-url">
          <span class="label-text">URL</span>
          <span class="required">*</span>
        </label>
        <input
          id="download-url"
          type="url"
          bind:value={url}
          placeholder="https://fshare.vn/file/... hoặc /folder/..."
          required
          disabled={isSubmitting || previewingFolder}
          autocomplete="off"
        />
        {#if detectedHost}
          <Badge
            text={detectedHost}
            variant={detectedHost === "Fshare" ? "success" : "warning"}
            size="sm"
          />
        {/if}
      </div>

      <div class="form-group">
        <label for="download-filename">
          <span class="label-text">Filename</span>
          <span class="optional">(optional)</span>
        </label>
        <input
          id="download-filename"
          type="text"
          bind:value={filename}
          placeholder="movie.mkv"
          disabled={isSubmitting}
          autocomplete="off"
        />
        <div class="hint">Leave empty to use original filename</div>
      </div>

      {#if isFshareFolderUrl(url)}
        <label class="recursive-toggle">
          <input type="checkbox" bind:checked={recursivePreview} disabled={previewingFolder || isSubmitting} />
          <span>Recursive subfolder scan</span>
        </label>
      {/if}

      {#if folderPreview}
        <div class="folder-preview">
          <div class="folder-preview-head">
            <div>
              <strong>{folderPreview.folder_name}</strong>
              <div class="hint">{folderPreview.file_count} file • {formatBytes(folderPreview.total_size)} total • {folderPreview.recursive ? "recursive" : "current folder only"}</div>
            </div>
            <div class="folder-preview-tools">
              <button type="button" class="mini-action" onclick={selectAllFolderItems}>Select all</button>
              <button type="button" class="mini-action" onclick={clearFolderSelection}>Clear</button>
            </div>
          </div>
          <div class="folder-preview-list">
            {#each folderPreview.items as item}
              <label class="folder-preview-item" class:is-dir={item.is_directory}>
                <div class="folder-preview-main">
                  {#if !item.is_directory}
                    <input
                      type="checkbox"
                      checked={selectedFolderItems.includes(item.url)}
                      onchange={(e) => toggleFolderItem(item.url, (e.currentTarget as HTMLInputElement).checked)}
                    />
                  {:else}
                    <span class="folder-placeholder-check"></span>
                  {/if}
                  <span class="material-icons">{item.is_directory ? "folder" : "description"}</span>
                  <div class="folder-preview-meta">
                    <div class="folder-preview-name">{item.name}</div>
                    <div class="folder-preview-sub">{item.quality}{item.year ? ` • ${item.year}` : ""}</div>
                  </div>
                </div>
                <div class="folder-preview-size">{item.is_directory ? "Folder" : formatBytes(item.size)}</div>
              </label>
            {/each}
          </div>
          <div class="folder-bulk-actions">
            <Button type="button" variant="ghost" size="sm" onclick={() => downloadFolderItems("selected")} disabled={isSubmitting || selectedFolderItems.length === 0}>Download selected</Button>
            <Button type="button" size="sm" onclick={() => downloadFolderItems("all")} disabled={isSubmitting}>Download all</Button>
          </div>
        </div>
      {/if}

      <div class="form-row">
        <div class="form-group">
          <label for="download-category">
            <span class="label-text">Category</span>
          </label>
          <select
            id="download-category"
            bind:value={category}
            disabled={isSubmitting}
          >
            <option value="movies">Movies</option>
            <option value="tv">TV Shows</option>
            <option value="music">Music</option>
            <option value="other">Other</option>
          </select>
        </div>

        <div class="form-group">
          <label for="download-priority">
            <span class="label-text">Priority</span>
          </label>
          <select
            id="download-priority"
            bind:value={priority}
            disabled={isSubmitting}
          >
            <option value="LOW">Low</option>
            <option value="NORMAL">Normal</option>
            <option value="HIGH">High</option>
          </select>
        </div>
      </div>

      {#if error}
        <div class="error-message">
          <span class="material-icons">error</span>
          <span>{error}</span>
        </div>
      {/if}

      <div class="modal-actions">
        <Button
          variant="ghost"
          size="md"
          type="button"
          onclick={closeModal}
          disabled={isSubmitting}>Cancel</Button
        >
        <Button
          size="md"
          icon={isSubmitting ? "sync" : "download"}
          loading={isSubmitting}
          type="submit"
          disabled={isSubmitting || previewingFolder}
          >{previewingFolder ? "Previewing Folder…" : isSubmitting ? "Initializing…" : isFshareFolderUrl(url) ? "Preview Folder" : "Start Download"}</Button
        >
      </div>
    </form>

    <div class="keyboard-hint">
      <span class="material-icons">keyboard</span>
      Press <kbd>Esc</kbd> to close
    </div>

    {#if pendingDownloadConfirm}
      <div class="confirm-layer" role="dialog" aria-modal="true" aria-label="Confirm download">
        <div class="confirm-card">
          <div class="confirm-hero">
            <span class="material-icons">download_for_offline</span>
            <div>
              <div class="confirm-eyebrow">Confirm download</div>
              <h3>{pendingDownloadConfirm.title}</h3>
              <p>{pendingDownloadConfirm.subtitle}</p>
            </div>
          </div>

          <div class="confirm-stats">
            <div><strong>{pendingDownloadConfirm.count}</strong><span>{pendingDownloadConfirm.count === 1 ? "file" : "files"}</span></div>
            <div><strong>{pendingDownloadConfirm.size ? formatBytes(pendingDownloadConfirm.size) : "—"}</strong><span>total size</span></div>
            <div><strong>{category}</strong><span>category</span></div>
          </div>

          {#if pendingDownloadConfirm.items.length > 0}
            <div class="confirm-file-list">
              {#each pendingDownloadConfirm.items.slice(0, 5) as item}
                <div class="confirm-file-row"><span class="material-icons">description</span><span>{item.name}</span><em>{formatBytes(item.size)}</em></div>
              {/each}
              {#if pendingDownloadConfirm.items.length > 5}
                <div class="confirm-more">+{pendingDownloadConfirm.items.length - 5} more files</div>
              {/if}
            </div>
          {/if}

          <div class="confirm-actions">
            <Button type="button" variant="ghost" size="md" onclick={cancelDownloadConfirm}>Cancel</Button>
            <Button type="button" size="md" icon="download" onclick={confirmPendingDownload}>Start download</Button>
          </div>
        </div>
      </div>
    {/if}
  {/snippet}

</Modal>

<style>

  .confirm-layer { position:absolute; inset:0; z-index:20; display:flex; align-items:center; justify-content:center; padding:1rem; background:rgba(0,0,0,.62); backdrop-filter: blur(10px); }
  .confirm-card { width:min(460px,100%); border:1px solid rgba(255, 138, 31, .22); background:linear-gradient(160deg, rgba(12,16,28,.98), rgba(5,7,12,.98)); border-radius:18px; padding:1rem; box-shadow:0 24px 60px rgba(0,0,0,.55), 0 0 36px rgba(255, 138, 31, .12); }
  .confirm-hero { display:flex; gap:.85rem; align-items:flex-start; padding:.35rem .25rem 1rem; }
  .confirm-hero > .material-icons { color:var(--color-primary,#ff8a1f); font-size:2.1rem; }
  .confirm-eyebrow { color:var(--color-primary,#ff8a1f); text-transform:uppercase; letter-spacing:.12em; font-size:.62rem; font-weight:900; }
  .confirm-hero h3 { margin:.15rem 0; color:#fff; font-size:1.05rem; line-height:1.25; }
  .confirm-hero p { margin:0; color:var(--text-muted,#9aa4b2); font-size:.78rem; }
  .confirm-stats { display:grid; grid-template-columns:repeat(3,1fr); gap:.55rem; margin-bottom:.8rem; }
  .confirm-stats div { padding:.65rem; border:1px solid rgba(255,255,255,.08); background:rgba(255,255,255,.035); border-radius:12px; min-width:0; }
  .confirm-stats strong { display:block; color:#fff; font-size:.9rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .confirm-stats span { color:var(--text-muted,#888); text-transform:uppercase; letter-spacing:.08em; font-size:.58rem; font-weight:800; }
  .confirm-file-list { max-height:180px; overflow:auto; display:flex; flex-direction:column; gap:.4rem; margin:.65rem 0; }
  .confirm-file-row { display:grid; grid-template-columns:20px minmax(0,1fr) auto; gap:.5rem; align-items:center; padding:.5rem; border-radius:10px; background:rgba(0,0,0,.22); color:#e8eef8; font-size:.72rem; }
  .confirm-file-row .material-icons { font-size:16px; color:var(--color-primary,#ff8a1f); }
  .confirm-file-row span:nth-child(2) { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .confirm-file-row em { color:var(--text-muted,#888); font-style:normal; font-size:.68rem; }
  .confirm-more { color:var(--text-muted,#888); font-size:.72rem; text-align:center; padding:.3rem; }
  .confirm-actions { display:flex; gap:.75rem; justify-content:flex-end; margin-top:1rem; }

  /* Header title row */
  .modal-title-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .modal-icon {
    color: var(--color-primary, #ff8a1f);
    font-size: 1.3rem;
  }
  h2 {
    font-size: 1.1rem;
    font-weight: 800;
    color: #fff;
    margin: 0;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    padding: 0.5rem;
    border-radius: 8px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary, #fff);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .required {
    color: #ff0064;
  }

  .optional {
    color: var(--text-muted, #888);
    font-weight: 400;
    font-size: 0.75rem;
  }

  input,
  select {
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 0.8rem 1.2rem;
    color: var(--text-primary, #fff);
    font-size: 0.875rem;
    font-family: var(--font-mono, monospace);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    clip-path: polygon(
      0% 0%,
      calc(100% - 10px) 0%,
      100% 10px,
      100% 100%,
      10px 100%,
      0% calc(100% - 10px)
    );
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--color-primary, #ff8a1f);
    background: rgba(255, 138, 31, 0.05);
    box-shadow: 0 0 20px rgba(255, 138, 31, 0.1);
  }

  .hint {
    font-size: 0.65rem;
    color: var(--text-muted, #888);
    margin-top: 0.25rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .folder-preview {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    padding: 0.9rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.03);
    border-radius: 10px;
  }

  .folder-preview-head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .folder-preview-tools,
  .folder-bulk-actions {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .folder-preview-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    max-height: 280px;
    overflow: auto;
  }

  .recursive-toggle {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.85rem;
    color: #d8e2f0;
  }

  .recursive-toggle input {
    width: 16px;
    height: 16px;
    margin: 0;
  }

  .folder-preview-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.7rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid rgba(255,255,255,0.08);
    background: rgba(0,0,0,0.22);
    border-radius: 8px;
  }

  .folder-preview-main {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
    flex: 1;
  }

  .folder-preview-item.is-dir {
    opacity: 0.75;
  }

  .folder-preview-item input {
    width: 16px;
    height: 16px;
    margin: 0;
  }

  .folder-placeholder-check {
    width: 16px;
    height: 16px;
    flex: 0 0 16px;
  }

  .folder-preview-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
  }

  .folder-preview-name {
    color: #fff;
    font-size: 0.86rem;
    word-break: break-word;
  }

  .folder-preview-sub {
    color: var(--text-muted, #888);
    font-size: 0.72rem;
  }

  .folder-preview-size {
    flex: 0 0 auto;
    color: #c6d2df;
    font-size: 0.78rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .mini-action {
    border: 1px solid rgba(255,255,255,0.12);
    background: rgba(255,255,255,0.04);
    color: #d8e2f0;
    border-radius: 8px;
    padding: 0.45rem 0.65rem;
    cursor: pointer;
  }

  @media (max-width: 640px) {
    .form-row {
      grid-template-columns: 1fr;
    }

    .folder-preview-head,
    .folder-preview-item,
    .folder-bulk-actions {
      flex-direction: column;
      align-items: stretch;
    }

    .folder-preview-main {
      align-items: flex-start;
    }

    .folder-preview-size {
      padding-left: 2rem;
    }
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: rgba(255, 0, 100, 0.1);
    border: 1px solid rgba(255, 0, 100, 0.3);
    border-radius: 8px;
    color: #ff0064;
    font-size: 0.875rem;
  }

  .error-message .material-icons {
    font-size: 1.25rem;
  }

  .modal-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  .btn-secondary,
  .btn-primary {
    flex: 1;
    padding: 0.8rem 1.5rem;
    font-size: 0.75rem;
    font-weight: 900;
    letter-spacing: 0.15em;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    border: none;
    font-family: var(--font-mono, monospace);
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted);
    border: 1px solid rgba(255, 255, 255, 0.1);
    clip-path: polygon(
      8px 0%,
      100% 0%,
      100% calc(100% - 8px),
      calc(100% - 8px) 100%,
      0% 100%,
      0% 8px
    );
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    border-color: rgba(255, 255, 255, 0.2);
  }

  .btn-primary {
    background: linear-gradient(135deg, #0a1018 0%, #152030 50%, #0a1018 100%);
    color: var(--color-primary);
    border: 2px solid var(--color-primary);
    position: relative;
    overflow: hidden;
    clip-path: polygon(
      8px 0%,
      100% 0%,
      100% calc(100% - 8px),
      calc(100% - 8px) 100%,
      0% 100%,
      0% 8px
    );
  }

  .btn-primary::before {
    content: "";
    position: absolute;
    inset: -2px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 138, 31, 0.2) 45%,
      rgba(255, 138, 31, 0.6) 50%,
      rgba(255, 138, 31, 0.2) 55%,
      transparent 100%
    );
    animation: scan 3s linear infinite;
    opacity: 0;
    transition: opacity 0.3s;
    pointer-events: none;
  }

  .btn-primary:hover:not(:disabled) {
    box-shadow: 0 0 25px rgba(255, 138, 31, 0.4);
    color: #fff;
    border-color: #fff;
    transform: translateY(-2px);
  }

  .btn-primary:hover:not(:disabled)::before {
    opacity: 1;
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(0, 0, 0, 0.3);
    border-top-color: #000;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .keyboard-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 1.5rem;
    padding-top: 1.5rem;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    font-size: 0.75rem;
    color: var(--text-muted, #888);
  }

  .keyboard-hint .material-icons {
    font-size: 1rem;
  }

  kbd {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    padding: 0.125rem 0.5rem;
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
    color: var(--text-primary, #fff);
  }

  @media (max-width: 640px) {
    .form-row {
      grid-template-columns: 1fr;
    }

    .modal-actions {
      flex-direction: column;
    }
  }
</style>
