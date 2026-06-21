<script lang="ts">
  import { onMount } from "svelte";

  type Track = { id:string; title:string; folder_url:string; folder_code:string; enabled:boolean; check_interval_secs:number; poster_url?:string; last_checked_at?:string; last_error?:string; created_at?:string; updated_at:string; };
  type TrackItem = { id:string; file_name:string; file_url?:string; fshare_code:string; file_size:number; season?:number; episode?:number; status:string; download_id?:string; first_seen_at?:string; queued_at?:string; completed_at?:string; error_message?:string; auto_queued?:boolean; };
  type TrackDetail = Track & { items: TrackItem[] };

  let tracks = $state<Track[]>([]);
  let details = $state<Record<string, TrackDetail>>({});
  let expandedId = $state<string | null>(null);
  let pendingDeleteTrack = $state<Track | null>(null);
  let selectedActivityDay = $state<string>(dayKey(new Date()));
  let activityOpen = $state(true);
  let loading = $state(false);
  let status = $state("Đang tải Auto Track...");
  let language = $state<"vi" | "en">("vi");

  onMount(() => {
    loadLanguage();
    selectedActivityDay = dayKey(new Date());
    const handleLanguage = (event: Event) => {
      const next = (event as CustomEvent).detail;
      if (next === "vi" || next === "en") language = next;
    };
    window.addEventListener("fhub-language-change", handleLanguage);
    void loadTracks();
    return () => window.removeEventListener("fhub-language-change", handleLanguage);
  });

  const text = $derived(language === "vi" ? {
    loading: "Đang tải Auto Track...",
    empty: "Chưa có bộ nào được Auto Track.",
    loadFailed: "Không tải được Auto Track",
    noEnabled: "Không có Auto Track đang bật để kiểm tra.",
    checkingMany: (count: number) => `Đang kiểm tra ${count} bộ...`,
    checkedSome: (ok: number, total: number, failed: number) => `Đã kiểm tra ${ok}/${total} bộ, ${failed} bộ lỗi.`,
    checkedAll: (ok: number) => `Đã kiểm tra tất cả ${ok} bộ.`,
    scanning: (title: string) => `Đang quét ${title}...`,
    scanned: "Đã quét xong.",
    heroTitle: "Theo dõi phim bộ",
    autoEvery: (value: string) => `Tự track mỗi ${value}`,
    checkAll: "Kiểm tra tất cả",
    historyTitle: "Lịch sử Auto Track tải về 7 ngày",
    hide: "Ẩn",
    show: "Hiện",
    dayItems: (count: number) => `${count} mục`,
    noDayActivity: "Ngày này chưa có tập nào được Auto Track tải về.",
    checkNow: "Check ngay",
    deleteTrack: "Xoá Auto Track",
    filesInTrack: (count: number) => `${count} file trong Auto Track`,
    loadingDetail: "Đang tải chi tiết...",
    noTracks: "Vào trang Tải xuống, check link thư mục phim bộ rồi bấm Auto Track.",
    deleteTitle: "Xoá Auto Track",
    deleteBody: "FHUB sẽ ngừng theo dõi folder này. File đã tải và queue hiện có vẫn được giữ nguyên.",
    cancel: "Huỷ",
    delete: "Xoá",
    downloaded: "Đã tải xong",
    downloading: "Đang tải",
    queued: "Đang chờ",
    paused: "Tạm dừng",
    skipped: "Đã bỏ qua",
    failed: "Lỗi tải xuống",
    notDownloaded: "Chưa tải",
    completedAt: (value: string) => `Tải xong: ${value}`,
    queuedAt: (value: string) => `Thêm hàng chờ: ${value}`,
    seenAt: (value: string) => `Ghi nhận: ${value}`,
    noTime: "Chưa có thời gian",
    downloadEpisode: (name: string) => `Tải ${name}`,
    addEpisode: (name: string, title: string) => `Đang thêm ${name} của ${title} vào Downloads...`,
    addedEpisode: (title: string, name: string) => `Đã thêm ${title} · ${name} vào Downloads.`,
    addFailed: (name: string) => `Không thêm được ${name}.`,
    nextWorker: "Lần tới: khi worker chạy",
    lastNone: "Track gần nhất: chưa check nền",
    lastAt: (value: string) => `Track gần nhất: ${value}`,
    nextAt: (value: string) => `Lần tới: ${value}`,
    mins: (value: number) => `${value} phút`,
    hours: (value: string | number) => `${value} giờ`,
  } : {
    loading: "Loading Auto Track...",
    empty: "No Auto Track series yet.",
    loadFailed: "Could not load Auto Track",
    noEnabled: "No enabled Auto Track series to check.",
    checkingMany: (count: number) => `Checking ${count} series...`,
    checkedSome: (ok: number, total: number, failed: number) => `Checked ${ok}/${total} series, ${failed} failed.`,
    checkedAll: (ok: number) => `Checked all ${ok} series.`,
    scanning: (title: string) => `Scanning ${title}...`,
    scanned: "Scan complete.",
    heroTitle: "Auto Track series",
    autoEvery: (value: string) => `Auto track every ${value}`,
    checkAll: "Check all",
    historyTitle: "Auto Track download history — 7 days",
    hide: "Hide",
    show: "Show",
    dayItems: (count: number) => `${count} item${count === 1 ? "" : "s"}`,
    noDayActivity: "No episodes were downloaded by Auto Track on this day.",
    checkNow: "Check now",
    deleteTrack: "Delete Auto Track",
    filesInTrack: (count: number) => `${count} files in Auto Track`,
    loadingDetail: "Loading details...",
    noTracks: "Go to Downloads, check a series folder link, then enable Auto Track.",
    deleteTitle: "Delete Auto Track",
    deleteBody: "FHUB will stop watching this folder. Downloaded files and existing queue items will be kept.",
    cancel: "Cancel",
    delete: "Delete",
    downloaded: "Downloaded",
    downloading: "Downloading",
    queued: "Queued",
    paused: "Paused",
    skipped: "Skipped",
    failed: "Download failed",
    notDownloaded: "Not downloaded",
    completedAt: (value: string) => `Downloaded: ${value}`,
    queuedAt: (value: string) => `Queued: ${value}`,
    seenAt: (value: string) => `Seen: ${value}`,
    noTime: "No time yet",
    downloadEpisode: (name: string) => `Download ${name}`,
    addEpisode: (name: string, title: string) => `Adding ${name} from ${title} to Downloads...`,
    addedEpisode: (title: string, name: string) => `Added ${title} · ${name} to Downloads.`,
    addFailed: (name: string) => `Could not add ${name}.`,
    nextWorker: "Next: when the worker runs",
    lastNone: "Last check: not checked in background yet",
    lastAt: (value: string) => `Last check: ${value}`,
    nextAt: (value: string) => `Next: ${value}`,
    mins: (value: number) => `${value} min`,
    hours: (value: string | number) => `${value} hr`,
  });

  function loadLanguage() {
    try {
      const saved = localStorage.getItem("fhub-ui-language");
      if (saved === "vi" || saved === "en") language = saved;
    } catch {
      // localStorage may be unavailable.
    }
  }

  async function loadTracks() {
    loading = true;
    try {
      const res = await fetch("/api/auto-track", { credentials: "include" });
      if (!res.ok) throw new Error(await res.text());
      tracks = await res.json();
      status = tracks.length ? "" : text.empty;
      await loadRecentDetails(tracks);
    } catch (err) { status = `${text.loadFailed}: ${messageOf(err)}`; }
    finally { loading = false; }
  }

  async function toggleDetails(track: Track) {
    if (expandedId === track.id) {
      expandedId = null;
      return;
    }
    expandedId = track.id;
    if (!details[track.id]) await loadTrackDetail(track);
  }

  async function loadTrackDetail(track: Track) {
    const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}`, { credentials: "include" });
    if (!res.ok) { status = await res.text(); return; }
    const data = await res.json();
    details = { ...details, [track.id]: data };
  }

  async function loadRecentDetails(trackList: Track[]) {
    const missing = trackList.filter((track) => !details[track.id]);
    if (!missing.length) return;
    const loaded = await Promise.all(missing.map(async (track) => {
      try {
        const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}`, { credentials: "include" });
        if (!res.ok) return null;
        return [track.id, await res.json()] as const;
      } catch { return null; }
    }));
    const next = { ...details };
    for (const item of loaded) if (item) next[item[0]] = item[1];
    details = next;
  }

  async function checkAllTracks() {
    const activeTracks = tracks.filter((track) => track.enabled);
    if (!activeTracks.length) {
      status = text.noEnabled;
      return;
    }
    loading = true;
    status = text.checkingMany(activeTracks.length);
    let ok = 0;
    let failed = 0;
    for (const track of activeTracks) {
      try {
        const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}/check`, { method: "POST", credentials: "include" });
        if (!res.ok) throw new Error(await res.text());
        ok += 1;
        await loadTrackDetail(track);
      } catch {
        failed += 1;
      }
    }
    loading = false;
    status = failed ? text.checkedSome(ok, activeTracks.length, failed) : text.checkedAll(ok);
    await loadTracks();
  }

  async function checkNow(track: Track) {
    status = text.scanning(track.title);
    const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}/check`, { method: "POST", credentials: "include" });
    const data = res.ok ? await res.json() : { message: await res.text() };
    status = data.message || text.scanned;
    await loadTracks();
    await loadTrackDetail(track);
    expandedId = track.id;
  }

  async function toggleTrack(track: Track) {
    const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}`, { method: "PATCH", headers: { "Content-Type": "application/json" }, credentials: "include", body: JSON.stringify({ enabled: !track.enabled }) });
    if (!res.ok) { status = await res.text(); return; }
    await loadTracks();
  }

  async function deleteTrack(track: Track) {
    const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}`, { method: "DELETE", credentials: "include" });
    if (!res.ok) { status = await res.text(); return; }
    if (expandedId === track.id) expandedId = null;
    pendingDeleteTrack = null;
    const next = { ...details };
    delete next[track.id];
    details = next;
    await loadTracks();
  }

  function messageOf(err: unknown) { return err instanceof Error ? err.message : String(err); }
  function dateLabel(v?: string) { return v ? new Date(v).toLocaleString(language === "vi" ? "vi-VN" : "en-US", { hour12: false }) : text.noTime; }
  function compactTrackTime(v?: string) {
    if (!v) return "Chưa track";
    const date = new Date(v);
    const now = new Date();
    const sameDay = date.getFullYear() === now.getFullYear() && date.getMonth() === now.getMonth() && date.getDate() === now.getDate();
    const time = date.toLocaleTimeString("vi-VN", { hour: "2-digit", minute: "2-digit", hour12: false });
    if (sameDay) return time;
    return `${time} · ${date.toLocaleDateString(language === "vi" ? "vi-VN" : "en-US", { day: "2-digit", month: "2-digit" })}`;
  }
  function lastCheckLabel(track: Track) {
    return track.last_checked_at ? text.lastAt(compactTrackTime(track.last_checked_at)) : text.lastNone;
  }
  function intervalLabel(seconds?: number) {
    const secs = Number(seconds || 3600);
    if (secs < 3600) return text.mins(Math.round(secs / 60));
    const hours = secs / 3600;
    return text.hours(Number.isInteger(hours) ? hours : hours.toFixed(1));
  }
  function nextTrackLabel(track: Track) {
    const base = track.last_checked_at;
    if (!base) return text.nextWorker;
    const next = new Date(new Date(base).getTime() + Number(track.check_interval_secs || 3600) * 1000);
    return text.nextAt(compactTrackTime(next.toISOString()));
  }
  function dayKey(date: Date) {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, "0");
    const d = String(date.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }

  function lastSevenDays() {
    const today = new Date();
    return Array.from({ length: 7 }, (_, offset) => 6 - offset).map((index) => {
      const date = new Date(today);
      date.setDate(today.getDate() - index);
      const key = dayKey(date);
      return {
        key,
        label: date.toLocaleDateString("vi-VN", { day: "2-digit", month: "2-digit" }),
        short: `${activityCountPlaceholder(index)}`,
      };
    });
  }

  function activityCountPlaceholder(_index: number) {
    return "";
  }

  function activityEvents() {
    const downloadableStatuses = new Set(["queued", "downloading", "paused", "completed", "failed"]);
    const events = Object.values(details).flatMap((detail) => detail.items.flatMap((item) => {
      const status = String(item.status || "").toLowerCase();
      const hasDownloadTask = !!(item.download_id || item.queued_at || item.completed_at);
      if (!item.auto_queued || !hasDownloadTask || !downloadableStatuses.has(status)) return [];
      const at = item.completed_at || item.queued_at || item.first_seen_at || "";
      if (!at) return [];
      return [{ track: detail, item, at, status: item.completed_at ? text.downloaded : itemStatusLabel(item) }];
    }));
    const min = new Date();
    min.setDate(min.getDate() - 6);
    min.setHours(0, 0, 0, 0);
    return events
      .filter((event) => event.at && new Date(event.at).getTime() >= min.getTime())
      .sort((a, b) => new Date(b.at).getTime() - new Date(a.at).getTime());
  }

  function activityForDay(key: string) {
    return activityEvents().filter((event) => dayKey(new Date(event.at)) === key);
  }

  function activityCount(key: string) {
    return activityForDay(key).length;
  }

  function selectedActivityEvents() {
    return activityForDay(selectedActivityDay);
  }

  function latestAutoQueuedCompletedAt(track: Track) {
    const detail = details[track.id];
    if (!detail) return "";
    return detail.items
      .filter((item) => item.auto_queued && item.completed_at)
      .map((item) => item.completed_at || "")
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime())[0] || "";
  }

  function hasNewAutoTrackEpisode(track: Track) {
    const latest = latestAutoQueuedCompletedAt(track);
    if (!latest) return false;
    return Date.now() - new Date(latest).getTime() < 24 * 60 * 60 * 1000;
  }

  function sortedTracks() {
    return [...tracks].sort((a, b) => {
      const aNew = hasNewAutoTrackEpisode(a);
      const bNew = hasNewAutoTrackEpisode(b);
      if (aNew !== bNew) return aNew ? -1 : 1;
      const aAt = latestAutoQueuedCompletedAt(a) || a.updated_at || a.created_at || "";
      const bAt = latestAutoQueuedCompletedAt(b) || b.updated_at || b.created_at || "";
      return new Date(bAt).getTime() - new Date(aAt).getTime();
    });
  }

  function itemEpisodeNumber(item: TrackItem) {
    if (item.episode) return Number(item.episode);
    const sxex = item.file_name.match(/S\d{1,2}\s*E(\d{1,3})/i)?.[1];
    if (sxex) return Number(sxex);
    const ep = item.file_name.match(/(?:tập|tap|episode|ep)[\s._:-]*(\d{1,3})/i)?.[1];
    return ep ? Number(ep) : 0;
  }

  function itemSeasonNumber(item: TrackItem) {
    if (item.season) return Number(item.season);
    const season = item.file_name.match(/S(\d{1,2})\s*E\d{1,3}/i)?.[1];
    return season ? Number(season) : 0;
  }

  function sortedTrackItems(items: TrackItem[]) {
    return [...items].sort((a, b) => {
      const seasonDiff = itemSeasonNumber(b) - itemSeasonNumber(a);
      if (seasonDiff) return seasonDiff;
      const episodeDiff = itemEpisodeNumber(b) - itemEpisodeNumber(a);
      if (episodeDiff) return episodeDiff;
      const aAt = a.completed_at || a.queued_at || a.first_seen_at || "";
      const bAt = b.completed_at || b.queued_at || b.first_seen_at || "";
      return new Date(bAt).getTime() - new Date(aAt).getTime();
    });
  }

  function episodeName(item: TrackItem) {
    if (item.season && item.episode) return `S${String(item.season).padStart(2, "0")}E${String(item.episode).padStart(2, "0")}`;
    if (item.episode) return language === "vi" ? `Tập ${item.episode}` : `Episode ${item.episode}`;
    const sxex = item.file_name.match(/S\d{1,2}\s*E\d{1,3}/i)?.[0]?.replace(/\s+/g, "").toUpperCase();
    if (sxex) return sxex;
    const ep = item.file_name.match(/(?:tập|tap|episode|ep)[\s._:-]*(\d{1,3})/i)?.[1];
    return ep ? (language === "vi" ? `Tập ${ep}` : `Episode ${ep}`) : item.file_name;
  }
  function timeLabel(v?: string) { return v ? new Date(v).toLocaleString(language === "vi" ? "vi-VN" : "en-US", { hour12: false }) : text.noTime; }
  function size(v?: number) { const n = v || 0; if (n > 1073741824) return `${(n/1073741824).toFixed(2)} GB`; if (n > 1048576) return `${(n/1048576).toFixed(1)} MB`; return `${n} B`; }

  function itemStatusLabel(item: TrackItem) {
    const s = String(item.status || "").toLowerCase();
    if (s === "completed") return text.downloaded;
    if (s === "downloading") return text.downloading;
    if (s === "queued") return text.queued;
    if (s === "paused") return text.paused;
    if (s === "skipped") return text.skipped;
    if (s === "failed") return text.failed;
    return text.notDownloaded;
  }

  function itemTimeLabel(item: TrackItem) {
    if (item.completed_at) return text.completedAt(timeLabel(item.completed_at));
    if (item.queued_at) return text.queuedAt(timeLabel(item.queued_at));
    if (item.first_seen_at) return text.seenAt(timeLabel(item.first_seen_at));
    return "";
  }

  function canDownloadItem(item: TrackItem) {
    const s = String(item.status || "").toLowerCase();
    return s !== "completed" && !!(item.file_url || item.fshare_code);
  }

  async function downloadTrackItem(track: Track, item: TrackItem, event?: Event) {
    event?.preventDefault();
    event?.stopPropagation();
    const url = item.file_url || `https://www.fshare.vn/file/${item.fshare_code}`;
    status = text.addEpisode(episodeName(item), track.title);
    try {
      const res = await fetch("/api/downloads", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          url,
          filename: `${track.title} - ${episodeName(item)}.mkv`,
          category: "tv",
          priority: "NORMAL",
          folder_name: track.title,
          batch_name: track.title,
        }),
      });
      if (!res.ok) throw new Error(await res.text());
      status = text.addedEpisode(track.title, episodeName(item));
      await loadTrackDetail(track);
    } catch (err) {
      status = `${text.addFailed(episodeName(item))} ${messageOf(err)}`;
    }
  }
