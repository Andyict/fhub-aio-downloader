<script lang="ts">
  import { onMount, tick } from "svelte";
  import { goto } from "$app/navigation";
  import { downloadStore } from "$lib/stores/downloads";
  import { toasts } from "$lib/stores/toasts";

  const BOOKMARKS_KEY = "fhub.search.bookmarks.v1";
  type BookmarkItem = {
    id?: string | number;
    title?: string;
    posterPath?: string;
    posterUrl?: string;
    releaseDate?: string;
    mediaType?: string;
    fcode?: string;
    originalFilename?: string;
    fileSize?: number;
    quality?: string;
    resolution?: string;
    source?: string;
    hasVietsub?: boolean;
    hasVietdub?: boolean;
    overview?: string;
  };
  type CastMember = { id: number; name: string; character?: string; profile?: string; };
  type RelatedFilm = { id: number; title: string; year?: string; score?: string; poster?: string; overview?: string; };

  let items = $state<BookmarkItem[]>([]);
  let loading = $state(true);
  let status = $state("Đang tải bookmark thật...");
  let selectedItem = $state<BookmarkItem | null>(null);
  let selectedCast = $state<CastMember[]>([]);
  let relatedFilms = $state<RelatedFilm[]>([]);
  let loadingMeta = $state(false);
  let detailPanelEl: HTMLElement | null = $state(null);

  onMount(() => {
    loadBookmarks();
    const onStorage = (event: StorageEvent) => {
      if (event.key === BOOKMARKS_KEY) loadBookmarks();
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  });

  function loadBookmarks() {
    loading = true;
    try {
      const raw = localStorage.getItem(BOOKMARKS_KEY) || "[]";
      const parsed = JSON.parse(raw);
      items = Array.isArray(parsed) ? parsed : [];
      status = items.length ? `${items.length} bookmark đã lưu` : "Chưa có bookmark nào";
    } catch {
      items = [];
      status = "Không đọc được dữ liệu bookmark.";
    } finally {
      loading = false;
    }
  }

  function persistBookmarks() {
    localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(items.slice(0, 100)));
    status = items.length ? `${items.length} bookmark đã lưu` : "Chưa có bookmark nào";
  }

  function itemKey(item: BookmarkItem) {
    return item.fcode || `${item.mediaType || "media"}:${item.id || ""}:${item.originalFilename || item.title || ""}`;
  }

  function posterFor(item: BookmarkItem) {
    if (item.posterUrl) return item.posterUrl;
    if (item.posterPath) return item.posterPath.startsWith("http") ? item.posterPath : `https://image.tmdb.org/t/p/w500${item.posterPath}`;
    return "/images/fhub-icon.clean.png";
  }

  function noteFor(item: BookmarkItem) {
    const bits = [item.resolution || item.quality, item.source, item.hasVietsub ? "Vietsub" : "", item.hasVietdub ? "Lồng tiếng" : ""].filter(Boolean);
    return bits.length ? bits.join(" · ") : item.originalFilename || "FHUB bookmark";
  }

  function tmdbImage(path?: string | null, size = "w185") {
    return path ? `https://image.tmdb.org/t/p/${size}${path}` : "";
  }

  function yearFrom(value?: string) {
    return value?.match(/^(19|20)\d{2}/)?.[0] || value?.match(/\b(19|20)\d{2}\b/)?.[0];
  }

  function mapRelatedFilm(item: any): RelatedFilm {
    const title = item.title || item.name || item.original_title || item.original_name || "Không rõ tên";
    return {
      id: item.id,
      title,
      year: yearFrom(item.release_date || item.first_air_date),
      score: item.vote_average ? Number(item.vote_average).toFixed(1) : undefined,
      poster: tmdbImage(item.poster_path, "w342") || posterFor({ title }),
      overview: item.overview,
    };
  }

  async function openItem(item: BookmarkItem) {
    selectedItem = item;
    selectedCast = [];
    relatedFilms = [];
    await tick();
    detailPanelEl?.scrollIntoView({ behavior: "smooth", block: "start" });
    await loadItemMeta(item);
  }

  async function loadItemMeta(item: BookmarkItem) {
    if (!item.id || !item.mediaType) return;
    loadingMeta = true;
    const kind = item.mediaType === "tv" ? "tv" : "movie";
    try {
      const [detailResponse, similarResponse] = await Promise.all([
        fetch(`/api/tmdb/${kind}/${item.id}?language=vi-VN`),
        fetch(`/api/tmdb/${kind}/${item.id}/similar?page=1&language=vi-VN`),
      ]);
      if (detailResponse.ok) {
        const detail = await detailResponse.json();
        selectedCast = (detail.credits?.cast || []).slice(0, 8).map((cast: any) => ({
          id: cast.id,
          name: cast.name || cast.original_name || "Diễn viên",
          character: cast.character,
          profile: tmdbImage(cast.profile_path, "w185") || posterFor({ title: cast.name }),
        }));
        if (selectedItem && itemKey(selectedItem) === itemKey(item)) {
          selectedItem = {
            ...selectedItem,
            overview: detail.overview || selectedItem.overview,
            posterUrl: tmdbImage(detail.backdrop_path, "w780") || selectedItem.posterUrl,
            releaseDate: detail.release_date || detail.first_air_date || selectedItem.releaseDate,
          };
        }
      }
      if (similarResponse.ok) {
        const payload = await similarResponse.json();
        relatedFilms = (payload.results || []).slice(0, 10).map(mapRelatedFilm);
      }
    } catch {
      selectedCast = [];
      relatedFilms = [];
    } finally {
      loadingMeta = false;
    }
  }

  function closeDetail() {
    selectedItem = null;
    selectedCast = [];
    relatedFilms = [];
  }

  function getLink(item: BookmarkItem | null) {
    if (!item?.title) return;
    const normalizedType = item.mediaType === "tv" ? "tv" : "movie";
    const params = new URLSearchParams({ q: item.title, media_type: normalizedType });
    if (item.id) params.set("open", `${normalizedType}:${item.id}`);
    goto(`/discover?${params.toString()}`);
  }

  async function openRelated(film: RelatedFilm) {
    await openItem({ id: film.id, title: film.title, posterUrl: film.poster, releaseDate: film.year, mediaType: selectedItem?.mediaType || "movie", overview: film.overview });
  }

  async function downloadItem(item: BookmarkItem) {
    if (!item.fcode) {
      toasts.error("Bookmark này chưa có mã FShare để tải.");
      return;
    }
    const tmdbId = Number(item.id);
    const mediaType: "movie" | "tv" = item.mediaType === "tv" ? "tv" : "movie";
    const tmdb = Number.isFinite(tmdbId)
      ? {
          tmdb_id: tmdbId,
          media_type: mediaType,
          title: item.title,
          year: item.releaseDate ? item.releaseDate.substring(0, 4) : undefined,
        }
      : undefined;
    const result = await downloadStore.addDownload({ url: `https://fshare.vn/file/${item.fcode}`, tmdb });
    if (result.success) toasts.success("Đã thêm bookmark vào hàng đợi tải.");
    else toasts.error(result.error || "Không thêm được bookmark vào hàng đợi.");
  }

  function removeItem(item: BookmarkItem) {
    const key = itemKey(item);
    items = items.filter((saved) => itemKey(saved) !== key);
    if (selectedItem && itemKey(selectedItem) === key) closeDetail();
    persistBookmarks();
    toasts.success("Đã xoá khỏi Bookmark.");
  }

  function clearBookmarks() {
    items = [];
    persistBookmarks();
    toasts.success("Đã xoá toàn bộ Bookmark.");
  }
