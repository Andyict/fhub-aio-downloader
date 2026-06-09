<script lang="ts">
  import { onMount, tick } from "svelte";

  type ApiDownload = {
    id?: string | number;
    filename?: string;
    name?: string;
    title?: string;
    batch_name?: string;
    tmdb_title?: string;
    tmdb_id?: number | string;
    media_type?: string;
    category?: string;
    state?: string;
    status?: string;
    progress?: number;
    speed?: number | string;
    downloaded_bytes?: number;
    total_bytes?: number;
    size?: string;
    owner?: string;
    url?: string;
    output_path?: string;
    file_path?: string;
    save_path?: string;
  };

  type EngineStats = {
    active_downloads?: number;
    queued?: number;
    completed?: number;
    failed?: number;
    total_speed?: number;
  };

  type QueueItem = {
    id: string;
    title: string;
    owner: string;
    size: string;
    speed: string;
    progress: number;
    code: string;
    state: "downloading" | "queued" | "completed" | "paused" | "failed";
    banner?: string;
    tmdbId?: string;
    mediaType?: string;
    downloadedBytes?: number;
    totalBytes?: number;
    sourceUrl?: string;
    outputPath?: string;
    viTitle?: string;
    runtime?: number;
  };

  type PreviewItem = {
    name: string;
    url: string;
    size: number;
    is_directory?: boolean;
    quality?: string;
  };

  type LinkPreview = {
    original_url: string;
    resolved_url: string;
    folder_name?: string;
    file_count?: number;
    total_size?: number;
    items: PreviewItem[];
  };

  let downloads = $state<QueueItem[]>([]);
  let stats = $state<EngineStats>({});
  let fshareLink = $state("");
  let activeTab = $state<"all" | QueueItem["state"]>("all");
  let status = $state("Đang kết nối queue FHUB...");
  let language = $state<"vi" | "en">("vi");
  let checkLoading = $state(false);
  let downloadLoading = $state(false);
  let preview = $state<LinkPreview | null>(null);
  let selectedUrls = $state<Set<string>>(new Set());
  let pendingDownloadItems = $state<PreviewItem[]>([]);
  let pendingDownloadName = $state("");
  let pendingDownloadSize = $state(0);
  let recursive = $state(true);
  let showDownloadConfirm = $state(false);
  let confirmTapInProgress = $state(false);
  let showLinkHistory = $state(false);
  let linkHistory = $state<string[]>([]);
  let bannerCache = $state<Record<string, string>>({});
  let detailCache = $state<Record<string, { viTitle?: string; runtime?: number }>>({});

  const tabs = [
    { value: "all", label: "Tất cả" },
    { value: "downloading", label: "Đang tải" },
    { value: "queued", label: "Đang chờ" },
    { value: "completed", label: "Hoàn thành" },
    { value: "failed", label: "Lỗi" },
  ] as const;

  const showcaseQueue = [
    { title: "Dune Part Two", quality: "4K HDR", progress: 78, speed: "12.4 MB/s" },
    { title: "Oppenheimer", quality: "REMUX", progress: 46, speed: "8.9 MB/s" },
    { title: "Interstellar", quality: "BluRay", progress: 24, speed: "5.2 MB/s" },
  ];

  const filtered = $derived(activeTab === "all" ? downloads : downloads.filter((item) => item.state === activeTab));
  const activeSpeed = $derived(formatBytes(stats.total_speed ?? downloads.reduce((sum, item) => sum + parseSpeed(item.speed), 0)) + "/s");
  const downloadablePreviewItems = $derived((preview?.items || []).filter((item) => !item.is_directory));
  const selectedPreviewItems = $derived(downloadablePreviewItems.filter((item) => selectedUrls.has(item.url)));
  const selectedPreviewSize = $derived(selectedPreviewItems.reduce((sum, item) => sum + (item.size || 0), 0));
  const confirmLabels = $derived(language === "vi" ? {
    title: "Xác nhận tải xuống?",
    count: "file sẽ được thêm vào queue.",
    size: "Dung lượng",
    cancel: "Huỷ",
    download: "Tải xuống",
    sending: "Đang gửi vào NAS...",
    aria: "Xác nhận tải xuống mobile",
  } : {
    title: "Confirm download?",
    count: "file(s) will be added to queue.",
    size: "Size",
    cancel: "Cancel",
    download: "Download",
    sending: "Sending to NAS...",
    aria: "Confirm mobile download",
  });

  onMount(() => {
    loadLanguage();
    loadLinkHistory();
    void refreshAll();
    const refreshTimer = window.setInterval(refreshAll, 5000);

    const handleConfirmTap = (event: Event) => {
      const target = event.target as HTMLElement | null;
      const button = target?.closest?.("[data-confirm-download]");
      if (!button) return;
      event.preventDefault();
      event.stopPropagation();
      triggerConfirmedDownload();
    };

    document.addEventListener("pointerup", handleConfirmTap, true);
    document.addEventListener("touchend", handleConfirmTap, true);
    const handleLanguage = (event: Event) => {
      const next = (event as CustomEvent).detail;
      if (next === "vi" || next === "en") language = next;
    };

    document.addEventListener("click", handleConfirmTap, true);
    window.addEventListener("fhub-language-change", handleLanguage);

    return () => {
      window.clearInterval(refreshTimer);
      document.removeEventListener("pointerup", handleConfirmTap, true);
      document.removeEventListener("touchend", handleConfirmTap, true);
      document.removeEventListener("click", handleConfirmTap, true);
      window.removeEventListener("fhub-language-change", handleLanguage);
    };
  });

  async function refreshAll() {
    await Promise.all([loadDownloads(), loadStats()]);
  }

  function loadLanguage() {
    try {
      const saved = localStorage.getItem("fhub-ui-language");
      if (saved === "vi" || saved === "en") language = saved;
    } catch {
      // localStorage may be unavailable.
    }
  }

  function loadLinkHistory() {
    try {
      const saved = JSON.parse(localStorage.getItem("fhub-link-history") || "[]");
      linkHistory = Array.isArray(saved) ? saved.filter((item) => typeof item === "string").slice(0, 12) : [];
    } catch {
      linkHistory = [];
    }
  }

  function rememberLink(url: string) {
    const clean = url.trim();
    if (!clean) return;
    linkHistory = [clean, ...linkHistory.filter((item) => item !== clean)].slice(0, 12);
    try {
      localStorage.setItem("fhub-link-history", JSON.stringify(linkHistory));
    } catch {
      // localStorage may be unavailable.
    }
  }

  function pickHistoryLink(url: string) {
    fshareLink = url;
    showLinkHistory = false;
    tick().then(() => document.querySelector<HTMLInputElement>(".link-panel input")?.focus());
  }

  function clearLinkHistory() {
    linkHistory = [];
    try {
      localStorage.removeItem("fhub-link-history");
    } catch {
      // localStorage may be unavailable.
    }
  }

  async function fetchWithTimeout(url: string, options: RequestInit = {}, timeoutMs = 30000) {
    const controller = new AbortController();
    const timer = window.setTimeout(() => controller.abort(), timeoutMs);
    try {
      return await fetch(url, { ...options, signal: controller.signal });
    } finally {
      window.clearTimeout(timer);
    }
  }

  async function loadDownloads() {
    try {
      const res = await fetch("/api/downloads", { credentials: "include" });
      if (!res.ok) throw new Error(await safeText(res));
      const data = await res.json();
      const tasks = Array.isArray(data?.tasks) ? data.tasks : Array.isArray(data?.downloads) ? data.downloads : Array.isArray(data) ? data : [];
      const previousBanners = new Map(downloads.filter((item) => item.banner).map((item) => [item.id, item.banner!]));
      const previousDetails = new Map(downloads.map((item) => [item.id, { viTitle: item.viTitle, runtime: item.runtime }]));
      downloads = tasks.map((task: ApiDownload, index: number) => normalizeDownload(task, index, previousBanners, previousDetails));
      void hydrateDownloadBanners(downloads);
      status = downloads.length ? `Đã đồng bộ ${downloads.length} task từ engine FHUB.` : "Queue đang trống. Dán link FShare để bắt đầu.";
    } catch (err) {
      downloads = [];
      status = `Không tải được queue: ${messageOf(err)}`;
    }
  }

  async function loadStats() {
    try {
      const res = await fetch("/api/engine/stats", { credentials: "include" });
      if (res.ok) stats = await res.json();
    } catch {
      // Stats are optional.
    }
  }

  function normalizeFshareUrl(value: string) {
    const clean = value.trim();
    if (!clean) return "";
    return /^https?:\/\//i.test(clean) ? clean : `https://${clean}`;
  }

  function isFolderPreview(value?: string) {
    return /fshare\.vn\/folder\//i.test(value || "");
  }

  function selectAllPreview() {
    selectedUrls = new Set(downloadablePreviewItems.map((item) => item.url));
  }

  function clearPreviewSelection() {
    selectedUrls = new Set();
  }

  function togglePreviewItem(url: string) {
    const next = new Set(selectedUrls);
    if (next.has(url)) next.delete(url);
    else next.add(url);
    selectedUrls = next;
  }

  async function checkLink() {
    const clean = normalizeFshareUrl(fshareLink);
    if (!clean || !/fshare\.vn\/(file|folder)\//i.test(clean)) {
      status = "Dán link FShare file hoặc folder hợp lệ trước.";
      return;
    }

    rememberLink(clean);
    checkLoading = true;
    preview = null;
    selectedUrls = new Set();
    status = "Đang check link FShare, chưa tải...";
    try {
      const res = await fetchWithTimeout("/api/downloads/preview-link", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ url: clean, recursive: true }),
      }, 35000);
      if (!res.ok) throw new Error(await safeText(res));
      preview = await res.json();
      selectAllPreview();
      const count = downloadablePreviewItems.length || preview?.file_count || 0;
      status = isFolderPreview(preview?.resolved_url)
        ? `Đã check thư mục: ${count} file · ${formatBytes(preview?.total_size || selectedPreviewSize)}. Chọn file rồi xác nhận tải.`
        : `Đã check file: ${preview?.folder_name || downloadablePreviewItems[0]?.name || "FShare file"}. Xác nhận nếu muốn tải.`;
    } catch (err) {
      status = err instanceof DOMException && err.name === "AbortError" ? "Check link quá lâu, đã tự dừng. Thử lại hoặc kiểm tra link FShare." : `Check link thất bại: ${messageOf(err)}`;
    } finally {
      checkLoading = false;
    }
  }

  function confirmPreviewFromPanel() {
    openDownloadConfirm();
  }

  function openDownloadConfirm() {
    const items = [...selectedPreviewItems];
    if (!items.length) {
      status = "Chưa chọn file nào để tải.";
      return;
    }
    pendingDownloadItems = items;
    pendingDownloadName = preview?.folder_name || (items.length === 1 ? items[0].name : `${items[0].name} + ${items.length - 1} file`);
    pendingDownloadSize = items.reduce((sum, item) => sum + (item.size || 0), 0);
    status = `Sẵn sàng thêm ${items.length} file vào queue. Bấm Download để xác nhận.`;
    showDownloadConfirm = true;
  }

  function triggerConfirmedDownload(event?: Event) {
    event?.preventDefault();
    event?.stopPropagation();
    if (confirmTapInProgress) return;
    if (!pendingDownloadItems.length) {
      status = "Chưa có file chờ xác nhận download.";
      return;
    }
    confirmTapInProgress = true;
    downloadLoading = true;
    status = "Đã nhận thao tác Download, đang gửi vào queue NAS...";
    void confirmPreviewDownload()
      .catch((err) => {
        showDownloadConfirm = false;
        status = `Không gửi được download: ${messageOf(err)}`;
      })
      .finally(() => {
        confirmTapInProgress = false;
        downloadLoading = false;
      });
  }

  function makeBatchId() {
    try {
      if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID();
    } catch {
      // Some mobile browsers expose crypto only in secure contexts.
    }
    return `fhub-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  }

  async function confirmPreviewDownload() {
    status = "Đã bấm xác nhận download, đang gửi vào queue NAS...";
    const itemsToDownload = [...pendingDownloadItems];
    if (!itemsToDownload.length) {
      status = "Chưa chọn file nào để tải.";
      return;
    }

    showDownloadConfirm = false;
    await tick();
    const batchId = makeBatchId();
    const batchName = pendingDownloadName || itemsToDownload[0]?.name || "FShare checked link";
    preview = null;
    selectedUrls = new Set();
    fshareLink = "";
    status = `Đã nhận lệnh download: đang thêm ${itemsToDownload.length} file vào queue...`;
    await tick();
    document.querySelector(".download-list")?.scrollIntoView({ behavior: "smooth", block: "start" });

    downloadLoading = true;
    try {
      const results = await Promise.allSettled(itemsToDownload.map((item) => fetchWithTimeout("/api/downloads", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          url: item.url,
          filename: item.name,
          category: "fshare",
          priority: "NORMAL",
          batch_id: batchId,
          batch_name: batchName,
        }),
      }, 15000)));

      const failed = results.filter((result) => result.status === "rejected" || (result.status === "fulfilled" && !result.value.ok));
      if (failed.length === itemsToDownload.length) {
        const first = failed[0];
        if (first?.status === "fulfilled") throw new Error(await first.value.text() || "Không thêm được file nào vào queue");
        throw new Error(first?.reason?.message || "Không thêm được file nào vào queue");
      }
      status = failed.length
        ? `Đã thêm ${itemsToDownload.length - failed.length}/${itemsToDownload.length} file vào queue, ${failed.length} file lỗi.`
        : `Đã thêm ${itemsToDownload.length} file vào queue FHUB.`;
      pendingDownloadItems = [];
      pendingDownloadName = "";
      pendingDownloadSize = 0;
      confirmTapInProgress = false;
      await refreshAll();
    } catch (err) {
      status = err instanceof DOMException && err.name === "AbortError" ? "Thêm download quá lâu, đã tự dừng. Kiểm tra queue rồi thử lại." : `Thêm download thất bại: ${messageOf(err)}`;
      await refreshAll();
    } finally {
      confirmTapInProgress = false;
      downloadLoading = false;
    }
  }


  async function toggle(item: QueueItem) {
    const action = item.state === "paused" ? "resume" : "pause";
    status = action === "pause" ? `Đang tạm dừng ${item.code}...` : `Đang tiếp tục ${item.code}...`;
    try {
      const res = await fetch(`/api/downloads/${encodeURIComponent(item.id)}/${action}`, { method: "POST", credentials: "include" });
      if (!res.ok) throw new Error(await safeText(res));
      await refreshAll();
    } catch (err) {
      status = `Không đổi được trạng thái ${item.code}: ${messageOf(err)}`;
    }
  }

  async function remove(item: QueueItem) {
    status = `Đang xóa ${item.code}...`;
    try {
      const res = await fetch(`/api/downloads/${encodeURIComponent(item.id)}`, { method: "DELETE", credentials: "include" });
      if (!res.ok) throw new Error(await safeText(res));
      await refreshAll();
    } catch (err) {
      status = `Không xóa được ${item.code}: ${messageOf(err)}`;
    }
  }

  function normalizeDownload(task: ApiDownload, index: number, previousBanners = new Map<string, string>(), previousDetails = new Map<string, { viTitle?: string; runtime?: number }>()): QueueItem {
    const title = task.tmdb_title || task.title || task.filename || task.name || task.batch_name || task.url || "FHUB Download";
    const id = String(task.id ?? task.url ?? `${title}-${index}`);
    const state = normalizeState(task.state || task.status);
    const mediaType = normalizeMediaType(task.media_type || task.category);
    const tmdbId = task.tmdb_id ? String(task.tmdb_id) : undefined;
    const cacheKey = tmdbId ? `${mediaType}:${tmdbId}` : "";
    const cachedDetail = cacheKey ? detailCache[cacheKey] : undefined;
    const previousDetail = previousDetails.get(id);
    return {
      id,
      title,
      owner: task.owner || "FHUB",
      size: formatDownloadSize(task),
      speed: typeof task.speed === "string" ? task.speed : `${formatBytes(task.speed)}/s`,
      progress: clampPercent(Number(task.progress ?? 0)),
      code: codeFor(title),
      state,
      banner: (cacheKey && bannerCache[cacheKey]) || previousBanners.get(id) || undefined,
      tmdbId,
      mediaType,
      downloadedBytes: Number(task.downloaded_bytes ?? 0),
      totalBytes: Number(task.total_bytes ?? 0),
      sourceUrl: task.url,
      outputPath: task.output_path || task.file_path || task.save_path,
      viTitle: cachedDetail?.viTitle || previousDetail?.viTitle,
      runtime: cachedDetail?.runtime || previousDetail?.runtime,
    };
  }

  function stateLabel(state: QueueItem["state"]) {
    return state === "completed" ? "Hoàn thành"
      : state === "downloading" ? "Đang tải"
      : state === "paused" ? "Tạm dừng"
      : state === "failed" ? "Lỗi"
      : "Đang chờ";
  }

  function stateIcon(state: QueueItem["state"]) {
    return state === "completed" ? "check_circle"
      : state === "downloading" ? "downloading"
      : state === "paused" ? "pause_circle"
      : state === "failed" ? "error"
      : "schedule";
  }

  function detailLine(item: QueueItem) {
    const pieces = [stateLabel(item.state)];
    if (item.size && item.size !== "0 B / 0 B") pieces.push(item.size);
    if (item.owner && item.owner !== "FHUB") pieces.push(item.owner);
    return pieces.join(" · ");
  }

  function movieInfoLine(item: QueueItem) {
    const pieces = [];
    const vi = item.viTitle && item.viTitle !== item.title ? item.viTitle : "";
    if (vi) pieces.push(vi);
    if (item.runtime && item.runtime > 0) pieces.push(formatRuntime(item.runtime));
    if (item.size && item.size !== "0 B / 0 B") pieces.push(item.size);
    return pieces.join(" · ") || (item.state === "completed" ? "Đã tải xong" : detailLine(item));
  }

  function completedDetailLine(item: QueueItem) {
    return movieInfoLine(item);
  }

  function formatRuntime(minutes: number) {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    if (h && m) return `${h}h ${m}m`;
    if (h) return `${h}h`;
    return `${m}m`;
  }

  function redownload(item: QueueItem) {
    if (!item.sourceUrl) {
      status = "Task này chưa có link gốc để tải lại.";
      return;
    }
    fshareLink = item.sourceUrl;
    status = `Đã đưa link tải lại của ${item.title} vào ô nhập.`;
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function canToggle(item: QueueItem) {
    return item.state === "downloading" || item.state === "paused" || item.state === "queued";
  }

  function formatDownloadSize(task: ApiDownload) {
    const rawSize = task.size;
    if (typeof rawSize === "number") return formatBytes(rawSize);
    if (typeof rawSize === "string" && rawSize.trim()) {
      const clean = rawSize.trim();
      if (/^\d+$/.test(clean)) return formatBytes(Number(clean));
      return clean;
    }
    const downloaded = Number(task.downloaded_bytes ?? 0);
    const total = Number(task.total_bytes ?? 0);
    if (total > 0) return formatBytes(total);
    if (downloaded > 0) return formatBytes(downloaded);
    return "";
  }

  function normalizeMediaType(value?: string) {
    const clean = String(value || "movie").toLowerCase();
    return clean === "tv" || clean === "series" ? "tv" : "movie";
  }

  async function hydrateDownloadBanners(items: QueueItem[]) {
    const missing = items
      .filter((item) => item.tmdbId && !item.banner && !bannerCache[`${item.mediaType}:${item.tmdbId}`])
      .map((item) => ({ key: `${item.mediaType}:${item.tmdbId}`, id: item.tmdbId!, mediaType: item.mediaType || "movie" }));
    const unique = Array.from(new Map(missing.map((item) => [item.key, item])).values());
    if (!unique.length) return;

    const entries = await Promise.all(unique.map(async (item) => {
      try {
        const res = await fetch(`/api/tmdb/${item.mediaType}/${item.id}`, { credentials: "include" });
        if (!res.ok) return [item.key, ""] as const;
        const detail = await res.json();
        const path = detail.poster_path || detail.backdrop_path;
        const viTitle = detail.title_vi || detail.vietnamese_title || detail.vi_title || detail.local_title || detail.title || detail.name || "";
        const runtime = Number(detail.runtime || detail.episode_run_time?.[0] || 0);
        return [item.key, path ? `https://image.tmdb.org/t/p/w342${path}` : "", { viTitle, runtime }] as const;
      } catch {
        return [item.key, "", {}] as const;
      }
    }));

    const nextCache = { ...bannerCache };
    const nextDetails = { ...detailCache };
    for (const [key, value, detail] of entries) {
      if (value || !nextCache[key]) nextCache[key] = value;
      if (detail && (detail.viTitle || detail.runtime)) nextDetails[key] = detail;
    }
    bannerCache = nextCache;
    detailCache = nextDetails;
    downloads = downloads.map((item) => {
      const key = item.tmdbId ? `${item.mediaType}:${item.tmdbId}` : "";
      if (!key) return item;
      return { ...item, banner: nextCache[key] || item.banner, viTitle: nextDetails[key]?.viTitle || item.viTitle, runtime: nextDetails[key]?.runtime || item.runtime };
    });
  }

  function normalizeState(value?: string): QueueItem["state"] {
    const state = String(value || "queued").toLowerCase();
    if (["complete", "completed", "done", "finished"].includes(state)) return "completed";
    if (["pause", "paused"].includes(state)) return "paused";
    if (["download", "downloading", "active", "running"].includes(state)) return "downloading";
    if (["fail", "failed", "error"].includes(state)) return "failed";
    return "queued";
  }

  function codeFor(value: string) {
    return value.replace(/[^a-z0-9]/gi, "").slice(0, 4).toUpperCase() || "FHUB";
  }

  function clampPercent(value: number) {
    if (!Number.isFinite(value)) return 0;
    return Math.max(0, Math.min(100, value));
  }

  function formatBytes(value?: number | string) {
    const bytes = Number(value ?? 0);
    if (!bytes || !Number.isFinite(bytes)) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const idx = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    return `${(bytes / Math.pow(1024, idx)).toFixed(idx ? 1 : 0)} ${units[idx]}`;
  }

  function parseSpeed(value: string) {
    const match = value.match(/([\d.]+)\s*(B|KB|MB|GB|TB)/i);
    if (!match) return 0;
    const units = ["B", "KB", "MB", "GB", "TB"];
    const index = units.indexOf(match[2].toUpperCase());
    return Number(match[1]) * Math.pow(1024, Math.max(0, index));
  }

  async function safeText(res: Response) {
    const text = await res.text();
    return text || `HTTP ${res.status}`;
  }

  function messageOf(err: unknown) {
    return err instanceof Error ? err.message : "lỗi không xác định";
  }