</script>

<section class="track-screen">
  <div class="track-hero">
    <div>
      <h1>{text.heroTitle}</h1>
      <div class="hero-meta">
        <span>{text.autoEvery(intervalLabel(tracks.find((track) => track.enabled)?.check_interval_secs || tracks[0]?.check_interval_secs))}</span>
        {#if tracks.find((track) => track.enabled)}<span>{nextTrackLabel(tracks.find((track) => track.enabled) || tracks[0])}</span>{/if}
      </div>
    </div>
    <button onclick={checkAllTracks} disabled={loading || !tracks.some((track) => track.enabled)}><span class="material-icons">sync</span>{text.checkAll}</button>
  </div>

  {#if status}
    <p class="status">{status}</p>
  {/if}

  <section class="recent-panel">
    <div class="recent-head">
      <div><span class="material-icons">event_note</span><strong>{text.historyTitle}</strong></div>
      <button type="button" class="activity-toggle" onclick={() => activityOpen = !activityOpen}>{activityOpen ? text.hide : text.show}</button>
    </div>
    <div class="day-tabs" role="tablist" aria-label="Chọn ngày lịch sử Auto Track">
      {#each lastSevenDays() as day}
        <button
          type="button"
          class:active={selectedActivityDay === day.key}
          onclick={() => selectedActivityDay = day.key}
          role="tab"
          aria-selected={selectedActivityDay === day.key}
        >
          <strong>{day.label}</strong>
          <small>{text.dayItems(activityCount(day.key))}</small>
        </button>
      {/each}
    </div>
    {#if activityOpen}
    <div class="recent-list">
      {#each selectedActivityEvents() as event}
        <div class="recent-item">
          <span class="material-icons">download_done</span>
          <div>
            <strong>{event.track.title} · {episodeName(event.item)}</strong>
            <small>{compactTrackTime(event.at)} · {event.status}</small>
          </div>
        </div>
      {:else}
        <div class="recent-empty">{text.noDayActivity}</div>
      {/each}
    </div>
    {/if}
  </section>

  <section class="cards">
    {#each sortedTracks() as track}
      {@const expanded = expandedId === track.id}
      {@const detail = details[track.id]}
      <article class:off={!track.enabled} class:expanded onclick={() => toggleDetails(track)} onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") void toggleDetails(track); }} role="button" tabindex="0" aria-expanded={expanded}>
        <div class="card-head">
          <div class="track-poster" class:has-image={!!track.poster_url}>
            {#if track.poster_url}
              <img src={track.poster_url} alt={`Poster ${track.title}`} loading="lazy" />
            {:else}
              <span class="material-icons">movie</span>
            {/if}
          </div>
          <div>
            <h2>{track.title}{#if hasNewAutoTrackEpisode(track)} <span class="new-badge">NEW</span>{/if}</h2>

          </div>
          <span class="expand-icon material-icons">{expanded ? "expand_less" : "expand_more"}</span>
        </div>

        <div class="actions compact-actions" onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()} role="group" aria-label="Auto Track actions">
          <button class="check-btn" onclick={() => checkNow(track)} title={text.checkNow} aria-label={text.checkNow}><span class="material-icons">sync</span></button>
          <button class:track-on={track.enabled} class:track-off={!track.enabled} onclick={() => toggleTrack(track)}>
            <span class="material-icons">{track.enabled ? "toggle_on" : "toggle_off"}</span>AutoTrack: {track.enabled ? "ON" : "OFF"}
          </button>
          <button class="delete-icon" onclick={() => pendingDeleteTrack = track} title={text.deleteTrack} aria-label={text.deleteTrack}><span class="material-icons">delete</span></button>
        </div>

        {#if expanded}
          <div class="detail-inline">
            {#if detail}
              <div class="detail-head"><strong>{text.filesInTrack(detail.items.length)}</strong></div>
              <div class="items">
                {#each sortedTrackItems(detail.items) as item}
                  <div class="item">
                    <span class="material-icons">movie</span>
                    <div>
                      <strong>{item.file_name}</strong>
                      <small>{item.fshare_code} · {size(item.file_size)}</small>
                      <small>{itemTimeLabel(item)}</small>
                      {#if String(item.status || '').toLowerCase() === "failed"}<small class="err">{text.failed}</small>{/if}
                    </div>
                    <div class="item-right">
                      <b class={String(item.status || '').toLowerCase()}>{itemStatusLabel(item)}</b>
                      {#if canDownloadItem(item)}
                        <button
                          type="button"
                          class="item-download"
                          title={text.downloadEpisode(episodeName(item))}
                          aria-label={text.downloadEpisode(episodeName(item))}
                          onclick={(event) => downloadTrackItem(track, item, event)}
                        >
                          <span class="material-icons">arrow_downward</span>
                        </button>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="loading-detail">{text.loadingDetail}</div>
            {/if}
          </div>
        {/if}
      </article>
    {:else}
      <div class="empty">{text.noTracks}</div>
    {/each}
  </section>

  {#if pendingDeleteTrack}
    <div class="confirm-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) pendingDeleteTrack = null; }}>
      <section class="confirm-card" role="dialog" aria-modal="true" aria-labelledby="delete-track-title">
        <div class="confirm-icon"><span class="material-icons">bookmark_remove</span></div>
        <div>
          <span class="eyebrow">{text.deleteTitle}</span>
          <h2 id="delete-track-title">{pendingDeleteTrack.title}</h2>
          <p>{text.deleteBody}</p>
        </div>
        <div class="confirm-actions">
          <button type="button" onclick={() => pendingDeleteTrack = null}>{text.cancel}</button>
          <button type="button" class="confirm-danger" onclick={() => pendingDeleteTrack && deleteTrack(pendingDeleteTrack)}><span class="material-icons">delete</span>{text.delete}</button>
        </div>
      </section>
    </div>
  {/if}
</section>

<style>
  .track-screen{display:grid;gap:1rem;width:min(100%,1280px);max-width:none;margin:0 auto}.track-hero{display:flex;justify-content:space-between;gap:1.35rem;align-items:center;padding:clamp(1.35rem,3vw,2.25rem);min-height:clamp(190px,18vw,260px);border-radius:30px;background:linear-gradient(135deg,rgba(124,58,237,.26),rgba(8,10,18,.96));border:1px solid rgba(167,139,250,.26)}.hero-meta{display:flex;flex-wrap:wrap;gap:.7rem;margin-top:1rem}.hero-meta span{padding:.48rem .85rem;border-radius:999px;color:#e0f2fe;background:rgba(56,189,248,.1);border:1px solid rgba(56,189,248,.16);font-size:clamp(.88rem,1.8vw,1.35rem);font-weight:900}.recent-panel{display:grid;gap:.7rem;padding:.9rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.94),rgba(10,14,24,.82));border:1px solid rgba(56,189,248,.18)}.recent-head{display:flex;align-items:center;justify-content:space-between;gap:.7rem;color:#fff}.recent-head>div{display:flex;align-items:center;gap:.45rem}.recent-head .material-icons{color:#38bdf8}.activity-toggle{min-height:32px;padding:0 .7rem;border-radius:999px;font-size:.78rem}.day-tabs{display:grid;grid-template-columns:repeat(7,minmax(0,1fr));gap:.45rem;overflow:auto;padding-bottom:.1rem}.day-tabs button{display:grid;gap:.12rem;min-height:54px;justify-items:start;align-content:center;padding:.45rem .55rem;border-radius:14px;background:rgba(255,255,255,.045);border:1px solid rgba(148,163,184,.12);color:#e5e7eb}.day-tabs button:first-child{color:#e5e7eb;background:rgba(255,255,255,.045);border:1px solid rgba(148,163,184,.12)}.day-tabs button.active{color:#080a12;background:linear-gradient(135deg,#38bdf8,#f8c14a);border:0}.day-tabs button small{color:inherit;opacity:.78}.recent-list{display:grid;gap:.5rem;max-height:326px;overflow-y:auto;padding-right:.2rem;scrollbar-width:thin;scrollbar-color:rgba(56,189,248,.45) rgba(255,255,255,.06)}.recent-list::-webkit-scrollbar{width:8px}.recent-list::-webkit-scrollbar-track{background:rgba(255,255,255,.05);border-radius:999px}.recent-list::-webkit-scrollbar-thumb{background:linear-gradient(180deg,#38bdf8,#a78bfa);border-radius:999px}.recent-item{display:grid;grid-template-columns:34px minmax(0,1fr);gap:.6rem;align-items:center;min-height:58px;padding:.62rem;border-radius:14px;background:rgba(255,255,255,.045);border:1px solid rgba(148,163,184,.12)}.recent-item.tracked>.material-icons{background:linear-gradient(135deg,#a78bfa,#38bdf8)}.recent-item>.material-icons{width:34px;height:34px;display:grid;place-items:center;border-radius:11px;color:#080a12;background:linear-gradient(135deg,#38bdf8,#a78bfa)}.recent-item strong,.recent-item small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.recent-item small{margin-top:.12rem;color:#aab4c3}.recent-empty{padding:.75rem;border-radius:14px;color:#aab4c3;background:rgba(255,255,255,.035);border:1px dashed rgba(148,163,184,.18)}.eyebrow{color:#c4b5fd;font-size:.72rem;font-weight:950;letter-spacing:.16em}h1{margin:.3rem 0;font-size:clamp(2.6rem,5.2vw,5.8rem);letter-spacing:-.06em;line-height:.95}p,small,.status{color:#aab4c3}.cards{display:grid;gap:.75rem}.cards article,.empty{padding:1rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.94),rgba(10,14,24,.82));border:1px solid rgba(148,163,184,.16)}.cards article{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:1rem;align-items:center;cursor:pointer}.cards article.expanded{border-color:rgba(167,139,250,.34);box-shadow:0 18px 48px rgba(0,0,0,.22)}.cards article.off{opacity:.65}.card-head{display:grid;grid-template-columns:82px minmax(0,1fr) auto;gap:1rem;align-items:start}.track-poster{width:82px;height:110px;border-radius:16px;overflow:hidden;display:grid;place-items:center;background:linear-gradient(135deg,rgba(56,189,248,.18),rgba(167,139,250,.14));border:1px solid rgba(148,163,184,.16);box-shadow:0 14px 34px rgba(0,0,0,.26)}.track-poster img{width:100%;height:100%;object-fit:cover;display:block}.track-poster .material-icons{color:#f8c14a;font-size:2rem}.expand-icon{width:34px;height:34px;display:grid;place-items:center;border-radius:12px;color:#c4b5fd;background:rgba(255,255,255,.055)}.delete-icon{width:40px;height:40px;min-height:40px;padding:0;border-radius:12px;color:#fecaca;background:rgba(248,113,113,.1);border:1px solid rgba(248,113,113,.18)}.delete-icon .material-icons{font-size:18px}.delete-icon:hover{background:rgba(248,113,113,.18);border-color:rgba(248,113,113,.32)}.cards h2{margin:.1rem 0;color:#fff;display:flex;align-items:center;gap:.45rem;flex-wrap:wrap}.new-badge{display:inline-flex;align-items:center;height:22px;padding:0 .45rem;border-radius:999px;font-size:.68rem;font-weight:950;letter-spacing:.04em;color:#080a12;background:linear-gradient(135deg,#38f8a6,#f8c14a);box-shadow:0 0 18px rgba(56,248,166,.28)}.actions{display:flex;flex-wrap:wrap;gap:.5rem}.compact-actions{display:grid;grid-template-columns:44px minmax(132px,176px) 44px;align-self:center}button{cursor:pointer;border:1px solid rgba(148,163,184,.16);color:#f8fafc;background:rgba(255,255,255,.06);border-radius:12px;min-height:40px;padding:0 .8rem;font-weight:850;display:inline-flex;align-items:center;justify-content:center;gap:.4rem}.check-btn{width:44px;padding:0}.check-btn,.track-hero button{color:#080a12;border:0;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.track-hero>button{min-width:min(100%,420px);min-height:74px;padding:0 2rem;border-radius:22px;font-size:clamp(1.05rem,2.1vw,1.55rem);box-shadow:0 18px 42px rgba(0,0,0,.28),0 0 28px rgba(167,139,250,.18)}.track-hero>button .material-icons{font-size:1.7em}.track-on{color:#052e16;border-color:rgba(34,197,94,.22);background:linear-gradient(135deg,#86efac,#38bdf8)}.track-off{color:#fee2e2;border-color:rgba(248,113,113,.24);background:rgba(248,113,113,.1)}.track-on .material-icons,.track-off .material-icons{font-size:1.35rem}.danger{color:#fecaca}.err{display:block;color:#fca5a5}.detail-inline{grid-column:1/-1;display:grid;gap:.7rem;padding-top:.75rem;border-top:1px solid rgba(148,163,184,.14)}.detail-head{display:flex;justify-content:space-between;gap:.8rem;align-items:center}.detail-head strong{color:#fff}.items{display:grid;gap:.5rem}.item{display:grid;grid-template-columns:32px minmax(0,1fr) auto;gap:.6rem;align-items:center;padding:.65rem;border-radius:14px;background:rgba(255,255,255,.045);border:1px solid rgba(148,163,184,.12)}.item strong,.item small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.item-right{display:flex;align-items:center;justify-content:flex-end;gap:.4rem}.item b{padding:.25rem .5rem;border-radius:999px;background:rgba(167,139,250,.16);color:#c4b5fd;white-space:nowrap}.item-download{width:32px;height:32px;min-height:32px;padding:0;display:grid;place-items:center;border-radius:999px;color:#080a12;background:linear-gradient(135deg,#38bdf8,#f8c14a);border:0;box-shadow:0 10px 22px rgba(0,0,0,.24)}.item-download .material-icons{font-size:18px;color:#080a12}.item-download:hover{filter:brightness(1.08);transform:translateY(-1px)}.item b.queued{background:rgba(56,189,248,.16);color:#7dd3fc}.item b.downloading{background:rgba(248,193,74,.16);color:#facc15}.item b.completed{background:rgba(34,197,94,.16);color:#86efac}.item b.paused{background:rgba(148,163,184,.14);color:#cbd5e1}.item b.skipped{background:rgba(148,163,184,.14);color:#cbd5e1}.item b.failed{background:rgba(248,113,113,.16);color:#fca5a5}.loading-detail{color:#aab4c3}.empty{text-align:center;color:#aab4c3}.confirm-backdrop{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;padding:1rem;background:rgba(2,6,23,.72);backdrop-filter:blur(12px)}.confirm-card{width:min(430px,100%);display:grid;gap:1rem;padding:1.1rem;border-radius:24px;border:1px solid rgba(248,113,113,.22);background:radial-gradient(circle at 0% 0%,rgba(248,193,74,.14),transparent 34%),linear-gradient(180deg,rgba(20,26,42,.98),rgba(7,10,18,.96));box-shadow:0 28px 90px rgba(0,0,0,.55)}.confirm-icon{width:56px;height:56px;display:grid;place-items:center;border-radius:18px;color:#080a12;background:linear-gradient(135deg,#f8c14a,#fb7185)}.confirm-icon .material-icons{color:#080a12}.confirm-card h2{margin:.2rem 0;color:#fff;font-size:1.35rem}.confirm-card p{margin:0;line-height:1.45}.confirm-actions{display:grid;grid-template-columns:1fr 1fr;gap:.65rem}.confirm-actions button{justify-content:center}.confirm-danger{color:#080a12!important;border:0!important;background:linear-gradient(135deg,#f8c14a,#fb7185)!important}.confirm-danger .material-icons{color:#080a12}@media(max-width:720px){.track-screen{width:100%}.cards article{grid-template-columns:1fr;align-items:stretch}.track-hero{min-height:0;border-radius:22px}.track-hero>button{width:100%;min-width:0;min-height:58px;border-radius:18px}.card-head{grid-template-columns:70px minmax(0,1fr) auto}.track-poster{width:70px;height:94px;border-radius:14px}.track-hero{display:grid;padding:1rem}.recent-head{align-items:flex-start}.day-tabs{grid-template-columns:repeat(7,78px);margin-inline:-.1rem}.day-tabs button{min-height:50px;padding:.42rem .5rem}.recent-list{max-height:196px}.actions,.compact-actions{display:grid;grid-template-columns:42px minmax(0,1fr) 42px}.actions button{justify-content:center;padding:0 .55rem}.check-btn{width:42px}.delete-icon{width:42px;height:40px;min-height:40px}.item{grid-template-columns:30px minmax(0,1fr);align-items:start}.item-right{grid-column:2;justify-self:start;justify-content:flex-start}.detail-head{display:grid}.track-screen{padding-bottom:.5rem}}
</style>