</script>

<svelte:head>
  <title>Bookmark - FHUB</title>
</svelte:head>

<div class="fav-surface">
  <section class="hero">
    <span class="eyebrow">Bookmark</span>
    <h1>Nội dung đã lưu</h1>
    <p>Dữ liệu lấy trực tiếp từ các mục Sếp Lớn bấm sao trong Search/Discover.</p>
  </section>

  <div class="toolbar">
    <span>{status}</span>
    <div>
      <button type="button" class="ghost" onclick={loadBookmarks}><span class="material-icons">refresh</span>Làm mới</button>
      {#if items.length > 0}
        <button type="button" class="ghost danger" onclick={clearBookmarks}><span class="material-icons">delete_sweep</span>Xoá hết</button>
      {/if}
    </div>
  </div>


  {#if selectedItem}
    <section class="film-info-panel" aria-label="Thông tin phim đã lưu" bind:this={detailPanelEl}>
      <div class="film-main-info">
        <img src={posterFor(selectedItem)} alt={selectedItem.title || selectedItem.originalFilename || "Bookmark"} />
        <div class="film-info-copy">
          <div class="film-info-head">
            <div>
              <span class="eyebrow">Thông tin phim</span>
              <h2>{selectedItem.title || selectedItem.originalFilename || "Không rõ tên"}</h2>
            </div>
            <button type="button" aria-label="Đóng thông tin phim" onclick={closeDetail}><span class="material-icons">close</span></button>
          </div>
          <div class="film-meta">
            <span>{yearFrom(selectedItem.releaseDate) || "Chưa rõ năm"}</span>
            <span>{selectedItem.mediaType === "tv" ? "TV" : "Movie"}</span>
            <span>{selectedItem.quality || selectedItem.resolution || "Metadata"}</span>
          </div>
          <p>{selectedItem.overview || selectedItem.originalFilename || "Chưa có mô tả chi tiết cho phim này."}</p>
          <div class="film-actions">
            <button type="button" class="primary-action" onclick={() => getLink(selectedItem)}>Get link</button>
            <button type="button" class="bookmarked" onclick={() => removeItem(selectedItem!)}><span class="material-icons">bookmark</span>Đã lưu</button>
            <button type="button" onclick={() => getLink(selectedItem)}>Tìm phim tương tự</button>
          </div>
        </div>
      </div>

      <div class="film-extra-grid">
        <section class="cast-strip" aria-label="Diễn viên liên quan">
          <div class="related-head"><div><span class="eyebrow">Diễn viên</span><h3>Dàn cast</h3></div><span>{loadingMeta ? "Đang tải..." : `${selectedCast.length}`}</span></div>
          {#if selectedCast.length}
            <div class="cast-list">
              {#each selectedCast as cast}
                <article class="cast-card"><img src={cast.profile || posterFor({ title: cast.name })} alt={cast.name} /><strong>{cast.name}</strong>{#if cast.character}<small>{cast.character}</small>{/if}</article>
              {/each}
            </div>
          {:else}
            <div class="download-empty">Chưa có dữ liệu diễn viên.</div>
          {/if}
        </section>

        <section class="related-films" aria-label="Film liên quan">
          <div class="related-head"><div><span class="eyebrow">Liên quan</span><h3>Film tương tự</h3></div><span>{loadingMeta ? "Đang tải..." : `${relatedFilms.length}`}</span></div>
          {#if relatedFilms.length}
            <div class="related-film-list">
              {#each relatedFilms as film}
                <button type="button" onclick={() => openRelated(film)}><img src={film.poster || posterFor({ title: film.title })} alt={film.title} /><div><strong>{film.title}</strong><small>{film.year || "—"}{film.score ? ` · ⭐ ${film.score}` : ""}</small></div></button>
              {/each}
            </div>
          {:else}
            <div class="download-empty">Chưa có film liên quan.</div>
          {/if}
        </section>
      </div>
    </section>
  {/if}

  {#if loading}
    <section class="empty-state"><span class="material-icons">hourglass_empty</span><strong>Đang tải bookmark...</strong></section>
  {:else if items.length === 0}
    <section class="empty-state">
      <span class="material-icons">bookmark_border</span>
      <strong>Chưa có bookmark thật</strong>
      <small>Vào Search/Discover, bấm biểu tượng sao trên phim/link muốn lưu.</small>
    </section>
  {:else}
    <div class="mobile-list-title">
      <div><span class="eyebrow">Bookmark</span><h2>Danh sách phim đã lưu</h2></div>
      <strong>{items.length}</strong>
    </div>
    <section class="fav-grid">
      {#each items as item (itemKey(item))}
        <article class="fav-row">
          <img src={posterFor(item)} alt={item.title || item.originalFilename || "Bookmark"} />
          <button class="fav-copy" type="button" onclick={() => openItem(item)} title={item.originalFilename || item.title}>
            <strong>{item.title || item.originalFilename || "Không rõ tên"}</strong>
            <small>{noteFor(item)}</small>
          </button>
          <div class="row-actions">
            <button aria-label={`Tải ${item.title || item.originalFilename}`} onclick={() => downloadItem(item)}><span class="material-icons">download</span></button>
            <button class="remove" aria-label={`Xoá ${item.title || item.originalFilename}`} onclick={() => removeItem(item)}><span class="material-icons">close</span></button>
          </div>
        </article>
      {/each}
    </section>
  {/if}
</div>

<style>
  .fav-surface, .fav-surface * { box-sizing: border-box; }
  .fav-surface { width: 100%; max-width: 980px; min-width: 0; display: grid; gap: .85rem; margin: 0 auto; overflow-x: clip; }
  .hero { min-height: 132px; display: grid; align-content: end; padding: 1.15rem 1.25rem; border-radius: 20px; border: 1px solid rgba(167,139,250,.2); background: radial-gradient(circle at 8% 52%, rgba(124,58,237,.34), transparent 42%), linear-gradient(120deg, rgba(30,24,56,.96), rgba(9,13,23,.96)); box-shadow: 0 18px 42px rgba(0,0,0,.22); }
  .eyebrow { color: #c4b5fd; font-size: .66rem; font-weight: 900; letter-spacing: .13em; text-transform: uppercase; }
  h1, p { margin: 0; }
  h1 { margin-top: .25rem; color: #fff; font-size: clamp(2rem, 5vw, 3.8rem); line-height: .94; letter-spacing: -.045em; }
  p { max-width: 560px; margin-top: .45rem; color: rgba(226,232,240,.68); font-size: .95rem; line-height: 1.35; }
  .toolbar { display: flex; align-items: center; justify-content: space-between; gap: .75rem; padding: .75rem .85rem; border-radius: 16px; border: 1px solid rgba(148,163,184,.13); background: rgba(15,23,42,.62); color: rgba(226,232,240,.72); font-weight: 800; }
  .toolbar div { display: flex; gap: .5rem; flex-wrap: wrap; justify-content: flex-end; }
  button { cursor: pointer; }
  .ghost { min-height: 36px; display: inline-flex; align-items: center; gap: .35rem; border: 1px solid rgba(148,163,184,.16); border-radius: 12px; color: #f8fafc; background: rgba(255,255,255,.055); font-weight: 850; }
  .ghost.danger { color: #fecaca; }
  .mobile-list-title { display: none; }
  .fav-grid { display: grid; gap: .65rem; }
  .fav-row { display: grid; grid-template-columns: 58px minmax(0,1fr) auto; align-items: center; gap: .75rem; min-height: 74px; padding: .55rem; border-radius: 16px; border: 1px solid rgba(148,163,184,.13); background: linear-gradient(180deg, rgba(20,26,42,.9), rgba(9,12,21,.84)); box-shadow: 0 12px 30px rgba(0,0,0,.18); }
  .fav-row img { width: 58px; height: 58px; object-fit: cover; border-radius: 13px; background: rgba(255,255,255,.06); }
  .fav-copy { min-width: 0; padding: 0; border: 0; text-align: left; background: transparent; }
  .fav-row strong, .fav-row small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fav-row strong { color: #fff; font-size: .98rem; }
  .fav-row small { margin-top: .15rem; color: rgba(226,232,240,.58); font-size: .78rem; }
  .row-actions { display: flex; gap: .4rem; }
  .row-actions button { width: 42px; height: 42px; border: 0; border-radius: 13px; color: #080a12; background: linear-gradient(135deg, #f8c14a, #a78bfa); }
  .row-actions button.remove { color: #fecaca; background: rgba(239,68,68,.12); border: 1px solid rgba(248,113,113,.2); }
  .row-actions button .material-icons { color: currentColor; font-size: 1.25rem; }
  .film-info-panel { width: 100%; max-width: 100%; min-width: 0; display: grid; gap: 1rem; padding: 1rem; border: 1px solid rgba(248,193,74,.18); border-radius: 24px; background: radial-gradient(circle at 0 0, rgba(248,193,74,.14), transparent 32%), linear-gradient(180deg, rgba(20,26,42,.94), rgba(9,12,21,.88)); box-shadow: 0 20px 58px rgba(0,0,0,.28); overflow: hidden; }
  .film-main-info { min-width: 0; display: grid; grid-template-columns: minmax(260px, 1.1fr) minmax(0, 1fr); gap: 1rem; align-items: stretch; }
  .film-main-info > img { width: 100%; min-height: 360px; max-height: 440px; object-fit: cover; border-radius: 20px; border: 1px solid rgba(255,255,255,.08); background: rgba(255,255,255,.05); }
  .film-info-copy { min-width: 0; display: grid; gap: .75rem; align-content: start; }
  .film-info-head { display: flex; align-items: flex-start; justify-content: space-between; gap: .75rem; }
  .film-info-head h2 { margin: .2rem 0 0; color: #fff; font-size: clamp(1.55rem, 3vw, 2.35rem); line-height: 1; letter-spacing: -.04em; }
  .film-info-head button { width: 40px; height: 40px; flex: 0 0 auto; display: grid; place-items: center; border: 1px solid rgba(255,255,255,.12); border-radius: 14px; color: #fff; background: rgba(255,255,255,.07); }
  .film-meta { display: flex; gap: .4rem; flex-wrap: wrap; }
  .film-meta span { padding: .32rem .58rem; border-radius: 999px; background: rgba(255,255,255,.08); color: rgba(248,250,252,.82); font-size: .76rem; font-weight: 850; }
  .film-info-copy p { max-width: 760px; margin: 0; color: rgba(226,232,240,.68); line-height: 1.5; }
  .film-actions { display: flex; gap: .5rem; flex-wrap: wrap; }
  .film-actions button { min-height: 40px; display: inline-flex; align-items: center; gap: .35rem; padding: 0 .85rem; border: 1px solid rgba(255,255,255,.1); border-radius: 14px; color: #fff; background: rgba(255,255,255,.07); font-weight: 900; }
  .film-actions .primary-action, .film-actions .bookmarked { color: #111827; border: 0; background: linear-gradient(135deg, #f8c14a, #ff7a59); }
  .film-extra-grid { min-width: 0; display: grid; grid-template-columns: minmax(0,1fr) minmax(0,1fr); gap: .8rem; }
  .cast-strip, .related-films { min-width: 0; overflow: hidden; padding: .85rem; border-radius: 18px; border: 1px solid rgba(148,163,184,.12); background: rgba(8,13,23,.54); }
  .related-head { display: flex; justify-content: space-between; gap: .6rem; margin-bottom: .7rem; }
  .related-head h3 { margin: .12rem 0 0; color: #fff; }
  .related-head > span { width: 28px; height: 28px; display: grid; place-items: center; border-radius: 999px; color: #111827; background: #f8c14a; font-weight: 950; font-size: .75rem; }
  .cast-list, .related-film-list { width: 100%; max-width: 100%; min-width: 0; display: flex; gap: .55rem; overflow-x: auto; overflow-y: hidden; overscroll-behavior-x: contain; padding-bottom: .15rem; }
  .cast-card { flex: 0 0 88px; display: grid; gap: .28rem; color: #fff; }
  .cast-card img { width: 72px; height: 72px; border-radius: 999px; object-fit: cover; border: 1px solid rgba(255,255,255,.1); }
  .cast-card strong, .cast-card small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cast-card strong { font-size: .72rem; }
  .cast-card small { color: rgba(226,232,240,.52); font-size: .64rem; }
  .related-film-list button { flex: 0 0 126px; display: grid; gap: .35rem; padding: 0; border: 0; background: transparent; text-align: left; }
  .related-film-list img { width: 100%; height: 150px; object-fit: cover; border-radius: 12px; background: rgba(255,255,255,.05); }
  .related-film-list strong, .related-film-list small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .related-film-list strong { color: #fff; font-size: .74rem; }
  .related-film-list small { color: rgba(226,232,240,.58); font-size: .68rem; }
  .download-empty { min-height: 88px; display: grid; place-items: center; color: rgba(226,232,240,.58); font-weight: 800; text-align: center; }
  .empty-state { min-height: 220px; display: grid; place-items: center; align-content: center; gap: .45rem; text-align: center; border-radius: 18px; border: 1px solid rgba(148,163,184,.13); background: linear-gradient(180deg, rgba(20,26,42,.9), rgba(9,12,21,.84)); color: rgba(226,232,240,.72); }
  .empty-state .material-icons { color: #c4b5fd; font-size: 2.4rem; }
  .empty-state strong { color: #fff; }
  .empty-state small { color: rgba(226,232,240,.58); }
  @media (max-width: 720px) {
    .fav-surface { width: 100%; max-width: 100%; gap: .65rem; padding-inline: 0; }
    .hero, .toolbar { display: none; }
    .mobile-list-title { display: flex; align-items: end; justify-content: space-between; gap: .75rem; padding: .2rem .12rem .28rem; }
    .mobile-list-title h2 { margin: .12rem 0 0; color: #fff; font-size: 1.25rem; line-height: 1; letter-spacing: -.035em; }
    .mobile-list-title > strong { min-width: 34px; height: 34px; display: grid; place-items: center; border-radius: 12px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); font-size: .92rem; }
    .eyebrow { font-size: .56rem; }
    h1 { font-size: clamp(1.75rem, 9vw, 2.55rem); }
    p { max-width: 32ch; margin-top: .3rem; font-size: .82rem; line-height: 1.28; }
    .fav-row { grid-template-columns: 82px minmax(0,1fr) 42px; min-height: 96px; gap: .72rem; padding: .62rem; border-radius: 16px; }
    .fav-row img { width: 82px; height: 82px; border-radius: 15px; }
    .row-actions { grid-column: auto; display: grid; grid-template-columns: 1fr; gap: .38rem; justify-content: stretch; align-self: stretch; }
    .row-actions button { width: 42px; height: 39px; border-radius: 12px; }
    .row-actions button .material-icons { font-size: 1.12rem; }
    .fav-row strong { font-size: 1rem; }
    .fav-row small { font-size: .76rem; }
    .film-info-panel { width: 100%; max-width: 100%; min-width: 0; gap: .72rem; padding: .68rem; border-radius: 18px; justify-self: stretch; }
    .film-main-info { width: 100%; max-width: 100%; min-width: 0; grid-template-columns: minmax(0,1fr); gap: .72rem; }
    .film-main-info > img { height: 280px; min-height: 0; border-radius: 16px; }
    .film-extra-grid { width: 100%; max-width: 100%; min-width: 0; grid-template-columns: minmax(0,1fr); gap: .65rem; }
    .cast-strip, .related-films { padding: .65rem; border-radius: 16px; }
    .film-info-head h2 { font-size: 1.25rem; }
    .film-info-head button { width: 34px; height: 34px; border-radius: 12px; }
    .film-info-copy p { display: -webkit-box; -webkit-line-clamp: 5; -webkit-box-orient: vertical; overflow: hidden; font-size: .84rem; line-height: 1.45; }
    .film-meta span { font-size: .72rem; padding: .28rem .48rem; }
    .film-actions { display: grid; grid-template-columns: 1fr; gap: .45rem; }
    .film-actions button { width: 100%; min-width: 0; min-height: 38px; border-radius: 12px; font-size: .82rem; justify-content: center; }
    .cast-card { flex-basis: 78px; }
    .cast-card img { width: 64px; height: 64px; }
    .related-film-list button { flex-basis: 116px; }
    .related-film-list img { height: 134px; }
  }
</style>