</script>

<div class="downloads-screen">
  <section class="download-hero">
    <div class="hero-copy hero-showcase" aria-label="FHub download cockpit">
      <div class="showcase-orbit">
        <span class="orbit-ring"></span>
        <span class="material-icons main-glyph">movie_filter</span>
        <span class="mini-glyph glyph-a material-icons">cloud_download</span>
        <span class="mini-glyph glyph-b material-icons">dns</span>
        <span class="mini-glyph glyph-c material-icons">bolt</span>
      </div>
      <div class="showcase-grid" aria-hidden="true">
        <span></span><span></span><span></span><span></span><span></span><span></span>
      </div>
      <div class="hero-actions showcase-actions">
        <a href="/discover"><span class="material-icons">movie_filter</span> Lấy link phim hot</a>
      </div>
    </div>

    <div class="hero-visual" aria-label="Download queue preview">
      <div class="speed-orb">
        <span class="online-dot"></span>
        <strong>{activeSpeed}</strong>
        <small>{stats.active_downloads ?? downloads.filter((item) => item.state === "downloading").length} active · {downloads.length} task</small>
      </div>
      <div class="queue-stack">
        {#each showcaseQueue as item, index}
          <article style={`--p:${item.progress}%;--d:${index * 120}ms`}>
            <div>
              <strong>{item.title}</strong>
              <small>{item.quality} · {item.speed}</small>
            </div>
            <span>{item.progress}%</span>
            <i></i>
          </article>
        {/each}
      </div>
      <div class="download-beam"></div>
    </div>
  </section>

  <div class="link-shell">
    <form class="link-panel" onsubmit={(event) => { event.preventDefault(); checkLink(); }}>
      <span class="material-icons">link</span>
      <input bind:value={fshareLink} placeholder="Dán link FShare hoặc folder..." disabled={checkLoading || downloadLoading} />
      <button type="submit" class="check-link-button" disabled={checkLoading || downloadLoading}><span class="material-icons">rule</span>{checkLoading ? "Đang check..." : "Check link"}</button>
      <button class="history-button" type="button" aria-label="Lịch sử link" title="Lịch sử link" onclick={() => showLinkHistory = !showLinkHistory} disabled={!linkHistory.length}>
        <span class="material-icons">history</span>
      </button>
    </form>
    {#if showLinkHistory && linkHistory.length}
      <div class="link-history-panel">
        <div class="history-head"><strong>Lịch sử link</strong><button type="button" onclick={clearLinkHistory}>Xóa</button></div>
        {#each linkHistory as url}
          <button type="button" class="history-item" onclick={() => pickHistoryLink(url)} title={url}>
            <span class="material-icons">history</span>
            <span>{url}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if preview}
    <section class="preview-panel">
      <div class="preview-summary">
        <div>
          <h2>{preview.folder_name || downloadablePreviewItems[0]?.name || "FShare link"}</h2>
          <p>{isFolderPreview(preview.resolved_url) ? "Thư mục FShare" : "File FShare"} · {downloadablePreviewItems.length} file · {formatBytes(preview.total_size || selectedPreviewSize)}</p>
        </div>
        <div class="preview-tools">
          <button type="button" onclick={selectAllPreview}>Chọn hết</button>
          <button type="button" onclick={clearPreviewSelection}>Bỏ chọn</button>
        </div>
      </div>
      <div class="preview-files">
        {#each downloadablePreviewItems as item}
          <label class="preview-file">
            <input type="checkbox" checked={selectedUrls.has(item.url)} onchange={() => togglePreviewItem(item.url)} />
            <span class="material-icons">movie</span>
            <div><strong>{item.name}</strong><small>{item.quality || "FShare"} · {formatBytes(item.size)}</small></div>
          </label>
        {/each}
      </div>
      <div class="preview-confirm">
        <span>{selectedPreviewItems.length} file đã chọn · {formatBytes(selectedPreviewSize)}</span>
        <button type="button" onclick={confirmPreviewFromPanel} disabled={checkLoading || downloadLoading || !selectedPreviewItems.length}><span class="material-icons">download</span>Download</button>
      </div>
    </section>
  {/if}

  {#if !status.startsWith("Đã đồng bộ")}
    <p class="status-line">{status}</p>
  {/if}

  <section class="tabs">
    {#each tabs as tab}
      <button type="button" class:active={activeTab === tab.value} onclick={() => activeTab = tab.value}>{tab.label}</button>
    {/each}
    <button type="button" onclick={refreshAll}><span class="material-icons">refresh</span>Làm mới</button>
  </section>

  <section class="download-list">
    {#if filtered.length}
      {#each filtered as item}
        <article class="download-card" class:active={item.state === "downloading"} class:completed={item.state === "completed"} class:failed={item.state === "failed"}>
          <div class="poster" class:has-image={!!item.banner}>
            {#if item.banner}
              <img src={item.banner} alt={`Banner ${item.title}`} loading="lazy" />
            {:else}
              <span>{item.code}</span>
            {/if}
            {#if item.state !== "completed"}<small>{Math.round(item.progress)}%</small>{/if}
          </div>
          <div class="download-info">
            <div class="title-row"><strong title={item.title}>{item.title}</strong></div>
            <div class="download-meta">
              <span class="state-pill {item.state}"><span class="material-icons">{stateIcon(item.state)}</span>{stateLabel(item.state)}</span>
              {#if item.state === "completed"}
                <span>{completedDetailLine(item)}</span>
              {:else if item.speed && item.speed !== "0 B/s" && item.state === "downloading"}
                <span>{item.speed} · {movieInfoLine(item)}</span>
              {:else}
                <span>{detailLine(item)}</span>
              {/if}
            </div>
            <div class="progress" class:done={item.state === "completed"} class:failed={item.state === "failed"}><i style={`width:${item.progress}%`}></i></div>
          </div>
          <div class="card-actions">
            {#if canToggle(item)}
              <button type="button" aria-label={item.state === "paused" ? "Resume" : "Pause"} title={item.state === "paused" ? "Tiếp tục" : "Tạm dừng"} onclick={() => toggle(item)}><span class="material-icons">{item.state === "paused" ? "play_arrow" : "pause"}</span></button>
            {:else if item.state === "completed" && item.sourceUrl}
              <button type="button" class="redownload-action" aria-label="Redownload" title="Tải lại link này" onclick={() => redownload(item)}><span class="material-icons">replay</span></button>
            {:else if item.state === "failed"}
              <button type="button" aria-label="Retry" title="Thử lại" onclick={() => toggle(item)}><span class="material-icons">restart_alt</span></button>
            {/if}
            <button type="button" class="delete-action" aria-label="Delete" title="Xóa khỏi danh sách" onclick={() => remove(item)}><span class="material-icons">delete</span></button>
          </div>
        </article>
      {/each}
    {:else}
      <div class="empty-state">
        <span class="material-icons">download_for_offline</span>
        <strong>Không có task trong bộ lọc này.</strong>
        <p>Dán link FShare ở trên để thêm task mới, hoặc bấm Làm mới để đồng bộ engine.</p>
      </div>
    {/if}
  </section>
</div>

{#if showDownloadConfirm}
  <div class="confirm-backdrop" role="presentation">
    <div class="confirm-modal compact" role="dialog" aria-modal="true" aria-label="Xác nhận download">
      <div class="confirm-top">
        <span class="material-icons modal-icon">download_for_offline</span>
        <div><h2>{confirmLabels.title}</h2><p>{pendingDownloadItems.length} {confirmLabels.count}</p></div>
      </div>
      <div class="confirm-file-list" aria-label="Danh sách file sẽ download">
        <div>
          {#each pendingDownloadItems as item, index}
            <strong title={item.name}>{index + 1}. {item.name}</strong>
          {/each}
        </div>
      </div>
      <div class="confirm-box"><span>{confirmLabels.size}</span><strong>{formatBytes(pendingDownloadSize)}</strong></div>
      <div class="modal-actions">
        <button type="button" onclick={() => { showDownloadConfirm = false; pendingDownloadItems = []; pendingDownloadName = ""; pendingDownloadSize = 0; status = language === "vi" ? "Đã huỷ tải xuống." : "Download cancelled."; }} disabled={false}>{confirmLabels.cancel}</button>
        <button
          type="button"
          class="danger-confirm"
          aria-label="Xác nhận tải vào NAS"
          data-confirm-download="true"
          onclick={triggerConfirmedDownload}
          disabled={!pendingDownloadItems.length}
        >{downloadLoading || confirmTapInProgress ? confirmLabels.sending : confirmLabels.download}</button>
      </div>
    </div>
    <div class="mobile-confirm-dock" role="group" aria-label={confirmLabels.aria}>
      <button
        type="button"
        class="dock-download"
        data-confirm-download="true"
        onclick={triggerConfirmedDownload}
        disabled={!pendingDownloadItems.length}
      >{downloadLoading || confirmTapInProgress ? confirmLabels.sending : confirmLabels.download}</button>
      <button type="button" class="dock-cancel" onclick={() => { showDownloadConfirm = false; pendingDownloadItems = []; pendingDownloadName = ""; pendingDownloadSize = 0; status = language === "vi" ? "Đã huỷ tải xuống." : "Download cancelled."; }}>{confirmLabels.cancel}</button>
    </div>
  </div>
{/if}

<style>
  .downloads-screen{display:grid;gap:1rem;max-width:1280px;margin:0 auto}.download-hero{position:relative;min-height:330px;display:grid;grid-template-columns:minmax(0,1fr) minmax(330px,440px);align-items:center;gap:clamp(1rem,3vw,2rem);padding:clamp(1.35rem,4vw,2.6rem);border-radius:22px;background:radial-gradient(circle at 78% 16%,rgba(56,189,248,.24),transparent 30%),linear-gradient(135deg,rgba(124,58,237,.28),transparent 43%),linear-gradient(180deg,rgba(24,28,42,.96),rgba(8,10,18,.98));border:1px solid rgba(56,189,248,.46);box-shadow:0 24px 70px rgba(0,0,0,.34);overflow:hidden}.download-hero:before{content:"";position:absolute;inset:auto -12% -45% 12%;height:240px;background:linear-gradient(90deg,transparent,rgba(248,193,74,.16),rgba(56,189,248,.18),transparent);filter:blur(28px);transform:rotate(-8deg)}.hero-copy,.hero-visual{position:relative;z-index:1}.eyebrow{color:#c4b5fd;font-size:.72rem;font-weight:950;letter-spacing:.16em;text-transform:uppercase}h1{max-width:760px;margin:.45rem 0;font-size:clamp(2.25rem,5vw,5rem);line-height:.92;letter-spacing:-.055em}p,small,.status-line{color:#aab4c3}.hero-copy p{max-width:640px;font-size:1rem}.hero-actions{display:flex;flex-wrap:wrap;gap:.75rem;margin-top:1.2rem}.hero-actions a{min-height:46px;padding:0 1rem;border-radius:14px;display:inline-flex;align-items:center;gap:.45rem;text-decoration:none;font-weight:900;color:#080a12;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.hero-actions a.ghost{color:#f8fafc;background:rgba(255,255,255,.06);border:1px solid rgba(148,163,184,.2)}.hero-showcase{min-height:250px;display:grid;place-items:center;align-content:center;gap:1rem}.showcase-orbit{position:relative;width:min(320px,58vw);aspect-ratio:1;display:grid;place-items:center;border-radius:999px;background:radial-gradient(circle at 50% 50%,rgba(248,193,74,.22),transparent 26%),radial-gradient(circle at 32% 28%,rgba(167,139,250,.34),transparent 30%),radial-gradient(circle at 70% 66%,rgba(56,189,248,.22),transparent 34%);box-shadow:inset 0 1px 0 rgba(255,255,255,.1),0 28px 90px rgba(0,0,0,.34)}.orbit-ring{position:absolute;inset:10%;border:1px solid rgba(255,255,255,.14);border-radius:999px;box-shadow:0 0 0 22px rgba(255,255,255,.025),0 0 0 46px rgba(56,189,248,.035);animation:orbit-pulse 3.8s ease-in-out infinite}.main-glyph{width:110px;height:110px;display:grid;place-items:center;border-radius:32px;color:#080a12!important;background:linear-gradient(135deg,#f8c14a,#fb7185 46%,#a78bfa);font-size:3.6rem!important;box-shadow:0 24px 60px rgba(248,193,74,.2),0 18px 50px rgba(167,139,250,.22);transform:rotate(-6deg)}.mini-glyph{position:absolute;width:54px;height:54px;display:grid;place-items:center;border:1px solid rgba(255,255,255,.14);border-radius:18px;color:#e0f2fe!important;background:linear-gradient(180deg,rgba(15,23,42,.9),rgba(8,12,22,.72));box-shadow:0 16px 38px rgba(0,0,0,.28);backdrop-filter:blur(14px)}.glyph-a{top:11%;right:18%}.glyph-b{left:10%;bottom:22%;color:#fde68a!important}.glyph-c{right:12%;bottom:16%;color:#c4b5fd!important}.showcase-grid{width:min(420px,100%);display:grid;grid-template-columns:repeat(6,1fr);gap:.42rem;opacity:.78}.showcase-grid span{height:7px;border-radius:999px;background:linear-gradient(90deg,rgba(56,189,248,.18),rgba(248,193,74,.44),rgba(167,139,250,.2));animation:beam-flow 8s linear infinite}.showcase-grid span:nth-child(even){opacity:.55;transform:translateY(7px)}.showcase-actions{margin-top:.25rem;justify-content:center}@keyframes orbit-pulse{0%,100%{transform:scale(1);opacity:.78}50%{transform:scale(1.05);opacity:1}}.hero-visual{display:grid;gap:.8rem}.speed-orb{justify-self:end;display:grid;gap:.28rem;min-width:210px;padding:1rem;border-radius:20px;background:linear-gradient(180deg,rgba(23,46,77,.92),rgba(13,21,37,.82));border:1px solid rgba(56,189,248,.22);box-shadow:0 20px 70px rgba(56,189,248,.12)}.speed-orb strong{color:#f8c14a;font-size:2.1rem;line-height:1}.online-dot{width:10px;height:10px;border-radius:50%;background:#38bdf8;box-shadow:0 0 18px rgba(56,189,248,.75)}.queue-stack{display:grid;gap:.7rem}.queue-stack article{position:relative;display:grid;grid-template-columns:minmax(0,1fr) auto;gap:.7rem;padding:.9rem 1rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.95),rgba(12,17,30,.88));border:1px solid rgba(148,163,184,.18);box-shadow:0 18px 42px rgba(0,0,0,.26);animation:float-card 3.8s ease-in-out infinite;animation-delay:var(--d)}.queue-stack article strong{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.queue-stack article span{color:#f8c14a;font-weight:950}.queue-stack article i{grid-column:1/-1;height:7px;border-radius:999px;background:linear-gradient(90deg,#38bdf8,var(--p),rgba(148,163,184,.15) var(--p));box-shadow:0 0 22px rgba(56,189,248,.24)}.download-beam{height:52px;border-radius:18px;background:repeating-linear-gradient(90deg,rgba(56,189,248,.08) 0 22px,rgba(248,193,74,.14) 22px 38px,rgba(167,139,250,.1) 38px 62px);mask-image:linear-gradient(90deg,transparent,#000 12%,#000 88%,transparent);animation:beam-flow 8s linear infinite}.link-shell{position:relative;display:grid;gap:.55rem}.link-panel{display:grid;grid-template-columns:24px minmax(0,1fr) minmax(128px,180px) 44px;gap:.65rem;align-items:center;min-height:64px;padding:.6rem .75rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.92),rgba(10,14,24,.78));border:1px solid rgba(148,163,184,.16)}.link-panel input{min-width:0;border:0;outline:0;background:transparent;color:#f8fafc;font-size:.98rem}.check-link-button{width:100%;padding:0 .85rem;white-space:nowrap}.history-button{width:44px;min-width:44px;padding:0;border-radius:14px}.history-button:disabled{opacity:.34}.link-history-panel{position:absolute;top:calc(100% + .45rem);left:0;right:0;z-index:20;display:grid;gap:.42rem;max-height:290px;overflow:auto;padding:.65rem;border:1px solid rgba(148,163,184,.18);border-radius:18px;background:linear-gradient(180deg,rgba(15,20,34,.98),rgba(7,10,18,.98));box-shadow:0 24px 70px rgba(0,0,0,.38)}.history-head{display:flex;align-items:center;justify-content:space-between;gap:.75rem;padding:.15rem .15rem .35rem;color:#f8fafc}.history-head button{min-height:32px;padding:0 .65rem;border-radius:10px;color:#fecaca;background:rgba(127,29,29,.22)}.history-item{width:100%;display:grid;grid-template-columns:24px minmax(0,1fr);justify-content:start;text-align:left;min-height:42px;padding:0 .65rem}.history-item span:last-child{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.material-icons{color:#c4b5fd}button{cursor:pointer;border:1px solid rgba(148,163,184,.16);color:#f8fafc;background:rgba(255,255,255,.055);border-radius:12px;min-height:44px;padding:0 1rem;font-weight:850;display:inline-flex;align-items:center;justify-content:center;gap:.45rem}button:disabled{opacity:.62;cursor:not-allowed}.link-panel button,.tabs .active{background:linear-gradient(135deg,#f8c14a,#a78bfa);color:#080a12;border:0}.status-line{margin:0}.tabs{display:flex;gap:.65rem;overflow-x:auto}.download-list{display:grid;grid-template-columns:repeat(auto-fit,minmax(360px,1fr));gap:.85rem}.download-card{display:grid;grid-template-columns:82px minmax(0,1fr) 46px;gap:.9rem;align-items:center;padding:.8rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.92),rgba(10,14,24,.78));border:1px solid rgba(148,163,184,.16);box-shadow:0 18px 48px rgba(0,0,0,.26)}.download-card.active{border-color:rgba(167,139,250,.35)}.poster{width:82px;height:108px;border-radius:14px;background:linear-gradient(135deg,rgba(56,189,248,.2),transparent 48%),linear-gradient(180deg,rgba(31,41,64,.96),rgba(9,13,23,.96));display:grid;place-items:center;color:#f8c14a;font-weight:950;position:relative;overflow:hidden;box-shadow:inset 0 1px 0 rgba(255,255,255,.08),0 14px 34px rgba(0,0,0,.32)}.poster img{width:100%;height:100%;object-fit:cover;display:block}.poster.has-image:after{content:"";position:absolute;inset:0;background:linear-gradient(180deg,rgba(2,6,23,.08),rgba(2,6,23,.46))}.poster span{position:relative;z-index:1;font-size:1.28rem;letter-spacing:.02em}.poster small{position:absolute;top:8px;right:8px;z-index:2;color:#c4b5fd;background:rgba(8,10,18,.62);border:1px solid rgba(196,181,253,.22);border-radius:999px;padding:.1rem .32rem;font-weight:950}.title-row{display:flex;justify-content:space-between;gap:1rem}.title-row strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.title-row span{color:#c4b5fd;font-weight:950;text-transform:uppercase}.download-info p{margin:.25rem 0}.progress{height:7px;border-radius:999px;overflow:hidden;background:rgba(148,163,184,.14);margin-top:.65rem}.progress i{display:block;height:100%;border-radius:inherit;background:linear-gradient(90deg,#38bdf8,#a78bfa,#f8c14a)}.card-actions{display:grid;gap:.45rem}.card-actions button{width:44px;height:44px;padding:0}.empty-state{grid-column:1/-1;display:grid;place-items:center;text-align:center;min-height:220px;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.92),rgba(10,14,24,.78));border:1px solid rgba(148,163,184,.16)}.preview-panel{display:grid;gap:.75rem;padding:.85rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.94),rgba(10,14,24,.82));border:1px solid rgba(248,193,74,.18)}.preview-summary,.preview-confirm{display:flex;align-items:center;justify-content:space-between;gap:.8rem}.preview-summary h2{margin:.18rem 0;color:#fff;font-size:1.25rem}.preview-tools{display:flex;gap:.45rem}.preview-files{display:grid;gap:.5rem;max-height:330px;overflow:auto}.preview-file{display:grid;grid-template-columns:auto 34px minmax(0,1fr);align-items:center;gap:.6rem;padding:.62rem;border:1px solid rgba(148,163,184,.13);border-radius:14px;background:rgba(255,255,255,.045)}.preview-file .material-icons{width:34px;height:34px;display:grid;place-items:center;border-radius:11px;color:#080a12;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.preview-file strong,.preview-file small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.preview-file small{margin-top:.16rem;color:#8ea0b5}.preview-confirm{padding-top:.2rem}.preview-confirm span{color:#cbd5e1;font-weight:850}.preview-confirm button{color:#080a12;border:0;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.confirm-backdrop{pointer-events:auto;position:fixed;inset:0;z-index:9999;display:grid;place-items:center;padding:1rem;background:rgba(2,6,23,.68);backdrop-filter:blur(10px)}.confirm-modal{pointer-events:auto;isolation:isolate;width:min(430px,100%);display:grid;gap:.85rem;padding:1.15rem;border:1px solid rgba(248,193,74,.22);border-radius:22px;background:linear-gradient(180deg,rgba(20,26,42,.98),rgba(8,12,22,.96));box-shadow:0 28px 80px rgba(0,0,0,.45)}.modal-icon{width:54px;height:54px;display:grid;place-items:center;border-radius:18px;color:#080a12;background:linear-gradient(135deg,#f8c14a,#a78bfa);font-size:1.8rem}.confirm-modal h2{font-size:1.35rem}.confirm-box{display:flex;align-items:center;justify-content:space-between;gap:.8rem;padding:.75rem;border-radius:14px;background:rgba(255,255,255,.06);border:1px solid rgba(255,255,255,.08)}.confirm-box span{color:#aab4c3}.confirm-box strong{color:#f8c14a}.modal-actions{pointer-events:auto;position:relative;z-index:3;display:grid;grid-template-columns:1fr 1fr;gap:.65rem}.modal-actions button{width:100%}.danger-confirm{color:#080a12;border:0;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.confirm-modal.compact{gap:.72rem;padding:.9rem;border-radius:18px}.confirm-top{display:grid;grid-template-columns:42px minmax(0,1fr);align-items:center;gap:.7rem}.confirm-modal.compact .modal-icon{width:42px;height:42px;border-radius:14px;font-size:1.35rem}.confirm-modal.compact h2{margin:0;font-size:1.08rem}.confirm-modal.compact p{margin:.1rem 0 0;font-size:.82rem}.confirm-file,.confirm-box{display:grid;grid-template-columns:88px minmax(0,1fr);align-items:start;gap:.65rem;padding:.58rem .65rem;border-radius:13px;background:rgba(255,255,255,.055);border:1px solid rgba(255,255,255,.08)}.confirm-file-list{display:block;padding:.58rem .65rem;border-radius:13px;background:rgba(255,255,255,.055);border:1px solid rgba(255,255,255,.08)}.confirm-file span,.confirm-box span{color:#aab4c3;font-size:.82rem}.confirm-file strong,.confirm-file-list strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#f8fafc;font-size:.9rem}.confirm-file-list > div{min-width:0;display:grid;gap:.34rem;max-height:38vh;overflow:auto;padding-right:.15rem}.confirm-box{align-items:center}.confirm-box strong{justify-self:end;color:#f8c14a;font-size:.95rem}.confirm-modal.compact .modal-actions button{min-height:42px;border-radius:13px}.confirm-modal.compact .modal-actions{pointer-events:auto;position:relative;z-index:3;gap:.55rem}
.confirm-modal button{pointer-events:auto;cursor:pointer;position:relative;z-index:4;transform:translateZ(0)}
.danger-confirm{touch-action:manipulation;user-select:none;-webkit-user-select:none}

.mobile-confirm-dock{display:none}
@media (max-width: 720px){
  .confirm-backdrop{align-items:start;place-items:start stretch;padding:.75rem .75rem calc(92px + env(safe-area-inset-bottom));overflow:auto;touch-action:auto}
  .confirm-modal{width:100%;margin-top:10vh;max-height:calc(100vh - 130px);overflow:auto}
  .mobile-confirm-dock{position:fixed;left:0;right:0;bottom:0;z-index:10050;display:grid;grid-template-columns:1fr;gap:.65rem;padding:.75rem .85rem calc(.85rem + env(safe-area-inset-bottom));background:linear-gradient(180deg,rgba(8,12,22,.88),rgba(8,12,22,.98));border-top:1px solid rgba(248,193,74,.25);box-shadow:0 -18px 50px rgba(0,0,0,.45);pointer-events:auto;touch-action:manipulation}
  .mobile-confirm-dock button{min-height:54px;border-radius:16px;font-size:1rem;font-weight:950;pointer-events:auto;touch-action:manipulation;-webkit-tap-highlight-color:transparent}
  .dock-cancel{background:rgba(255,255,255,.07);color:#f8fafc;border:1px solid rgba(148,163,184,.2)}
  .dock-download{background:linear-gradient(135deg,#f8c14a,#a78bfa);color:#080a12;border:0}
}

@keyframes float-card{0%,100%{transform:translateY(0)}50%{transform:translateY(-5px)}}@keyframes beam-flow{from{background-position:0 0}to{background-position:240px 0}}@media(max-width:900px){.download-hero{grid-template-columns:1fr}.speed-orb{justify-self:start}.hero-visual{max-width:520px}}@media(max-width:720px){.download-hero{display:none}.link-panel{grid-template-columns:24px minmax(0,1fr) 44px}.link-panel .check-link-button{grid-column:1/3}.link-panel .history-button{grid-column:3/4}.preview-summary,.preview-confirm{align-items:stretch;flex-direction:column}.preview-tools{width:100%}.preview-tools button,.preview-confirm button{width:100%}.download-list{grid-template-columns:1fr}.download-card{grid-template-columns:74px minmax(0,1fr) 38px}.poster{width:74px;height:96px}.card-actions button{width:38px;height:38px}}


/* Richer download state cards */
.download-card.completed { border-color: rgba(34,197,94,.26); background: linear-gradient(135deg,rgba(34,197,94,.08),transparent 42%),linear-gradient(180deg,rgba(20,26,42,.92),rgba(10,14,24,.78)); }
.download-card.failed { border-color: rgba(248,113,113,.28); }
.download-meta { min-width:0; display:flex; flex-wrap:wrap; align-items:center; gap:.4rem .55rem; margin:.18rem 0 .05rem; color:rgba(226,232,240,.68); font-size:.78rem; font-weight:850; }
.download-meta > span:not(.state-pill) { min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.state-pill { display:inline-flex; align-items:center; gap:.28rem; min-height:24px; padding:0 .48rem; border-radius:999px; border:1px solid rgba(148,163,184,.16); background:rgba(255,255,255,.055); color:#cbd5e1; }
.state-pill .material-icons { font-size:1rem; color:inherit; }
.state-pill.completed { color:#86efac; border-color:rgba(34,197,94,.26); background:rgba(34,197,94,.1); }
.state-pill.downloading { color:#93c5fd; border-color:rgba(59,130,246,.24); background:rgba(59,130,246,.1); }
.state-pill.paused,.state-pill.queued { color:#fde68a; border-color:rgba(248,193,74,.24); background:rgba(248,193,74,.1); }
.state-pill.failed { color:#fecaca; border-color:rgba(248,113,113,.28); background:rgba(127,29,29,.16); }
.poster small.done { color:#052e16; border-color:rgba(134,239,172,.55); background:linear-gradient(135deg,#86efac,#d9f99d); }
.progress.done i { background:linear-gradient(90deg,#22c55e,#86efac); }
.progress.failed i { background:linear-gradient(90deg,#ef4444,#fb7185); }
.card-actions .success-action { color:#86efac; border-color:rgba(34,197,94,.24); background:rgba(34,197,94,.1); opacity:1; cursor:default; }
.card-actions .success-action .material-icons { color:#86efac; }

/* Mobile download card stability fixes */
.download-card { min-width: 0; overflow: hidden; }
.download-info { min-width: 0; overflow: hidden; }
.poster { flex: 0 0 auto; isolation: isolate; }
.poster img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center; }
.poster.has-image span { display: none; }
.poster:not(.has-image) span { max-width: 88%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: center; }
.poster small { z-index: 3; left: 50%; right: auto; transform: translateX(-50%); min-width: 52px; text-align: center; }
.title-row strong { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; white-space: normal; overflow: hidden; overflow-wrap: anywhere; }

@media(max-width:720px){
  .download-list{gap:.7rem;}
  .download-card{grid-template-columns:88px minmax(0,1fr); gap:.8rem; align-items:center; padding:.7rem; border-radius:20px;}
  .poster{width:88px;height:124px;border-radius:16px;}
  .download-info{align-self:stretch;display:grid;align-content:center;gap:.45rem;}
  .title-row strong{font-size:clamp(1.05rem,5.2vw,1.45rem);line-height:1.12;letter-spacing:-.035em;}
  .download-info p{margin:0;font-size:clamp(.86rem,4.2vw,1.08rem);}
  .progress{height:8px;margin-top:.1rem;}
  .card-actions{grid-column:1/-1;display:flex;justify-content:flex-end;gap:.45rem;margin-top:-.15rem;}
  .card-actions button{width:40px;height:36px;min-height:36px;border-radius:12px;}
}

@media(max-width:420px){
  .download-card{grid-template-columns:78px minmax(0,1fr);gap:.65rem;padding:.62rem;}
  .poster{width:78px;height:112px;border-radius:15px;}
  .poster small{min-width:46px;font-size:.72rem;}
  .title-row strong{font-size:clamp(1rem,5vw,1.25rem);}
}


/* Compact completed download cards */
.download-card.completed { grid-template-columns:82px minmax(0,1fr) 42px; align-items:start; }
.download-card.completed .download-info { padding-top:.08rem; }
.download-card.completed .card-actions { align-self:start; }
.download-card.completed .card-actions button { width:40px;height:40px;min-height:40px;border-radius:13px; }
.download-card.completed .progress { margin-top:.45rem; }
.download-card.completed .state-pill.completed { display:none; }
.download-card.completed .download-meta { margin-top:.32rem; }
.download-card.completed .download-meta > span:not(.state-pill) { font-size:.82rem;color:rgba(226,232,240,.74); }

@media(max-width:720px){
  .download-card.completed{grid-template-columns:88px minmax(0,1fr) 42px; align-items:start;}
  .download-card.completed .poster{width:88px;height:124px;}
  .download-card.completed .download-info{align-self:start;padding-top:.12rem;}
  .download-card.completed .card-actions{grid-column:auto;margin-top:0;justify-content:start;align-self:start;}
  .download-card.completed .card-actions button{width:40px;height:40px;min-height:40px;}
}
@media(max-width:420px){
  .download-card.completed{grid-template-columns:78px minmax(0,1fr) 40px;}
  .download-card.completed .poster{width:78px;height:112px;}
  .download-card.completed .card-actions button{width:38px;height:38px;min-height:38px;}
}


/* Compact movie info + redownload */
.download-card.completed .download-meta > span:not(.state-pill) { white-space:normal; display:-webkit-box; -webkit-line-clamp:2; line-clamp:2; -webkit-box-orient:vertical; }
.card-actions .redownload-action { color:#bae6fd; border-color:rgba(56,189,248,.24); background:rgba(56,189,248,.1); }
.card-actions .redownload-action .material-icons { color:#bae6fd; }
.card-actions .delete-action { color:#ddd6fe; }
.download-card.completed .card-actions { gap:.38rem; }
@media(max-width:720px){
  .download-card.completed{grid-template-columns:88px minmax(0,1fr) 40px;}
  .download-card.completed .card-actions{display:grid;grid-column:auto;gap:.36rem;}
  .download-card.completed .card-actions button{width:38px;height:38px;min-height:38px;}
}
@media(max-width:420px){
  .download-card.completed{grid-template-columns:78px minmax(0,1fr) 38px;}
  .download-card.completed .card-actions button{width:36px;height:36px;min-height:36px;}
}

</style>
