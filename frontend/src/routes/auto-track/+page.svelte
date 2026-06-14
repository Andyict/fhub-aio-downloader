<script lang="ts">
  import { onMount } from "svelte";

  type Track = { id:string; title:string; folder_url:string; folder_code:string; enabled:boolean; check_interval_secs:number; last_checked_at?:string; last_error?:string; updated_at:string; };
  type TrackItem = { id:string; file_name:string; fshare_code:string; file_size:number; season?:number; episode?:number; status:string; download_id?:string; first_seen_at?:string; queued_at?:string; completed_at?:string; error_message?:string; };
  type TrackDetail = Track & { items: TrackItem[] };

  let tracks = $state<Track[]>([]);
  let details = $state<Record<string, TrackDetail>>({});
  let expandedId = $state<string | null>(null);
  let pendingDeleteTrack = $state<Track | null>(null);
  let loading = $state(false);
  let status = $state("Đang tải Auto Track...");

  onMount(() => { void loadTracks(); });

  async function loadTracks() {
    loading = true;
    try {
      const res = await fetch("/api/auto-track", { credentials: "include" });
      if (!res.ok) throw new Error(await res.text());
      tracks = await res.json();
      status = tracks.length ? `Đang theo dõi ${tracks.length} bộ.` : "Chưa có bộ nào được Auto Track.";
    } catch (err) { status = `Không tải được Auto Track: ${messageOf(err)}`; }
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

  async function checkNow(track: Track) {
    status = `Đang quét ${track.title}...`;
    const res = await fetch(`/api/auto-track/${encodeURIComponent(track.id)}/check`, { method: "POST", credentials: "include" });
    const data = res.ok ? await res.json() : { message: await res.text() };
    status = data.message || "Đã quét xong.";
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
  function dateLabel(v?: string) { return v ? new Date(v).toLocaleString("vi-VN", { hour12: false }) : "Chưa quét"; }
  function timeLabel(v?: string) { return v ? new Date(v).toLocaleString("vi-VN", { hour12: false }) : "Chưa có thời gian"; }
  function size(v?: number) { const n = v || 0; if (n > 1073741824) return `${(n/1073741824).toFixed(2)} GB`; if (n > 1048576) return `${(n/1048576).toFixed(1)} MB`; return `${n} B`; }

  function itemStatusLabel(item: TrackItem) {
    const s = String(item.status || "").toLowerCase();
    if (["queued", "downloading", "completed", "skipped"].includes(s)) return "Đã tải";
    if (s === "failed") return "Lỗi";
    return "Đã thấy";
  }

  function itemTimeLabel(item: TrackItem) {
    const when = item.completed_at || item.queued_at || item.first_seen_at;
    if (item.completed_at) return `Tải xong: ${timeLabel(item.completed_at)}`;
    if (item.queued_at) return `Thời gian tải: ${timeLabel(item.queued_at)}`;
    if (item.first_seen_at) return `Ghi nhận: ${timeLabel(item.first_seen_at)}`;
    return "";
  }
</script>

<section class="track-screen">
  <div class="track-hero">
    <div>
      <span class="eyebrow">FHUB AUTO TRACK</span>
      <h1>Theo dõi phim bộ</h1>
      <p>FHUB quét thư mục FShare mỗi 1 giờ. Có tập mới thì tự thêm vào queue và đánh dấu tập đã tải.</p>
    </div>
    <button onclick={loadTracks} disabled={loading}><span class="material-icons">refresh</span>Làm mới</button>
  </div>

  <p class="status">{status}</p>

  <section class="cards">
    {#each tracks as track}
      {@const expanded = expandedId === track.id}
      {@const detail = details[track.id]}
      <article class:off={!track.enabled} class:expanded onclick={() => toggleDetails(track)} onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") void toggleDetails(track); }} role="button" tabindex="0" aria-expanded={expanded}>
        <div class="card-head">
          <div>
            <h2>{track.title}</h2>
            <p>{track.folder_code} · {track.enabled ? "Đang bật" : "Đang tắt"}</p>
            <small>Check gần nhất: {dateLabel(track.last_checked_at)}</small>
            {#if track.last_error}<small class="err">{track.last_error}</small>{/if}
          </div>
          <span class="expand-icon material-icons">{expanded ? "expand_less" : "expand_more"}</span>
        </div>

        <div class="actions" onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()} role="group" aria-label="Auto Track actions">
          <button onclick={() => checkNow(track)}><span class="material-icons">sync</span>Check ngay</button>
          <button onclick={() => toggleTrack(track)}>{track.enabled ? "Tạm dừng" : "Bật lại"}</button>
          <button class="danger" onclick={() => pendingDeleteTrack = track}><span class="material-icons">delete</span></button>
        </div>

        {#if expanded}
          <div class="detail-inline">
            {#if detail}
              <div class="detail-head"><strong>{detail.items.length} file đã ghi nhận</strong><small>Ấn lại vào phim để thu gọn</small></div>
              <div class="items">
                {#each detail.items as item}
                  <div class="item">
                    <span class="material-icons">movie</span>
                    <div>
                      <strong>{item.file_name}</strong>
                      <small>{item.fshare_code} · {size(item.file_size)}</small>
                      <small>{itemTimeLabel(item)}</small>
                      {#if item.error_message}<small class="err">{item.error_message}</small>{/if}
                    </div>
                    <b class={String(item.status || '').toLowerCase()}>{itemStatusLabel(item)}</b>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="loading-detail">Đang tải chi tiết...</div>
            {/if}
          </div>
        {/if}
      </article>
    {:else}
      <div class="empty">Vào trang Tải xuống, check link thư mục phim bộ rồi bấm Auto Track.</div>
    {/each}
  </section>

  {#if pendingDeleteTrack}
    <div class="confirm-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) pendingDeleteTrack = null; }}>
      <section class="confirm-card" role="dialog" aria-modal="true" aria-labelledby="delete-track-title">
        <div class="confirm-icon"><span class="material-icons">bookmark_remove</span></div>
        <div>
          <span class="eyebrow">Xoá Auto Track</span>
          <h2 id="delete-track-title">{pendingDeleteTrack.title}</h2>
          <p>FHUB sẽ ngừng theo dõi folder này. File đã tải và queue hiện có vẫn được giữ nguyên.</p>
        </div>
        <div class="confirm-actions">
          <button type="button" onclick={() => pendingDeleteTrack = null}>Huỷ</button>
          <button type="button" class="confirm-danger" onclick={() => pendingDeleteTrack && deleteTrack(pendingDeleteTrack)}><span class="material-icons">delete</span>Xoá</button>
        </div>
      </section>
    </div>
  {/if}
</section>

<style>
  .track-screen{display:grid;gap:1rem;max-width:980px;margin:0 auto}.track-hero{display:flex;justify-content:space-between;gap:1rem;align-items:center;padding:1.4rem;border-radius:22px;background:linear-gradient(135deg,rgba(124,58,237,.26),rgba(8,10,18,.96));border:1px solid rgba(167,139,250,.26)}.eyebrow{color:#c4b5fd;font-size:.72rem;font-weight:950;letter-spacing:.16em}h1{margin:.3rem 0;font-size:clamp(2rem,4vw,4rem);letter-spacing:-.05em}p,small,.status{color:#aab4c3}.cards{display:grid;gap:.75rem}.cards article,.empty{padding:1rem;border-radius:18px;background:linear-gradient(180deg,rgba(20,26,42,.94),rgba(10,14,24,.82));border:1px solid rgba(148,163,184,.16)}.cards article{display:grid;gap:1rem;cursor:pointer}.cards article.expanded{border-color:rgba(167,139,250,.34);box-shadow:0 18px 48px rgba(0,0,0,.22)}.cards article.off{opacity:.65}.card-head{display:grid;grid-template-columns:minmax(0,1fr) 34px;gap:.8rem;align-items:start}.expand-icon{width:34px;height:34px;display:grid;place-items:center;border-radius:12px;color:#c4b5fd;background:rgba(255,255,255,.055)}.cards h2{margin:.1rem 0;color:#fff}.actions{display:flex;flex-wrap:wrap;gap:.5rem}button{cursor:pointer;border:1px solid rgba(148,163,184,.16);color:#f8fafc;background:rgba(255,255,255,.06);border-radius:12px;min-height:40px;padding:0 .8rem;font-weight:850;display:inline-flex;align-items:center;gap:.4rem}button:first-child,.track-hero button{color:#080a12;border:0;background:linear-gradient(135deg,#f8c14a,#a78bfa)}.danger{color:#fecaca}.err{display:block;color:#fca5a5}.detail-inline{display:grid;gap:.7rem;padding-top:.75rem;border-top:1px solid rgba(148,163,184,.14)}.detail-head{display:flex;justify-content:space-between;gap:.8rem;align-items:center}.detail-head strong{color:#fff}.items{display:grid;gap:.5rem}.item{display:grid;grid-template-columns:32px minmax(0,1fr) auto;gap:.6rem;align-items:center;padding:.65rem;border-radius:14px;background:rgba(255,255,255,.045);border:1px solid rgba(148,163,184,.12)}.item strong,.item small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.item b{padding:.25rem .5rem;border-radius:999px;background:rgba(167,139,250,.16);color:#c4b5fd;white-space:nowrap}.item b.queued,.item b.downloading,.item b.completed,.item b.skipped{background:rgba(34,197,94,.16);color:#86efac}.item b.failed{background:rgba(248,113,113,.16);color:#fca5a5}.loading-detail{color:#aab4c3}.empty{text-align:center;color:#aab4c3}.confirm-backdrop{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;padding:1rem;background:rgba(2,6,23,.72);backdrop-filter:blur(12px)}.confirm-card{width:min(430px,100%);display:grid;gap:1rem;padding:1.1rem;border-radius:24px;border:1px solid rgba(248,113,113,.22);background:radial-gradient(circle at 0% 0%,rgba(248,193,74,.14),transparent 34%),linear-gradient(180deg,rgba(20,26,42,.98),rgba(7,10,18,.96));box-shadow:0 28px 90px rgba(0,0,0,.55)}.confirm-icon{width:56px;height:56px;display:grid;place-items:center;border-radius:18px;color:#080a12;background:linear-gradient(135deg,#f8c14a,#fb7185)}.confirm-icon .material-icons{color:#080a12}.confirm-card h2{margin:.2rem 0;color:#fff;font-size:1.35rem}.confirm-card p{margin:0;line-height:1.45}.confirm-actions{display:grid;grid-template-columns:1fr 1fr;gap:.65rem}.confirm-actions button{justify-content:center}.confirm-danger{color:#080a12!important;border:0!important;background:linear-gradient(135deg,#f8c14a,#fb7185)!important}.confirm-danger .material-icons{color:#080a12}@media(max-width:720px){.track-hero{display:grid;padding:1rem}.actions{display:grid;grid-template-columns:1fr 1fr 44px}.actions button{justify-content:center;padding:0 .55rem}.item{grid-template-columns:30px minmax(0,1fr);align-items:start}.item b{grid-column:2;justify-self:start}.detail-head{display:grid}.track-screen{padding-bottom:.5rem}}
</style>
