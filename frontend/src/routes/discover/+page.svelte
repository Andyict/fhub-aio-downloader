<script lang="ts">
  import { onMount, tick } from "svelte";
  import { page } from "$app/state";

  type UiLanguage = "vi" | "en";

  type DiscoverItem = {
    id?: number;
    title: string;
    year?: string;
    score?: string;
    type?: string;
    size?: string;
    seed?: string;
    img?: string;
    heroImg?: string;
    overview?: string;
    fshareCount?: number;
    url?: string;
    originalTitle?: string;
  };

  type FshareResult = {
    name?: string;
    original_name?: string;
    url?: string;
    size?: number;
    quality?: string | null;
    resolution?: string | null;
    source?: string | null;
  };

  type PreviewItem = {
    name: string;
    url: string;
    size: number;
    is_directory?: boolean;
    quality?: string;
    season?: number | null;
    episode?: number | null;
  };

  type CastMember = { id: number; name: string; character?: string; profile?: string; };
  type RelatedFilm = { id: number; title: string; year?: string; score?: string; poster?: string; overview?: string; };

  const BOOKMARKS_KEY = "fhub.search.bookmarks.v1";
  const fallbackPoster = "https://placehold.co/500x750/111827/e5e7eb?text=";
  const filterGenres = ["Tất cả", "Hành động", "Viễn tưởng", "Kinh dị", "Phiêu lưu", "Hình sự", "Tâm lý", "Hoạt hình", "Chiến tranh", "Thảm hoạ"];
  const genreIds: Record<string, { movie: number; tv: number }> = {
    "Hành động": { movie: 28, tv: 10759 },
    "Viễn tưởng": { movie: 878, tv: 10765 },
    "Kinh dị": { movie: 27, tv: 9648 },
    "Phiêu lưu": { movie: 12, tv: 10759 },
    "Hình sự": { movie: 80, tv: 80 },
    "Tâm lý": { movie: 18, tv: 18 },
    "Hoạt hình": { movie: 16, tv: 16 },
    "Chiến tranh": { movie: 10752, tv: 10768 },
  };
  const keywordIds: Record<string, number> = {
    // TMDB keyword: https://www.themoviedb.org/keyword/10617-disaster/movie
    "Thảm hoạ": 10617,
  };
  const hotWindow: Record<string, string> = { "Hôm nay": "day", "Tuần này": "week", "Tháng này": "all", "Tất cả": "all" };
  const genreLabels: Record<UiLanguage, Record<string, string>> = {
    vi: { "Tất cả": "Tất cả", "Hành động": "Hành động", "Viễn tưởng": "Viễn tưởng", "Kinh dị": "Kinh dị", "Phiêu lưu": "Phiêu lưu", "Hình sự": "Hình sự", "Tâm lý": "Tâm lý", "Hoạt hình": "Hoạt hình", "Chiến tranh": "Chiến tranh", "Thảm hoạ": "Thảm hoạ" },
    en: { "Tất cả": "All", "Hành động": "Action", "Viễn tưởng": "Sci‑Fi", "Kinh dị": "Horror", "Phiêu lưu": "Adventure", "Hình sự": "Crime", "Tâm lý": "Drama", "Hoạt hình": "Animation", "Chiến tranh": "War", "Thảm hoạ": "Disaster" },
  };
  const hotLabels: Record<UiLanguage, Record<string, string>> = {
    vi: { "Hôm nay": "Hôm nay", "Tuần này": "Tuần này", "Tháng này": "Tháng này", "Tất cả": "Tất cả" },
    en: { "Hôm nay": "Today", "Tuần này": "This week", "Tháng này": "This month", "Tất cả": "All" },
  };
  const discoverLabels = {
    vi: { genre: "Thể loại", hot: "Nội dung nổi bật" },
    en: { genre: "Genres", hot: "Trending" },
  } as const;
  const mediaTypes = ["Film", "TV", "Film bộ"];
  const hotRanges = ["Hôm nay", "Tuần này", "Tháng này", "Tất cả"];

  let uiLanguage = $state<UiLanguage>("vi");
  let d = $derived(discoverLabels[uiLanguage]);
  let query = $state("");
  let mobileLinkQuery = $state("");
  let recent = $state<string[]>([]);
  let message = $state("Đang tải dữ liệu Discovery thật...");
  let activeGenre = $state("Tất cả");
  let activeType = $state("Film");
  let activeHot = $state("Hôm nay");
  let loading = $state(true);
  let searching = $state(false);
  let loadingMore = $state(false);
  let discoveryPage = $state(1);
  let searchPage = $state(1);
  let canLoadMore = $state(true);
  let recommendations = $state<DiscoverItem[]>([]);
  let trending = $state<DiscoverItem[]>([]);
  let searchResults = $state<DiscoverItem[]>([]);
  let fshareLinks = $state<FshareResult[]>([]);
  let selectedFilm = $state<DiscoverItem | null>(null);
  let selectedLinks = $state<FshareResult[]>([]);
  let selectedDownloadUrls = $state<string[]>([]);
  let selectedLinkIndex = $state<number | null>(null);
  let seriesMode = $state(false);
  let showSeriesHelp = $state(false);
  let selectedCast = $state<CastMember[]>([]);
  let relatedFilms = $state<RelatedFilm[]>([]);
  let loadingMeta = $state(false);
  let hasScannedLinks = $state(false);
  let loadingLinks = $state(false);
  let addingLinkUrl = $state<string | null>(null);
  let addedLinkUrls = $state<string[]>([]);
  let linkErrors = $state<Record<string, string>>({});
  let confirmDownloadLink = $state<FshareResult | null>(null);
  let confirmDownloadLinks = $state<FshareResult[]>([]);
  let showDownloadConfirm = $state(false);
  let showTrailerModal = $state(false);
  let selectedTrailerKey = $state<string | null>(null);
  let trailerModalRef = $state<HTMLDivElement | null>(null);
  let trailerIframeRef = $state<HTMLIFrameElement | null>(null);
  let selectedImages = $state<string[]>([]);
  let selectedImageIndex = $state(0);
  let imageTimer: number | null = null;
  let bookmarkedKeys = $state<string[]>([]);
  let filmInfoPanelEl: HTMLElement | null = $state(null);
  let pendingOpenRef = $state<string | null>(null);
  let overviewExpanded = $state(false);
  let heroSlideIndex = $state(0);
  let heroSlideTimer: number | null = null;
  let posterGridColumns = $state(5);

  const mediaType = $derived(activeType === "TV" || activeType === "Film bộ" ? "tv" : "movie");
  const isTvSeriesSource = $derived(activeType === "Film bộ");
  const displayed = $derived(searchResults.length ? searchResults : recommendations);
  const displayedGrid = $derived(trimToGridMultiple(displayed, posterGridColumns));
  const heroSlides = $derived((trending.length ? trending : displayed).filter((item) => item.heroImg || item.img).slice(0, 6));
  const activeHeroSlide = $derived(heroSlides[heroSlideIndex % Math.max(heroSlides.length, 1)]);
  const activeHeroImage = $derived(selectedImages[selectedImageIndex] || selectedFilm?.img || fallbackPoster);
  onMount(() => {
    syncLanguage();
    const syncPosterGridColumns = () => {
      posterGridColumns = window.matchMedia("(max-width: 720px)").matches ? 3 : window.matchMedia("(max-width: 1360px)").matches ? 4 : 5;
    };
    syncPosterGridColumns();
    window.addEventListener("resize", syncPosterGridColumns);
    const onLanguageChange = (event: Event) => {
      const next = (event as CustomEvent<UiLanguage>).detail;
      if (next === "vi" || next === "en") {
        uiLanguage = next;
        if (selectedFilm?.id) void loadFilmMeta(selectedFilm);
      }
    };
    const resyncLanguage = () => syncLanguage();
    window.addEventListener("fhub-language-change", onLanguageChange);
    window.addEventListener("focus", resyncLanguage);
    document.addEventListener("visibilitychange", resyncLanguage);
    loadBookmarkKeys();
    heroSlideTimer = window.setInterval(() => {
      if (heroSlides.length > 1) heroSlideIndex = (heroSlideIndex + 1) % heroSlides.length;
    }, 3600);
    const params = new URLSearchParams(window.location.search);
    const queryParam = params.get("q")?.trim();
    const mediaTypeParam = params.get("media_type");
    const openParam = params.get("open");
    pendingOpenRef = openParam;
    if (mediaTypeParam === "tv" || mediaTypeParam === "movie") {
      activeType = mediaTypeParam === "tv" ? "TV" : "Film";
    }
    if (queryParam) {
      query = queryParam;
      void runSearch(queryParam, 1, false);
    } else {
      void loadDiscovery();
    }
    return () => {
      window.removeEventListener("fhub-language-change", onLanguageChange);
      window.removeEventListener("focus", resyncLanguage);
      window.removeEventListener("resize", syncPosterGridColumns);
      document.removeEventListener("visibilitychange", resyncLanguage);
      if (heroSlideTimer) window.clearInterval(heroSlideTimer);
    };
  });

  $effect(() => {
    const queryParam = page.url.searchParams.get("q")?.trim();
    if (queryParam && queryParam !== query) {
      query = queryParam;
      void runSearch(queryParam, 1, false);
    }
  });

  $effect(() => {
    const queryParam = page.url.searchParams.get("q")?.trim();
    if (!queryParam && query) query = "";
  });

  function bookmarkKey(item?: DiscoverItem | null) {
    if (!item) return "";
    return `${item.type === "TV" ? "tv" : "movie"}:${item.id || item.title}`;
  }

  function loadBookmarkKeys() {
    try {
      const parsed = JSON.parse(localStorage.getItem(BOOKMARKS_KEY) || "[]");
      bookmarkedKeys = Array.isArray(parsed) ? parsed.map((item: any) => item.fcode || `${item.mediaType || "movie"}:${item.id || item.title}`) : [];
    } catch {
      bookmarkedKeys = [];
    }
  }

  function toggleBookmark(item?: DiscoverItem | null) {
    if (!item) return;
    const key = bookmarkKey(item);
    try {
      const parsed = JSON.parse(localStorage.getItem(BOOKMARKS_KEY) || "[]");
      const items = Array.isArray(parsed) ? parsed : [];
      const exists = items.some((saved: any) => (saved.fcode || `${saved.mediaType || "movie"}:${saved.id || saved.title}`) === key);
      const next = exists
        ? items.filter((saved: any) => (saved.fcode || `${saved.mediaType || "movie"}:${saved.id || saved.title}`) !== key)
        : [{
            id: item.id,
            title: item.title,
            posterUrl: item.img,
            mediaType: item.type === "TV" ? "tv" : "movie",
            releaseDate: item.year,
            originalFilename: item.originalTitle || item.title,
            quality: item.seed,
          }, ...items].slice(0, 100);
      localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(next));
      bookmarkedKeys = next.map((saved: any) => saved.fcode || `${saved.mediaType || "movie"}:${saved.id || saved.title}`);
    } catch {
      // ignore bookmark write failures
    }
  }

  function syncLanguage() {
    try {
      const saved = localStorage.getItem("fhub-ui-language");
      if (saved === "vi" || saved === "en") uiLanguage = saved;
    } catch {
      // ignore
    }
  }

  function formatSize(bytes?: number) {
    if (!bytes) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = bytes;
    let idx = 0;
    while (size >= 1024 && idx < units.length - 1) {
      size /= 1024;
      idx += 1;
    }
    return `${size.toFixed(size >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`;
  }

  function extractYear(...values: Array<unknown>) {
    for (const value of values) {
      if (value === null || value === undefined) continue;
      const text = String(value);
      const dateYear = text.match(/^(19|20)\d{2}/)?.[0];
      if (dateYear) return dateYear;
      const bracketYear = text.match(/[\[(](19|20)\d{2}[\])]/)?.[0]?.replace(/\D/g, "");
      if (bracketYear) return bracketYear;
      const looseYear = text.match(/\b(19|20)\d{2}\b/)?.[0];
      if (looseYear) return looseYear;
    }
    return undefined;
  }

  function yearFromDate(value?: string) {
    return extractYear(value);
  }

  function isFshareUrl(value?: string | null) {
    return /https?:\/\/(www\.)?fshare\.vn\/(file|folder)\//i.test((value || '').trim());
  }

  function isFshareFolderUrl(value?: string | null) {
    return /https?:\/\/(www\.)?fshare\.vn\/folder\//i.test((value || '').trim());
  }

  function routeSearchValue(value: string) {
    const clean = value.trim();
    if (!clean) return;
    if (isFshareUrl(clean)) {
      window.location.href = `/downloads?url=${encodeURIComponent(clean)}`;
      return;
    }
    void runSearch(clean, 1, false);
  }


  function mapFsharePreviewItems(preview: any, sourceUrl: string): DiscoverItem[] {
    const items = Array.isArray(preview?.items) ? preview.items : [];
    const folderName = preview?.folder_name || preview?.name || 'FShare folder';
    if (!items.length) {
      return [{
        id: undefined,
        title: preview?.title || folderName,
        type: 'FShare',
        size: preview?.total_size ? formatSize(preview.total_size) : undefined,
        seed: 'Link FShare',
        img: '',
        heroImg: '',
        overview: preview?.resolved_url || preview?.original_url || sourceUrl,
        fshareCount: preview?.file_count || 1,
        url: preview?.resolved_url || preview?.original_url || sourceUrl,
        originalTitle: folderName,
      }];
    }
    return items.map((item: any, index: number) => ({
      id: undefined,
      title: item.name || `${folderName} #${index + 1}`,
      year: extractYear(item.name, item.title),
      type: 'FShare',
      size: formatSize(item.size),
      seed: item.quality || item.resolution || item.source || 'Link FShare',
      img: '',
      heroImg: '',
      overview: item.url || item.link || sourceUrl,
      fshareCount: items.length,
      url: item.url || item.link || sourceUrl,
      originalTitle: item.title || folderName,
    }));
  }



  function pickTrailerKey(detail: any) {
    const videos = detail?.videos?.results || [];
    const trailer = videos.find((video: any) => video.site === "YouTube" && video.type === "Trailer" && video.official) ||
      videos.find((video: any) => video.site === "YouTube" && video.type === "Trailer") ||
      videos.find((video: any) => video.site === "YouTube");
    return trailer?.key || null;
  }

  function isMobileViewport() {
    return typeof window !== "undefined" && window.matchMedia("(max-width: 720px)").matches;
  }

  async function enterTrailerFullscreen() {
    if (!isMobileViewport()) return;
    await tick();
    try {
      await trailerIframeRef?.requestFullscreen?.();
      return;
    } catch {
      // Safari may refuse iframe fullscreen unless the YouTube player owns the gesture.
    }
    try {
      await trailerModalRef?.requestFullscreen?.();
    } catch {
      // CSS fallback still renders the trailer as a full-screen overlay.
    }
  }

  function sendTrailerCommand(func: string, args: unknown[] = []) {
    try {
      trailerIframeRef?.contentWindow?.postMessage(JSON.stringify({ event: "command", func, args }), "https://www.youtube.com");
    } catch {
      // YouTube/Safari may ignore scripted audio commands; the player still remains usable manually.
    }
  }

  async function primeTrailerPlayback() {
    await tick();
    sendTrailerCommand("setVolume", [100]);
    sendTrailerCommand("unMute");
    sendTrailerCommand("playVideo");
    window.setTimeout(() => {
      sendTrailerCommand("setVolume", [100]);
      sendTrailerCommand("unMute");
      sendTrailerCommand("playVideo");
    }, 250);
  }

  function openTrailer() {
    if (!selectedTrailerKey) {
      message = "Chưa có trailer cho phim này.";
      return;
    }
    document.body.classList.add("fhub-trailer-open");
    showTrailerModal = true;
    void enterTrailerFullscreen();
    void primeTrailerPlayback();
  }

  async function closeTrailer() {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
    } catch {}
    document.body.classList.remove("fhub-trailer-open");
    showTrailerModal = false;
  }

  function patchFilmYearInLists(target: DiscoverItem, year?: string) {
    if (!target.id || !year) return;
    const key = `${target.type}:${target.id}`;
    const patch = (item: DiscoverItem) => `${item.type}:${item.id}` === key ? { ...item, year: item.year || year } : item;
    recommendations = recommendations.map(patch);
    searchResults = searchResults.map(patch);
    trending = trending.map(patch);
  }

  async function hydrateMissingYears(items: DiscoverItem[]) {
    const targets = items.filter((item) => item.id && item.type !== "FShare" && (!item.year || !item.heroImg || !item.overview)).slice(0, 24);
    if (!targets.length) return;
    const updates = await Promise.all(targets.map(async (item) => {
      try {
        const kind = item.type === "TV" ? "tv" : "movie";
        const response = await fetch(`/api/tmdb/${kind}/${item.id}?language=${uiLanguage === "vi" ? "vi-VN" : "en-US"}`);
        if (!response.ok) return null;
        const detail = await response.json();
        const year = extractYear(detail.release_date, detail.first_air_date, detail.title, detail.name);
        const heroImg = tmdbImage(detail.backdrop_path, "w1280") || tmdbImage(detail.poster_path, "w780");
        return { key: `${item.type}:${item.id}`, year, heroImg, overview: detail.overview };
      } catch {
        return null;
      }
    }));
    const updateMap = new Map(updates.filter(Boolean).map((entry: any) => [entry.key, entry]));
    if (!updateMap.size) return;
    const apply = (item: DiscoverItem) => {
      if (!item.id) return item;
      const update = updateMap.get(`${item.type}:${item.id}`);
      return update ? { ...item, year: item.year || update.year, heroImg: item.heroImg || update.heroImg, overview: item.overview || update.overview } : item;
    };
    recommendations = recommendations.map(apply);
    searchResults = searchResults.map(apply);
    trending = trending.map(apply);
    if (selectedFilm?.id) selectedFilm = apply(selectedFilm);
  }

  function mapPopular(item: any): DiscoverItem {
    const title = item.title || item.name || item.original_title || item.original_name || "Không rõ tên";
    return {
      id: item.id,
      title,
      year: extractYear(item.year, item.release_date, item.first_air_date, title, item.original_title, item.original_name),
      score: Number(item.vote_average || item.score || 0).toFixed(1),
      type: item.media_type === "tv" ? "TV" : "Movie",
      seed: "TMDB",
      size: "Metadata",
      img: optimizePosterUrl(item.poster_url || tmdbImage(item.poster_path, "w342"), "w342"),
      heroImg: item.backdrop_url || tmdbImage(item.backdrop_path, "w1280"),
      overview: item.overview,
      fshareCount: item.fshare_count || 0,
    };
  }

  function mapTmdbSearch(item: any): DiscoverItem {
    const title = item.title || item.name || item.original_title || item.original_name || "Không rõ tên";
    return {
      id: item.id,
      title,
      originalTitle: item.original_title || item.original_name || title,
      year: extractYear(item.release_date, item.first_air_date, title, item.original_title, item.original_name),
      score: item.vote_average ? Number(item.vote_average).toFixed(1) : undefined,
      type: item.media_type === "tv" || mediaType === "tv" ? "TV" : "Movie",
      seed: "TMDB",
      size: "Metadata",
      img: tmdbImage(item.poster_path, "w342") || fallbackPoster,
      heroImg: tmdbImage(item.backdrop_path, "w1280"),
      overview: item.overview,
    };
  }

  function isThuviencineTvItem(item: any) {
    return item.media_type === "tv" && (
      Boolean(item.original_filename) ||
      /thuviencine\./i.test(String(item.url || "")) ||
      /\s[–-]\s/.test(String(item.name || item.title || ""))
    );
  }

  function displayVietnameseTitle(item: any) {
    const original = item.original_filename || item.name || item.title || "";
    const beforeDash = original.split("–")[0]?.split(" - ")[0]?.trim() || original.trim();
    return beforeDash.replace(/:\s*Phần\s+\d+(?:\s*-\s*\d+)?\s*$/i, "").trim() || item.name || item.tmdb_title || "Không rõ tên";
  }

  function mapTrending(item: any): DiscoverItem {
    const isTvFromThuviencine = isThuviencineTvItem(item);
    return {
      id: item.tmdb_id,
      title: isTvFromThuviencine && uiLanguage === "vi" ? displayVietnameseTitle(item) : (item.tmdb_title || item.name || item.original_filename || "Không rõ tên"),
      originalTitle: item.tmdb_title || item.parsed_name || item.name || item.original_filename,
      year: extractYear(item.year, item.release_date, item.first_air_date, item.tmdb_title, item.name, item.original_filename, item.title),
      score: item.vote_average ? Number(item.vote_average).toFixed(1) : undefined,
      type: item.media_type === "tv" ? "TV" : "Movie",
      seed: item.quality || "FShare",
      size: formatSize(Number(item.size || 0)),
      img: optimizePosterUrl(item.poster_url, "w342"),
      heroImg: item.backdrop_url || tmdbImage(item.backdrop_path, "w1280"),
      overview: isTvFromThuviencine ? undefined : item.original_filename,
      url: item.url,
    };
  }

  function mapFshare(item: FshareResult): DiscoverItem {
    return {
      title: item.name || item.original_name || "Kết quả FShare",
      type: "FShare",
      seed: item.quality || item.resolution || item.source || "Link FShare",
      size: formatSize(item.size),
      img: fallbackPoster,
      overview: item.original_name,
      url: item.url,
    };
  }

  function tmdbImage(path?: string | null, size = "w185") {
    return path ? `https://image.tmdb.org/t/p/${size}${path}` : undefined;
  }

  function optimizePosterUrl(url?: string | null, size = "w342") {
    if (!url) return fallbackPoster;
    return url.replace(/https:\/\/image\.tmdb\.org\/t\/p\/(original|w\d+)/, `https://image.tmdb.org/t/p/${size}`);
  }

  function posterSrcSet(url?: string | null) {
    if (!url || !url.includes("image.tmdb.org/t/p/")) return undefined;
    const small = optimizePosterUrl(url, "w185");
    const medium = optimizePosterUrl(url, "w342");
    const large = optimizePosterUrl(url, "w500");
    return `${small} 185w, ${medium} 342w, ${large} 500w`;
  }

  function mapRelatedFilm(item: any): RelatedFilm {
    return {
      id: item.id,
      title: item.title || item.name || "Không rõ tên",
      year: yearFromDate(item.release_date || item.first_air_date),
      score: item.vote_average ? Number(item.vote_average).toFixed(1) : undefined,
      poster: tmdbImage(item.poster_path, "w342"),
      overview: item.overview,
    };
  }

  async function loadDiscovery(page = 1, append = false) {
    loading = !append;
    loadingMore = append;
    message = append ? "Đang tải thêm phim..." : "Đang tải phim mới nhất từ API thật...";
    try {
      const genre = activeGenre !== "Tất cả" ? genreIds[activeGenre]?.[mediaType] : undefined;
      const keyword = activeGenre !== "Tất cả" ? keywordIds[activeGenre] : undefined;
      const window = hotWindow[activeHot] || "day";
      const params = new URLSearchParams({ media_type: mediaType, limit: "24", window, page: String(page) });
      if (genre) params.set("genre", String(genre));
      if (keyword) params.set("keyword", String(keyword));

      const popularUrl = isTvSeriesSource
        ? `/api/discovery/thuviencine-tv?limit=24&page=${page}`
        : keyword
          ? `https://api.themoviedb.org/3/discover/${mediaType}?api_key=8d95150f3391194ca66fef44df497ad6&sort_by=popularity.desc&with_keywords=${keyword}&page=${page}`
          : `/api/discovery/popular-today?${params.toString()}`;
      const [popularResponse, trendingResponse] = await Promise.all([
        fetch(popularUrl),
        isTvSeriesSource ? Promise.resolve(null) : fetch("/api/discovery/trending"),
      ]);

      if (!popularResponse.ok) throw new Error(await popularResponse.text() || "Không tải được TMDB popular");
      const popularPayload = await popularResponse.json();
      const nextItems = (popularPayload.results || []).map(isTvSeriesSource ? mapTrending : mapPopular);
      if (append) {
        const seen = new Set(recommendations.map((item: DiscoverItem) => `${item.type || mediaType}:${item.id || item.title}`));
        const uniqueNext = nextItems.filter((item: DiscoverItem) => {
          const key = `${item.type || mediaType}:${item.id || item.title}`;
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
        recommendations = [...recommendations, ...uniqueNext];
        canLoadMore = uniqueNext.length > 0;
        void hydrateMissingYears(uniqueNext);
      } else {
        recommendations = nextItems;
        canLoadMore = nextItems.length >= 20;
      }
      discoveryPage = page;

      if (isTvSeriesSource) {
        trending = nextItems.slice(0, 12);
      } else if (trendingResponse?.ok) {
        const trendingPayload = await trendingResponse.json();
        trending = (trendingPayload.results || []).map(mapTrending);
      } else {
        trending = [];
      }

      message = `Đã tải ${recommendations.length} phim theo bộ lọc ${activeGenre} / ${activeHot}.`;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Không tải được Discovery";
      message = `Lỗi tải Discovery: ${msg}`;
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  async function runSearch(value = query, page = 1, append = false) {
    const clean = value.trim();
    if (!clean) {
      searchResults = [];
      fshareLinks = [];
      message = "Nhập tên phim hoặc series để tìm.";
      return;
    }

    query = clean;
    recent = [clean, ...recent.filter((item) => item.toLowerCase() !== clean.toLowerCase())].slice(0, 6);
    searching = !append;
    loadingMore = append;
    message = append ? `Đang tìm thêm phim cho "${clean}"...` : `Đang tìm FShare cho "${clean}"...`;

    try {
      if (isFshareUrl(clean)) {
        message = isFshareFolderUrl(clean) ? "Đang đọc folder FShare..." : "Đang đọc link FShare...";
        const previewResponse = await fetch(`/api/downloads/preview-link`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          credentials: "include",
          body: JSON.stringify({ url: clean, recursive: false }),
        });
        const preview = previewResponse.ok ? await previewResponse.json() : { success: false, message: await previewResponse.text() };
        if (!previewResponse.ok || preview?.success === false) throw new Error(preview?.message || "Không đọc được link FShare");
        const mapped = mapFsharePreviewItems(preview, clean);
        fshareLinks = [];
        searchResults = mapped;
        canLoadMore = false;
        searchPage = 1;
        message = mapped.length ? `Đã đọc ${mapped.length} file từ FShare.` : "Folder FShare không có file hiển thị.";
        return;
      }

      const searchTerms = [clean, ...aliasSearchQueries(clean)];
      const fetchSearch = (term: string, language: string) => fetch(`/api/tmdb/search?media_type=${mediaType}&q=${encodeURIComponent(term)}&language=${language}&page=${page}`);
      const responses = await Promise.all(searchTerms.flatMap((term) => [fetchSearch(term, "vi-VN"), fetchSearch(term, "en-US")]));
      const payloads = await Promise.all(responses.map(async (response) => (response.ok ? response.json() : { results: [] })));
      const mergedResults = payloads.flatMap((payload) => payload.results || []);
      if (!mergedResults.length) throw new Error("Không tìm được thông tin phim");

      const seen = new Set();
      const merged = mergedResults.filter((item) => {
        const key = `${item.id}:${item.media_type || mediaType}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });

      fshareLinks = [];
      const nextItems = merged.slice(0, 24).map(mapTmdbSearch);

      if (append) {
        const seen = new Set(searchResults.map((item) => `${item.type || mediaType}:${item.id || item.title}`));
        const uniqueNext = nextItems.filter((item) => {
          const key = `${item.type || mediaType}:${item.id || item.title}`;
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
        searchResults = [...searchResults, ...uniqueNext];
        canLoadMore = uniqueNext.length > 0;
        void hydrateMissingYears(uniqueNext);
      } else {
        searchResults = nextItems;
        canLoadMore = nextItems.length >= 20;
        void hydrateMissingYears(nextItems);
        const target = pendingOpenRef;
        if (target && nextItems.length) {
          const [targetType, targetId] = target.split(":");
          const match = nextItems.find((item) => String(item.id || "") === targetId && (item.type === "TV" ? "tv" : "movie") === targetType) || nextItems[0];
          pendingOpenRef = null;
          setTimeout(() => void openMovie(match), 0);
        }
      }
      searchPage = page;
      message = searchResults.length ? `Tìm thấy ${searchResults.length} phim/series cho "${clean}". Chọn phim rồi bấm Get link.` : `Chưa tìm thấy phim/series cho "${clean}".`;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Tìm kiếm thất bại";
      message = `Lỗi tìm kiếm: ${msg}`;
    } finally {
      searching = false;
      loadingMore = false;
    }
  }


  function stopImageCarousel() {
    if (imageTimer) window.clearInterval(imageTimer);
    imageTimer = null;
  }

  function startImageCarousel() {
    stopImageCarousel();
    if (selectedImages.length <= 1) return;
    imageTimer = window.setInterval(() => {
      selectedImageIndex = (selectedImageIndex + 1) % selectedImages.length;
    }, 3600);
  }

  function setSelectedImages(images: string[]) {
    const unique = [...new Set(images.filter(Boolean))];
    selectedImages = unique.length ? unique : selectedFilm?.img ? [selectedFilm.img] : [];
    selectedImageIndex = 0;
    startImageCarousel();
  }

  async function openMovie(item: DiscoverItem) {
    selectedFilm = item;
    overviewExpanded = false;
    setSelectedImages(item.img ? [item.img] : []);
    selectedLinks = [];
    selectedDownloadUrls = [];
    selectedLinkIndex = null;
    selectedCast = [];
    relatedFilms = [];
    hasScannedLinks = false;
    message = `Đang xem thông tin ${item.title}. Bấm Get link để lấy bản tải FShare.`;
    await tick();
    filmInfoPanelEl?.scrollIntoView({ behavior: "smooth", block: "start" });
    await loadFilmMeta(item);
  }

  async function loadFilmMeta(item: DiscoverItem) {
    if (!item.id || item.type === "FShare") return;
    loadingMeta = true;
    const kind = item.type === "TV" ? "tv" : "movie";
    try {
      const [detailResponse, similarResponse] = await Promise.all([
        fetch(`/api/tmdb/${kind}/${item.id}?language=${uiLanguage === "vi" ? "vi-VN" : "en-US"}&append_to_response=images`),
        fetch(`/api/tmdb/${kind}/${item.id}/similar?page=1&language=${uiLanguage === "vi" ? "vi-VN" : "en-US"}`),
      ]);
      if (detailResponse.ok) {
        const detail = await detailResponse.json();
        selectedCast = (detail.credits?.cast || []).slice(0, 8).map((cast: any) => ({
          id: cast.id,
          name: cast.name || cast.original_name || "Diễn viên",
          character: cast.character,
          profile: tmdbImage(cast.profile_path, "w185"),
        }));
        const detailYear = extractYear(detail.release_date, detail.first_air_date, selectedFilm?.year, detail.title, detail.name);
        selectedTrailerKey = pickTrailerKey(detail);
        patchFilmYearInLists(item, detailYear);
        if (selectedFilm?.id === item.id) {
          selectedFilm = {
            ...selectedFilm,
            overview: detail.overview || selectedFilm.overview,
            year: detailYear || selectedFilm.year,
            score: detail.vote_average ? Number(detail.vote_average).toFixed(1) : selectedFilm.score,
            img: tmdbImage(detail.backdrop_path, "w780") || selectedFilm.img,
          };
          const heroImages = [
            tmdbImage(detail.backdrop_path, "w1280"),
            ...(detail.images?.backdrops || []).slice(0, 8).map((image: any) => tmdbImage(image.file_path, "w1280")),
            tmdbImage(detail.poster_path, "w780"),
            ...(detail.images?.posters || []).slice(0, 4).map((image: any) => tmdbImage(image.file_path, "w780")),
            selectedFilm.img,
          ];
          setSelectedImages(heroImages);
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


  function normalizeTitleText(value?: string | null) {
    return (value || "")
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/đ/g, "d")
      .replace(/[^a-z0-9]+/g, " ")
      .trim();
  }

  function aliasSearchQueries(value: string) {
    const normalized = normalizeTitleText(value);
    const aliases: Record<string, string[]> = {
      "nguoi sat": ["Iron Man", "Tony Stark"],
      "ironman": ["Iron Man"],
      "iron man": ["Iron Man"],
      "tony stark": ["Iron Man", "Tony Stark"],
      "nguoi nhen": ["Spider-Man", "Spiderman"],
      "spiderman": ["Spider-Man", "Spiderman"],
      "spider man": ["Spider-Man", "Spiderman"],
      "nguoi doi": ["Batman", "The Dark Knight"],
      "batman": ["Batman", "The Dark Knight"],
      "nguoi bi sat": ["The Wolverine", "Wolverine"],
      "nguoi dan ong": ["The Batman", "Batman"],
      "sieu nhan": ["Superman", "Man of Steel"],
      "nguoi khong lo xanh": ["The Hulk", "Hulk"],
      "ke huy diet": ["Terminator"],
      "qua nhanh qua nguy hiem": ["Fast & Furious", "Fast and Furious"],
      "nhiem vu bat kha thi": ["Mission Impossible", "Mission: Impossible"],
      "cuop bien caribe": ["Pirates of the Caribbean"],
      "chien tranh giua cac vi sao": ["Star Wars"],
      "chua te nhung chiec nhan": ["The Lord of the Rings"],
      "nguoi van chuyen": ["The Transporter", "Transporter"],
      "sat thu john wick": ["John Wick"],
      "phi hanh gia": ["Interstellar", "Gravity"],
      "chien binh vu tru": ["Guardians of the Galaxy"],
    };
    return aliases[normalized] || [];
  }

  function titleTokens(value?: string | null) {
    const stop = new Set(["the", "a", "an", "and", "of", "in", "on", "to", "va", "cua", "cho", "mot", "bo", "ban", "tap", "phim", "movie", "tv", "fshare", "bluray", "web", "dl", "1080p", "720p", "2160p", "4k"]);
    return normalizeTitleText(value).split(/\s+/).filter((token) => token.length >= 3 && !stop.has(token));
  }

  function scoreLinkAgainstTitle(link: FshareResult, title?: string | null) {
    const linkName = link.original_name || link.name || "";
    const linkNorm = normalizeTitleText(linkName);
    const aliasNorm = normalizeTitleText(title);
    const tokens = titleTokens(title);
    if (!aliasNorm || !tokens.length) return 0;
    if (linkNorm.includes(aliasNorm)) return 1;
    const matched = tokens.filter((token) => linkNorm.includes(token)).length;
    return matched / tokens.length;
  }

  function linkRelevance(link: FshareResult) {
    const originalScore = scoreLinkAgainstTitle(link, selectedFilm?.originalTitle);
    const titleScore = scoreLinkAgainstTitle(link, selectedFilm?.title);
    let best = Math.max(originalScore, titleScore);
    const year = selectedFilm?.year;
    const linkNorm = normalizeTitleText(link.original_name || link.name || "");
    if (year && linkNorm.includes(year)) best += 0.18;
    return Math.min(best, 1.18);
  }

  function filterRelevantLinks(results: FshareResult[]) {
    if (!selectedFilm || !results.length) return results;
    const hasOriginal = !!selectedFilm.originalTitle && normalizeTitleText(selectedFilm.originalTitle) !== normalizeTitleText(selectedFilm.title);
    const scored = results.map((link) => ({
      link,
      score: linkRelevance(link),
      originalScore: scoreLinkAgainstTitle(link, selectedFilm?.originalTitle),
      titleScore: scoreLinkAgainstTitle(link, selectedFilm?.title),
    })).sort((a, b) => b.score - a.score);

    // For bilingual TV-series sources, Vietnamese titles can be too generic (e.g. "Kẻ Thù...").
    // If an English/original title exists, require it so we don't mix in unrelated FShare results.
    const strict = hasOriginal
      ? scored.filter((item) => item.originalScore >= 0.65).map((item) => item.link)
      : scored.filter((item) => item.score >= 0.55).map((item) => item.link);
    const filtered = strict.length
      ? strict
      : scored.filter((item) => item.score >= 0.35 && selectedFilm?.year && normalizeTitleText(item.link.original_name || item.link.name).includes(selectedFilm.year)).map((item) => item.link);
    return dedupeDownloadLinks(filtered);
  }

  function episodeLabelFromName(value?: string | number | null) {
    const raw = String(value ?? "");
    const compact = raw.replace(/[._-]+/g, " ");
    const sxe = raw.match(/s(\d{1,2})\s*e(\d{1,3})/i) || compact.match(/s\s*(\d{1,2})\s*e\s*(\d{1,3})/i);
    if (sxe) return `S${sxe[1].padStart(2, "0")}E${sxe[2].padStart(2, "0")}`;
    const ep = compact.match(/(?:ep|episode|tap|tập)\s*(\d{1,3})/i);
    if (ep) return `Tập ${ep[1].padStart(2, "0")}`;
    return "";
  }

  function linkQualityRank(link: FshareResult) {
    const text = `${link.quality || ""} ${link.resolution || ""} ${link.original_name || ""} ${link.name || ""}`.toLowerCase();
    if (/2160p|4k|uhd/.test(text)) return 4;
    if (/1080p|fhd/.test(text)) return 3;
    if (/720p|hd/.test(text)) return 2;
    return 1;
  }

  function dedupeDownloadLinks(links: FshareResult[]) {
    const byKey = new Map<string, FshareResult>();
    const keyFor = (link: FshareResult) => {
      const raw = String(link.original_name || link.name || "");
      const episode = episodeLabelFromName(raw);
      const quality = linkQualityRank(link);
      const normalized = normalizeTitleText(raw)
        .replace(/\b(2160p|1080p|720p|4k|uhd|fhd|bluray|blu ray|web dl|webrip|hdtv|nf|netflix|vie|sub|multi|x264|x265|hevc|h264|h265|aac|ddp|dd5|atmos)\b/g, " ")
        .replace(/\b\d+(gb|mb)\b/g, " ")
        .replace(/\s+/g, " ")
        .trim();
      const sizeBucket = link.size ? Math.round(Number(link.size) / (250 * 1024 * 1024)) : 0;
      // TV series should show/download one best candidate per episode.
      return episode ? `episode:${episode}` : `${normalized}|q${quality}|s${sizeBucket}`;
    };
    const better = (a: FshareResult, b: FshareResult) => {
      const scoreA = linkRelevance(a) * 100 + linkQualityRank(a) * 10 + (a.size || 0) / (1024 * 1024 * 1024);
      const scoreB = linkRelevance(b) * 100 + linkQualityRank(b) * 10 + (b.size || 0) / (1024 * 1024 * 1024);
      return scoreA >= scoreB ? a : b;
    };
    for (const link of links) {
      const key = keyFor(link);
      const previous = byKey.get(key);
      byKey.set(key, previous ? better(previous, link) : link);
    }
    return Array.from(byKey.values()).sort((a, b) => {
      const ea = episodeLabelFromName(a.original_name || a.name);
      const eb = episodeLabelFromName(b.original_name || b.name);
      return ea.localeCompare(eb, undefined, { numeric: true }) || linkQualityRank(b) - linkQualityRank(a) || (b.size || 0) - (a.size || 0);
    });
  }

  function dedupePreviewItems(items: PreviewItem[]) {
    const byKey = new Map<string, PreviewItem>();
    const episodeFromItem = (item: PreviewItem) => {
      if (item.season && item.episode) return `S${String(item.season).padStart(2, "0")}E${String(item.episode).padStart(2, "0")}`;
      return episodeLabelFromName(item.name);
    };
    const keyFor = (item: PreviewItem) => {
      const episode = episodeFromItem(item);
      const quality = linkQualityRank({ name: item.name, quality: item.quality, size: item.size });
      const normalized = normalizeTitleText(item.name)
        .replace(/\b(2160p|1080p|720p|4k|uhd|fhd|bluray|blu ray|web dl|webrip|hdtv|nf|netflix|vie|sub|multi|x264|x265|hevc|h264|h265|aac|ddp|dd5|atmos)\b/g, " ")
        .replace(/\b\d+(gb|mb)\b/g, " ")
        .replace(/\s+/g, " ")
        .trim();
      const sizeBucket = item.size ? Math.round(Number(item.size) / (250 * 1024 * 1024)) : 0;
      // Queue downloads should only add one best file per TV episode.
      return episode ? `episode:${episode}` : `${normalized}|q${quality}|s${sizeBucket}`;
    };
    const better = (a: PreviewItem, b: PreviewItem) => {
      const scoreA = linkQualityRank({ name: a.name, quality: a.quality, size: a.size }) * 10 + (a.size || 0) / (1024 * 1024 * 1024) + (a.season && a.episode ? 5 : 0);
      const scoreB = linkQualityRank({ name: b.name, quality: b.quality, size: b.size }) * 10 + (b.size || 0) / (1024 * 1024 * 1024) + (b.season && b.episode ? 5 : 0);
      return scoreA >= scoreB ? a : b;
    };
    for (const item of items) {
      const key = keyFor(item);
      const previous = byKey.get(key);
      byKey.set(key, previous ? better(previous, item) : item);
    }
    return Array.from(byKey.values()).sort((a, b) => {
      const ea = episodeFromItem(a);
      const eb = episodeFromItem(b);
      return ea.localeCompare(eb, undefined, { numeric: true }) || linkQualityRank({ name: b.name, quality: b.quality }) - linkQualityRank({ name: a.name, quality: a.quality }) || (b.size || 0) - (a.size || 0);
    });
  }

  function previewItemToFshareResult(item: PreviewItem): FshareResult {
    return {
      name: selectedFilm?.title || item.name || "FShare",
      original_name: item.name || selectedFilm?.title || "FShare",
      url: item.url,
      size: item.size || 0,
      score: 100,
      fcode: (item.url || "").split("/file/")[1]?.split(/[?&/]/)[0] || (item.url || "").split("/folder/")[1]?.split(/[?&/]/)[0] || "",
      quality: item.quality || undefined,
      resolution: item.quality || undefined,
      source: "FShare",
      viet_sub: false,
      viet_dub: false,
    };
  }

  async function expandFolderLinks(links: FshareResult[]) {
    const expanded: FshareResult[] = [];
    for (const link of links) {
      if (link.url && /fshare\.vn\/folder\//i.test(link.url)) {
        const previewResponse = await fetch("/api/downloads/preview-link", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          credentials: "include",
          body: JSON.stringify({ url: link.url, recursive: true }),
        });
        if (previewResponse.ok) {
          const preview = await previewResponse.json();
          const rawItems = (preview.items || []).filter((item: PreviewItem) => item.url && !item.is_directory);
          expanded.push(...dedupePreviewItems(rawItems).map(previewItemToFshareResult));
          continue;
        }
      }
      expanded.push(link);
    }
    return expanded;
  }

  function displayDownloadTitle(link: FshareResult) {
    const label = episodeLabelFromName(link.original_name || link.name);
    const base = selectedFilm?.title || link.name || "Bản tải FShare";
    return label ? `${base} · ${label}` : (link.name || link.original_name || "Bản tải FShare");
  }

  async function loadFilmLinks(title: string) {
    const clean = title.trim();
    if (!clean) return;
    loadingLinks = true;
    hasScannedLinks = true;
    try {
      let rawLinks: FshareResult[] = [];
      let usedExactSource = false;
      if (selectedFilm?.type === "TV" && selectedFilm?.url) {
        const sourceResponse = await fetch(`/api/discovery/resolve-source-link?url=${encodeURIComponent(selectedFilm.url)}`);
        if (sourceResponse.ok) {
          const sourcePayload = await sourceResponse.json();
          rawLinks = sourcePayload.results || [];
          usedExactSource = rawLinks.length > 0;
        }
      }
      if (!rawLinks.length) {
        const year = selectedFilm?.year ? `&year=${encodeURIComponent(selectedFilm.year)}` : "";
        const original = selectedFilm?.originalTitle && selectedFilm.originalTitle !== clean ? `&original_title=${encodeURIComponent(selectedFilm.originalTitle)}` : "";
        const limit = selectedFilm?.type === "TV" ? "&limit=100" : "";
        const response = await fetch(`/api/discovery/available-on-fshare?title=${encodeURIComponent(clean)}${year}${original}${limit}`);
        if (!response.ok) throw new Error(await response.text() || "Không tải được link liên quan");
        const payload = await response.json();
        rawLinks = payload.results || [];
      }
      if (usedExactSource) {
        message = "Đã lấy đúng folder nguồn, đang tách các tập trong folder...";
        selectedLinks = dedupeDownloadLinks(await expandFolderLinks(rawLinks));
      } else {
        selectedLinks = filterRelevantLinks(rawLinks);
      }
      selectedLinkIndex = null;
      selectedDownloadUrls = selectedLinks.map((link) => link.url || "").filter(Boolean);
      seriesMode = selectedLinks.length > 1 || selectedFilm?.type === "TV";
      const removed = rawLinks.length - selectedLinks.length;
      message = selectedLinks.length
        ? `${usedExactSource ? "Đã lấy link từ đúng nguồn phim" : "Tìm thấy"} ${selectedLinks.length} bản tải${removed > 0 ? `, đã lọc ${removed} link lệch/trùng.` : "."}`
        : `Chưa thấy bản tải đủ khớp cho ${clean}${rawLinks.length ? ` — đã ẩn ${rawLinks.length} link lệch/trùng.` : "."}`;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Không tải được link liên quan";
      message = msg;
      selectedLinks = [];
      selectedDownloadUrls = [];
      selectedLinkIndex = null;
    } finally {
      loadingLinks = false;
    }
  }

  function openDownloadConfirm(link: FshareResult) {
    if (!link.url) {
      message = "Bản tải này chưa có URL FShare hợp lệ.";
      return;
    }
    confirmDownloadLink = link;
    confirmDownloadLinks = [];
    showDownloadConfirm = true;
    message = "Kiểm tra thông tin rồi bấm Download để xác nhận tải vào NAS.";
  }

  function closeDownloadConfirm() {
    showDownloadConfirm = false;
    confirmDownloadLink = null;
    confirmDownloadLinks = [];
  }

  async function confirmRelatedDownload() {
    if (confirmDownloadLinks.length) {
      const links = confirmDownloadLinks;
      for (const link of links) {
        if (!addedLinkUrls.includes(link.url || "")) await addRelatedDownload(link);
      }
      const hasError = links.some((link) => link.url && linkErrors[link.url]);
      if (!hasError) closeDownloadConfirm();
      return;
    }
    if (!confirmDownloadLink) return;
    await addRelatedDownload(confirmDownloadLink);
    if (!linkErrors[confirmDownloadLink.url || ""]) closeDownloadConfirm();
  }

  function toggleSelectedDownload(link: FshareResult) {
    if (!link.url) return;
    selectedDownloadUrls = selectedDownloadUrls.includes(link.url)
      ? selectedDownloadUrls.filter((url) => url !== link.url)
      : [...selectedDownloadUrls, link.url];
  }

  function selectAllDownloads() {
    selectedDownloadUrls = selectedLinks.map((link) => link.url || "").filter(Boolean);
  }

  function clearSelectedDownloads() {
    selectedDownloadUrls = [];
  }

  function openSelectedDownloadConfirm() {
    const links = selectedLinks.filter((link) => link.url && selectedDownloadUrls.includes(link.url));
    if (!links.length) {
      message = "Chưa chọn tập nào để tải xuống.";
      return;
    }
    confirmDownloadLinks = links;
    confirmDownloadLink = links[0] ?? null;
    showDownloadConfirm = true;
    message = `Kiểm tra ${links.length} tập rồi bấm Download để xác nhận tải vào NAS.`;
  }

  async function addRelatedDownload(link: FshareResult) {
    if (!link.url) {
      message = "Bản tải này chưa có URL FShare hợp lệ.";
      return;
    }
    addingLinkUrl = link.url;
    linkErrors = { ...linkErrors, [link.url]: "" };
    try {
      message = `Đang thêm ${link.name || link.original_name || selectedFilm?.title || "bản tải"} vào hàng đợi...`;
      const batchId = `discovery-${Date.now()}`;
      const batchName = selectedFilm?.title || link.name || "Discovery";
      const category = selectedFilm?.type === "TV" ? "tv" : "movies";
      let itemsToAdd: PreviewItem[] = [{ name: link.original_name || link.name || selectedFilm?.title || "FShare", url: link.url, size: link.size || 0, quality: link.quality || link.resolution || undefined }];

      if (/fshare\.vn\/folder\//i.test(link.url)) {
        message = "Đang mở folder FShare để lấy danh sách file...";
        const previewResponse = await fetch("/api/downloads/preview-link", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          credentials: "include",
          body: JSON.stringify({ url: link.url, recursive: true }),
        });
        if (!previewResponse.ok) throw new Error(await previewResponse.text() || "Không đọc được folder FShare");
        const preview = await previewResponse.json();
        const rawPreviewItems = (preview.items || []).filter((item: PreviewItem) => item.url && !item.is_directory);
        itemsToAdd = dedupePreviewItems(rawPreviewItems);
        if (!itemsToAdd.length) throw new Error("Folder FShare không có file tải được.");
        const removedPreview = rawPreviewItems.length - itemsToAdd.length;
        message = `Đang thêm ${itemsToAdd.length} file từ folder vào queue${removedPreview > 0 ? `, đã lọc ${removedPreview} file trùng.` : ""}...`;
      }

      const results = await Promise.allSettled(itemsToAdd.map((item) => fetch("/api/downloads", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          url: item.url,
          filename: item.name,
          category: seriesMode ? "tv" : category,
          priority: "NORMAL",
          batch_id: batchId,
          batch_name: batchName,
          folder_name: seriesMode ? batchName : undefined,
          tmdb: selectedFilm?.id ? {
            tmdb_id: selectedFilm.id,
            media_type: selectedFilm.type === "TV" ? "tv" : "movie",
            title: selectedFilm.title,
            year: selectedFilm.year ? Number(selectedFilm.year) || undefined : undefined,
            season: item.season || undefined,
            episode: item.episode || undefined,
          } : undefined,
        }),
      })));
      const failed = results.filter((result) => result.status === "rejected" || (result.status === "fulfilled" && !result.value.ok));
      if (failed.length === itemsToAdd.length) {
        const first = failed[0];
        if (first?.status === "fulfilled") throw new Error(await first.value.text() || "Không thêm được file nào vào queue");
        throw new Error(first?.reason?.message || "Không thêm được file nào vào queue");
      }
      addedLinkUrls = Array.from(new Set([...addedLinkUrls, link.url]));
      message = failed.length
        ? `Đã thêm ${itemsToAdd.length - failed.length}/${itemsToAdd.length} file vào queue, ${failed.length} file lỗi.`
        : `Đã thêm ${itemsToAdd.length} file vào queue tải xuống.`;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Không thêm được vào hàng đợi tải";
      linkErrors = { ...linkErrors, [link.url]: errorMessage };
      message = errorMessage;
    } finally {
      addingLinkUrl = null;
    }
  }

  function closeFilmInfo() {
    selectedFilm = null;
    overviewExpanded = false;
    stopImageCarousel();
    selectedImages = [];
    selectedImageIndex = 0;
    selectedLinks = [];
    selectedLinkIndex = null;
    addingLinkUrl = null;
    addedLinkUrls = [];
    linkErrors = {};
    selectedCast = [];
    relatedFilms = [];
    hasScannedLinks = false;
  }

  function clearSearch() {
    query = "";
    searchResults = [];
    searchPage = 1;
    canLoadMore = true;
    fshareLinks = [];
    message = "Đã quay về danh sách phim mới nhất.";
  }

  function trimToGridMultiple(items: DiscoverItem[], multiple = 3) {
    if (!items.length) return items;
    if (items.length < multiple) return items;
    const count = items.length - (items.length % multiple);
    return count > 0 ? items.slice(0, count) : items;
  }

  function loadMoreFilms() {
    if (loadingMore) return;
    if (searchResults.length) {
      void runSearch(query, searchPage + 1, true);
    } else {
      void loadDiscovery(discoveryPage + 1, true);
    }
  }
</script>

<div class="discover-screen">
  <form class="mobile-link-search mobile-link-search-top" onsubmit={(event) => { event.preventDefault(); routeSearchValue(mobileLinkQuery); }} aria-label="Tìm link FShare hoặc tên phim">
    <span class="material-icons">link</span>
    <input
      bind:value={mobileLinkQuery}
      type="text"
      placeholder="Dán link FShare hoặc nhập tên phim..."
      autocomplete="off"
      autocapitalize="off"
      autocorrect="off"
      spellcheck="false"
      inputmode="text"
      enterkeyhint="search"
      disabled={searching}
    />
    <button class="check-link-button" type="submit" disabled={searching || !mobileLinkQuery.trim()} aria-label="Tìm kiếm">
      <span class="material-icons">search</span>
    </button>
  </form>

  {#if activeHeroSlide}
    <section class="discover-hero featured-hero">
      <button type="button" class="hero-wide-media" aria-label={`Mở ${activeHeroSlide.title}`} onclick={() => openMovie(activeHeroSlide)}>
        {#each heroSlides as item, index}
          <img class:active={index === heroSlideIndex % heroSlides.length} src={item.img || item.heroImg || fallbackPoster} alt="" loading={index === 0 ? "eager" : "lazy"} decoding="async" />
        {/each}
        <div class="hero-info-card">
          <span class="eyebrow">FSHARE TRENDING</span>
          <h1>{activeHeroSlide.title}</h1>
          <div class="hero-meta">
            <span>{activeHeroSlide.year || "Chưa rõ năm"}</span>
            <span>{activeHeroSlide.type || "Movie"}</span>
            {#if activeHeroSlide.score}<span>⭐ {activeHeroSlide.score}</span>{/if}
            {#if activeHeroSlide.seed}<span>{activeHeroSlide.seed}</span>{/if}
          </div>
          {#if activeHeroSlide.overview}<p>{activeHeroSlide.overview}</p>{/if}
          <span class="hero-open-action">Xem thông tin</span>
        </div>
        {#if heroSlides.length > 1}
          <div class="hero-dots" aria-hidden="true">
            {#each heroSlides as _, index}
              <span class:active={index === heroSlideIndex % heroSlides.length}></span>
            {/each}
          </div>
        {/if}
      </button>
    </section>
  {/if}

  <section class="discovery-controls" aria-label="Bộ lọc khám phá">
    <div class="type-switch" role="tablist" aria-label="Loại nội dung" style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));grid-auto-flow:column;">
      {#each mediaTypes as type}
        <button type="button" role="tab" aria-selected={activeType === type} class:active={activeType === type} onclick={() => { activeType = type; discoveryPage = 1; searchResults = []; closeFilmInfo(); pendingOpenRef = null; void loadDiscovery(1); }}>
          <strong>{type}</strong>
          <span class="material-icons">{type === "TV" ? "live_tv" : type === "Film bộ" ? "video_library" : "movie"}</span>
        </button>
      {/each}
    </div>

    <div class="filter-card">
      <div class="filter-section compact-genre-filter">
        <div class="filter-options">
          {#each filterGenres as genre}
            <button type="button" class:active={activeGenre === genre} onclick={() => { activeGenre = genre; discoveryPage = 1; searchResults = []; closeFilmInfo(); void loadDiscovery(1); }}>{genreLabels[uiLanguage][genre]}</button>
          {/each}
        </div>
      </div>
    </div>
  </section>

  {#if selectedFilm}
    <section class="film-info-panel" aria-label="Thông tin phim đã chọn" bind:this={filmInfoPanelEl}>
      <div class="film-main-info">
        <div class="film-hero-carousel">
          <img src={activeHeroImage} alt={selectedFilm.title} />
          {#if selectedImages.length > 1}
            <div class="image-dots" aria-label="Poster đang hiển thị">
              {#each selectedImages as _, index}
                <span class:active={index === selectedImageIndex}></span>
              {/each}
            </div>
          {/if}
        </div>
        <div class="film-info-copy">
          <div class="film-info-head">
            <div>
              <span class="eyebrow">Thông tin phim</span>
              <h2>{selectedFilm.title}</h2>
            </div>
            <button type="button" aria-label="Đóng thông tin phim" onclick={closeFilmInfo}><span class="material-icons">close</span></button>
          </div>
          <div class="film-meta">
            <span>{selectedFilm.year || "Chưa rõ năm"}</span>
            <span>{selectedFilm.type || "Movie"}</span>
            {#if selectedFilm.score}<span>⭐ {selectedFilm.score}</span>{/if}
            {#if selectedFilm.size}<span>{selectedFilm.size}</span>{/if}
          </div>
          {#if selectedFilm.overview}
            <div class="overview-block" class:expanded={overviewExpanded}>
              <p>{selectedFilm.overview}</p>
              <button type="button" class="overview-toggle" onclick={() => overviewExpanded = !overviewExpanded}>
                {overviewExpanded ? "Thu gọn" : "… Xem thêm"}
              </button>
            </div>
          {:else}
            <p>{uiLanguage === "vi" ? "Chưa có tóm tắt tiếng Việt cho phim này." : "No overview is available for this title yet."}</p>
          {/if}
          <div class="film-actions">
            <button type="button" class="primary-action" onclick={() => void loadFilmLinks(selectedFilm?.title || "")}>{loadingLinks ? "Đang lấy link..." : hasScannedLinks ? "Get link lại" : "Get link"}</button>
            <button type="button" class="trailer-action" onclick={openTrailer} disabled={!selectedTrailerKey}><span class="material-icons">play_circle</span>Play trailer</button>
            <button type="button" class:bookmarked={bookmarkedKeys.includes(bookmarkKey(selectedFilm))} onclick={() => toggleBookmark(selectedFilm)}>
              <span class="material-icons">{bookmarkedKeys.includes(bookmarkKey(selectedFilm)) ? "bookmark" : "bookmark_border"}</span>
              {bookmarkedKeys.includes(bookmarkKey(selectedFilm)) ? "Đã lưu" : "Bookmark"}
            </button>
            <button type="button" onclick={() => void runSearch(selectedFilm?.title || "")}>Tìm phim tương tự</button>
          </div>
        </div>
      </div>

      <div class="film-extra-grid">
        <section class="cast-strip" aria-label="Diễn viên liên quan">
          <div class="related-head">
            <div><span class="eyebrow">Diễn viên</span><h3>Dàn cast</h3></div>
          </div>
          {#if selectedCast.length}
            <div class="cast-list">
              {#each selectedCast as cast}
                <article class="cast-card">
                  <img src={cast.profile || fallbackPoster} alt={cast.name} />
                  <strong>{cast.name}</strong>
                  {#if cast.character}<small>{cast.character}</small>{/if}
                </article>
              {/each}
            </div>
          {:else}
            <div class="download-empty">Chưa có dữ liệu diễn viên.</div>
          {/if}
        </section>

        <section class="related-films" aria-label="Film liên quan">
          <div class="related-head">
            <div><span class="eyebrow">Liên quan</span><h3>Film tương tự</h3></div>
          </div>
          {#if relatedFilms.length}
            <div class="related-film-list">
              {#each relatedFilms as film}
                <button type="button" onclick={() => openMovie({ id: film.id, title: film.title, year: film.year, score: film.score, type: selectedFilm?.type, img: film.poster, overview: film.overview })}>
                  <img src={film.poster || fallbackPoster} alt={film.title} />
                  <div><strong>{film.title}</strong><small>{film.year ? `${film.year}` : ""}{film.score ? `${film.year ? " · " : ""}⭐ ${film.score}` : ""}</small></div>
                </button>
              {/each}
            </div>
          {:else}
            <div class="download-empty">Chưa có film liên quan.</div>
          {/if}
        </section>
      </div>

      <div class="related-downloads">
        <div class="related-head">
          <div><span class="eyebrow">Nội dung liên quan</span><h3>Bản tải khả dụng</h3></div>
        </div>
        {#if loadingLinks}
          <div class="download-empty">Đang quét danh sách bản phim liên quan...</div>
        {:else if selectedLinks.length}
          <div class="download-bulk-panel" class:series={seriesMode}>
            <div class="bulk-actions">
              <strong>{selectedDownloadUrls.length}/{selectedLinks.length} tập đã chọn</strong>
              <button type="button" onclick={selectAllDownloads}>Chọn hết</button>
              <button type="button" onclick={clearSelectedDownloads}>Bỏ chọn</button>
            </div>
            <div class="mode-line">
              <button type="button" class:active={!seriesMode} onclick={() => (seriesMode = false)}>Phim lẻ</button>
              <button type="button" class:active={seriesMode} onclick={() => (seriesMode = true)}>Phim bộ</button>
              <button type="button" class="help-dot" aria-expanded={showSeriesHelp} onclick={() => (showSeriesHelp = !showSeriesHelp)}>!</button>
            </div>
            {#if showSeriesHelp}
              <p class="series-help"><b>Phim bộ</b> sẽ gom các tập vào cùng thư mục: {selectedFilm?.title || "tự nhận diện"}. <b>Phim lẻ</b> tải từng file riêng.</p>
            {/if}
            <button type="button" class="bulk-download-btn" disabled={!selectedDownloadUrls.length || !!addingLinkUrl} onclick={openSelectedDownloadConfirm}>{addingLinkUrl ? "Đang tải..." : `Tải ${selectedDownloadUrls.length} tập đã chọn`}</button>
          </div>
          <div class="download-list">
            {#each selectedLinks as link, index}
              <article class="download-row" class:open={selectedLinkIndex === index} class:added={addedLinkUrls.includes(link.url || "")}>
                <div class="download-toggle-row">
                  <label class="download-check" aria-label="Chọn tập tải">
                    <input type="checkbox" checked={!!link.url && selectedDownloadUrls.includes(link.url)} onchange={() => toggleSelectedDownload(link)} />
                    <span class="select-box"><span class="material-icons">check</span></span>
                  </label>
                  <button class="download-toggle" type="button" onclick={() => selectedLinkIndex = selectedLinkIndex === index ? null : index}>
                    <div>
                      <strong>{displayDownloadTitle(link)}</strong>
                      <small>{episodeLabelFromName(link.original_name || link.name) || "Chưa rõ tập"} · {link.quality || link.resolution || link.source || "Chưa rõ chất lượng"} · {formatSize(link.size)}</small>
                    </div>
                    {#if addedLinkUrls.includes(link.url || "")}<span class="download-status-badge">Đã thêm</span>{/if}
                    <span class="material-icons">{selectedLinkIndex === index ? "expand_less" : "expand_more"}</span>
                  </button>
                </div>
              </article>
              {#if selectedLinkIndex === index}
                <div class="download-detail download-detail-row" role="button" tabindex="0" onclick={() => selectedLinkIndex = null} onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); selectedLinkIndex = null; } }} aria-label="Thu gọn thông tin tập">
                  <p title={link.original_name || link.name}>{link.original_name || link.name || "Bản tải FShare"}</p>
                  <div class="download-tags">
                    <span>Dung lượng: {formatSize(link.size)}</span>
                    <span>Chất lượng: {link.quality || link.resolution || "—"}</span>
                    <span>Nguồn: {link.source || "FShare"}</span>
                  </div>
                  <div class="download-actions">
                    <button type="button" disabled={addingLinkUrl === link.url || addedLinkUrls.includes(link.url || "")} class:added={addedLinkUrls.includes(link.url || "")} onclick={(event) => { event.stopPropagation(); openDownloadConfirm(link); }}>
                      {addingLinkUrl === link.url ? "Đang thêm..." : addedLinkUrls.includes(link.url || "") ? "Đã thêm" : "Thêm riêng tập này"}
                    </button>
                  </div>
                  {#if link.url && linkErrors[link.url]}<p class="download-error">{linkErrors[link.url]}</p>{/if}
                </div>
              {/if}
            {/each}
          </div>
        {:else if hasScannedLinks}
          <div class="download-empty">Chưa tìm thấy bản tải liên quan cho phim này.</div>
        {:else}
          <div class="download-empty">Chưa lấy link. Bấm “Get link” ở phần thông tin phim để tìm bản tải.</div>
        {/if}
      </div>
    </section>
  {/if}

  <section class="discover-layout">
    <div class="main-column">
      <section class="panel content-panel">
        {#if searchResults.length}
          <div class="row-head compact-head">
            <div>
              <span class="eyebrow">Kết quả phim</span>
              <h2>Chọn phim để Get link</h2>
            </div>
            <button type="button" onclick={clearSearch}>Xem phim mới</button>
          </div>
        {/if}
        <div class="poster-grid">
          {#if loading}
            {#each Array(9) as _}<div class="poster-card skeleton"></div>{/each}
          {:else if displayedGrid.length}
            {#each displayedGrid as item, index}
              <button type="button" class="poster-card" onclick={() => openMovie(item)}>
                <img src={optimizePosterUrl(item.img, "w342")} srcset={posterSrcSet(item.img)} sizes="(max-width: 720px) 31vw, 170px" alt={item.title} loading={index < 6 ? "eager" : "lazy"} fetchpriority={index < 5 ? "high" : "auto"} decoding="async" />
                <div class="poster-copy">
                  <strong>{item.title}</strong>
                  <small>{item.year ? `${item.year} · ` : ""}{item.type || "Movie"}{item.score ? ` · ⭐ ${item.score}` : ""}</small>
                  {#if item.size && item.size !== "Metadata" && !/link$/i.test(item.size)}<em>{item.size}</em>{/if}
                </div>
              </button>
            {/each}
          {:else}
            <p class="empty-state">Chưa có dữ liệu Discovery.</p>
          {/if}
        </div>
        {#if displayedGrid.length && canLoadMore}
          <div class="load-more-row">
            <button type="button" onclick={loadMoreFilms} disabled={loadingMore}>{loadingMore ? "Đang tải thêm..." : "Xem thêm phim"}</button>
          </div>
        {/if}
      </section>
    </div>

    <aside class="side-column">
      <section class="panel chips-panel">
        <div class="row-head">
          <div>
            <span class="eyebrow">Lịch sử</span>
            <h2>Tìm gần đây</h2>
          </div>
          <button type="button" onclick={() => { recent = []; message = "Đã xóa tìm kiếm gần đây."; }}>Xóa</button>
        </div>
        <div class="chips">
          {#if recent.length}
            {#each recent as item}
              <button type="button" onclick={() => routeSearchValue(item)}>{item}</button>
            {/each}
          {:else}
            <small>Chưa có tìm kiếm gần đây.</small>
          {/if}
        </div>
      </section>

      <section class="panel top-search">
        <div class="row-head">
          <div>
            <span class="eyebrow">FShare Trending</span>
            <h2>Đang hot</h2>
          </div>
        </div>
        {#if trending.length}
          {#each trending.slice(0, 4) as item, index}
            <button type="button" onclick={() => openMovie(item)}>
              <b>{index + 1}</b>
              <img src={item.img || fallbackPoster} alt={item.title} />
              <div>
                <strong>{item.title}</strong>
                <small>{item.year ? `${item.year} · ` : ""}{item.seed || "FShare"}</small>
              </div>
              <span class="material-icons">chevron_right</span>
            </button>
          {/each}
        {:else}
          <p class="empty-state">Chưa tải được trending FShare.</p>
        {/if}
      </section>

    </aside>
  </section>
</div>


{#if showTrailerModal && selectedTrailerKey}
  <div class="trailer-backdrop" role="presentation" onclick={closeTrailer}>
    <div bind:this={trailerModalRef} class="trailer-modal" role="dialog" aria-modal="true" aria-label="Trailer" onclick={(event) => event.stopPropagation()}>
      <div class="trailer-head">
        <strong>{selectedFilm?.title || "Trailer"}</strong>
        <button type="button" aria-label="Đóng trailer" onclick={closeTrailer}><span class="material-icons">close</span></button>
      </div>
      <iframe
        bind:this={trailerIframeRef}
        title="Trailer"
        src={`https://www.youtube.com/embed/${selectedTrailerKey}?autoplay=1&playsinline=1&enablejsapi=1&rel=0&modestbranding=1&origin=${encodeURIComponent(window.location.origin)}`}
        allow="autoplay; accelerometer; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        allowfullscreen
      ></iframe>
    </div>
  </div>
{/if}

{#if showDownloadConfirm && (confirmDownloadLink || confirmDownloadLinks.length)}
  <div class="confirm-backdrop" role="presentation">
    <div class="confirm-modal compact" role="dialog" aria-modal="true" aria-label="Xác nhận download">
      <div class="confirm-top">
        <span class="material-icons modal-icon">download_for_offline</span>
        <div>
          <h2>Xác nhận download?</h2>
          <p>{confirmDownloadLinks.length ? `${confirmDownloadLinks.length} tập sẽ được tải vào queue NAS.` : "1 bản tải sẽ được tải vào queue NAS."}</p>
        </div>
      </div>
      {#if confirmDownloadLinks.length}
        <div class="confirm-file-list multi" aria-label="Các file sẽ download">
          {#each confirmDownloadLinks.slice(0, 5) as link}
            <strong title={link.original_name || link.name}>{link.original_name || link.name || "Bản tải FShare"}</strong>
          {/each}
          {#if confirmDownloadLinks.length > 5}<small>+{confirmDownloadLinks.length - 5} tập khác</small>{/if}
        </div>
        <div class="confirm-box"><span>Số tập</span><strong>{confirmDownloadLinks.length}</strong></div>
        <div class="confirm-box"><span>Chế độ</span><strong>{seriesMode ? "Phim bộ" : "Phim lẻ"}</strong></div>
      {:else if confirmDownloadLink}
        <div class="confirm-file-list" aria-label="File sẽ download">
          <strong title={confirmDownloadLink.original_name || confirmDownloadLink.name}>{confirmDownloadLink.original_name || confirmDownloadLink.name || "Bản tải FShare"}</strong>
        </div>
        <div class="confirm-box"><span>Dung lượng</span><strong>{formatSize(confirmDownloadLink.size)}</strong></div>
        <div class="confirm-box"><span>Chất lượng</span><strong>{confirmDownloadLink.quality || confirmDownloadLink.resolution || confirmDownloadLink.source || "—"}</strong></div>
      {/if}
      <div class="modal-actions">
        <button type="button" onclick={closeDownloadConfirm}>Hủy</button>
        <button type="button" class="danger-confirm" onclick={() => void confirmRelatedDownload()} disabled={!!addingLinkUrl}>{addingLinkUrl ? "Đang gửi vào NAS..." : "Download"}</button>
      </div>
      {#if confirmDownloadLinks.length}
        {#each confirmDownloadLinks.filter((link) => link.url && linkErrors[link.url]) as link}
          <p class="download-error">{linkErrors[link.url || ""]}</p>
        {/each}
      {:else if confirmDownloadLink?.url && linkErrors[confirmDownloadLink.url]}<p class="download-error">{linkErrors[confirmDownloadLink.url]}</p>{/if}
    </div>
  </div>
{/if}

<style>
  .discover-screen { display: grid; gap: 1rem; width: 100%; max-width: 1440px; min-width: 0; margin: 0 auto; overflow-x: hidden; }
  .discover-hero { position: relative; display: block; height: 500px; padding: 28px; overflow: hidden; border: 1px solid rgba(129, 140, 248, 0.34); border-radius: 28px; background: radial-gradient(circle at 24% 52%, rgba(139, 92, 246, 0.18), transparent 35%), radial-gradient(circle at 78% 24%, rgba(59, 130, 246, 0.12), transparent 32%), linear-gradient(110deg, #101426 0%, #070b16 55%, #050811 100%); box-shadow: 0 34px 110px rgba(0,0,0,.5), inset 0 1px 0 rgba(255,255,255,.055); }
  .poster-only-hero { grid-template-columns: 1fr; }
  .featured-hero { grid-template-columns: 1fr; }
  .hero-wide-media { position: absolute; inset: 28px; min-width: 0; overflow: visible; border: 0; background: transparent; }
  .hero-wide-media::before { content: ""; position: absolute; left: 0; top: 0; width: min(76%, 900px); height: 100%; border-radius: 22px; background: rgba(139, 92, 246, .18); filter: blur(34px); opacity: .72; pointer-events: none; }
  .hero-wide-media::after { content: ""; position: absolute; inset: -28px; z-index: 1; background: linear-gradient(90deg, rgba(5,8,17,0) 0%, rgba(5,8,17,0) 42%, rgba(5,8,17,.22) 48%, rgba(5,8,17,.58) 58%, rgba(5,8,17,.90) 72%, rgba(5,8,17,.99) 100%); pointer-events: none; }
  .hero-wide-media img { position: absolute; left: 0; top: 0; width: min(76%, 900px); height: 100%; object-fit: cover; object-position: center; border: 0; border-radius: 22px 0 0 22px; opacity: 0; transform: scale(1.004); image-rendering: auto; filter: saturate(1.04) brightness(.98); -webkit-mask-image: linear-gradient(90deg, #000 0%, #000 44%, rgba(0,0,0,.92) 50%, rgba(0,0,0,.58) 60%, rgba(0,0,0,.20) 72%, transparent 86%); mask-image: linear-gradient(90deg, #000 0%, #000 44%, rgba(0,0,0,.92) 50%, rgba(0,0,0,.58) 60%, rgba(0,0,0,.20) 72%, transparent 86%); box-shadow: 0 28px 70px rgba(0,0,0,.58), 0 0 46px rgba(129,140,248,.16); transition: opacity .55s ease, transform 3.6s ease; }
  .hero-wide-media img.active { opacity: 1; transform: scale(1); }
  .hero-info-card { position: absolute; left: clamp(40%, 42vw, 45%); right: clamp(34px, 6vw, 86px); top: 50%; z-index: 3; display: flex; min-width: 0; max-width: 620px; flex-direction: column; align-items: flex-start; justify-content: center; text-align: left; transform: translateY(-50%); }
  .hero-info-card .eyebrow { margin-bottom: 14px; font-size: 13px; font-weight: 900; letter-spacing: .18em; color: #c4a7ff; text-transform: uppercase; }
  .hero-info-card h1 { margin: 0; color: #fff; font-size: clamp(54px, 5vw, 72px); font-weight: 950; line-height: .96; letter-spacing: -.06em; text-shadow: 0 18px 46px rgba(0,0,0,.58); }
  .hero-info-card p { display: -webkit-box; max-width: 580px; margin-top: 24px; overflow: hidden; color: rgba(255,255,255,.7); font-size: 17px; line-height: 1.62; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
  .hero-meta { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 22px; }
  .hero-meta span { padding: 7px 14px; border: 1px solid rgba(255,255,255,.14); border-radius: 999px; color: #e8eaf2; background: rgba(255,255,255,.075); font-size: 14px; font-weight: 800; backdrop-filter: blur(12px); box-shadow: inset 0 1px 0 rgba(255,255,255,.045); }
  .hero-open-action { align-self: flex-start; width: fit-content; margin-top: 28px; padding: 15px 26px; border-radius: 16px; color: #101425; background: linear-gradient(135deg, #ffb21f 0%, #c58cff 100%); font-size: 16px; font-weight: 900; box-shadow: 0 18px 45px rgba(177,117,255,.28), 0 12px 34px rgba(255,178,31,.14); }
  .hero-dots { position: absolute; right: clamp(1rem, 2vw, 1.5rem); bottom: clamp(1rem, 2vw, 1.5rem); z-index: 3; display: flex; gap: 5px; padding: 5px 7px; border-radius: 999px; background: rgba(3,6,14,.42); backdrop-filter: blur(10px); }
  .hero-dots span { width: 7px; height: 7px; border-radius: 999px; background: rgba(255,255,255,.38); }
  .hero-dots span.active { width: 18px; background: #f8c14a; }
  .hero-info-card { position: absolute; left: clamp(49%, 51vw, 53%); right: clamp(2rem, 4vw, 4.8rem); top: 50%; z-index: 2; display: flex; min-width: 0; max-width: 660px; flex-direction: column; align-items: flex-start; text-align: left; transform: translateY(-50%); }
  .hero-info-card p { display: -webkit-box; max-width: 580px; margin-top: 1.05rem; overflow: hidden; color: rgba(203,213,225,.8); font-size: clamp(.92rem, 1.08vw, 1.02rem); line-height: 1.62; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
  .hero-meta { display: flex; flex-wrap: wrap; gap: .5rem; margin-top: 1rem; }
  .hero-meta span { padding: .42rem .72rem; border: 1px solid rgba(255,255,255,.13); border-radius: 999px; color: rgba(255,255,255,.88); background: rgba(15,23,42,.62); font-size: .78rem; font-weight: 850; backdrop-filter: blur(12px); box-shadow: inset 0 1px 0 rgba(255,255,255,.05); }
  .hero-open-action { align-self: flex-start; margin-top: 1.45rem; padding: .84rem 1.14rem; border-radius: 16px; color: #111827; background: linear-gradient(135deg, #f8c14a 0%, #f59e0b 38%, #a78bfa 100%); font-weight: 950; box-shadow: 0 16px 36px rgba(167,139,250,.26), 0 8px 22px rgba(248,193,74,.16); }
  .search-box { display: grid; grid-template-columns: 24px minmax(0, 1fr) 46px; width: 100%; min-width: 0; align-items: center; gap: 0.65rem; max-width: 760px; min-height: 68px; margin-top: 1.15rem; padding: 0 0.65rem 0 1rem; border: 1px solid rgba(148, 163, 184, 0.18); border-radius: 20px; background: rgba(3, 6, 14, 0.58); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05); }
  .search-box input { min-width: 0; border: 0; color: #fff; background: transparent; outline: 0; font-size: 1.02rem; font-weight: 850; }
  .search-box button { width: 46px; height: 46px; border: 0; border-radius: 15px; color: #080a12; background: linear-gradient(135deg, #f8c14a, #a78bfa); }

  .panel { border: 1px solid rgba(148, 163, 184, 0.15); background: linear-gradient(180deg, rgba(20, 26, 42, 0.92), rgba(9, 12, 21, 0.86)); box-shadow: 0 18px 48px rgba(0, 0, 0, 0.25); }
  .panel p, .chips small { color: rgba(226, 232, 240, 0.62); }
  .discovery-controls { display: grid; gap: .72rem; min-width: 0; }
  .type-switch { position: relative; display: grid !important; grid-template-columns: repeat(3, minmax(0, 1fr)) !important; grid-auto-flow: column; gap: .45rem; padding: .48rem; border: 1px solid rgba(255,255,255,.12); border-radius: 30px; background: radial-gradient(circle at 20% 0%, rgba(167,139,250,.28), transparent 34%), linear-gradient(180deg, rgba(255,255,255,.1), rgba(255,255,255,.045)); box-shadow: inset 0 1px 0 rgba(255,255,255,.08), 0 18px 52px rgba(0,0,0,.28); }
  .type-switch button { min-width: 0; width: 100%; min-height: 74px; display: grid; grid-template-columns: minmax(0,1fr) 44px; align-items: center; gap: .55rem; padding: 0 1.1rem; border: 1px solid rgba(255,255,255,.1); border-radius: 24px; color: rgba(255,255,255,.82); background: rgba(255,255,255,.08); box-shadow: inset 0 1px 0 rgba(255,255,255,.08); }
  .type-switch button.active { color: #111827; border-color: rgba(255,210,180,.55); background: linear-gradient(135deg,#ffd29a,#fb7185 72%,#a78bfa); box-shadow: 0 12px 34px rgba(251,113,133,.26), inset 0 1px 0 rgba(255,255,255,.32); }
  .type-switch strong { font-size: clamp(1.35rem, 3.8vw, 2.05rem); letter-spacing: .02em; }
  .type-switch .material-icons { width: 44px; height: 44px; display: grid; place-items: center; border-radius: 15px; color: inherit; background: rgba(255,255,255,.16); font-size: 1.5rem; }
  .filter-card { position: relative; display: grid; min-width: 0; padding: .72rem .9rem; overflow: hidden; border: 1px solid rgba(255,255,255,.10); border-radius: 22px; background: radial-gradient(circle at 0 0, rgba(167,139,250,.16), transparent 32%), linear-gradient(135deg, rgba(46,35,77,.74), rgba(25,27,42,.78)); box-shadow: inset 0 1px 0 rgba(255,255,255,.05), 0 14px 42px rgba(0,0,0,.22); }
  .filter-section { display: grid; min-width: 0; }
  .filter-label { display: none; }
  .filter-options { display: flex; gap: .55rem; overflow-x: auto; scrollbar-width: none; }
  .filter-options::-webkit-scrollbar { display: none; }
  .filter-options button { flex: 0 0 auto; min-height: 44px; padding: 0 1.05rem; border: 1px solid rgba(255,255,255,.11); border-radius: 999px; color: #fff; background: linear-gradient(180deg, rgba(255,255,255,.105), rgba(255,255,255,.052)); box-shadow: inset 0 1px 0 rgba(255,255,255,.07); font-size: .88rem; font-weight: 950; }
  .filter-options button.active { color: #111827; border-color: rgba(255,210,180,.55); background: linear-gradient(135deg,#ffd29a,#fb7185); box-shadow: 0 8px 20px rgba(251,113,133,.18); }
  .hot-options button { min-width: 132px; }
  .film-info-panel { display: grid; gap: 1rem; padding: 1rem; border: 1px solid rgba(248,193,74,.18); border-radius: 24px; background: radial-gradient(circle at 0 0, rgba(248,193,74,.14), transparent 32%), linear-gradient(180deg, rgba(20,26,42,.94), rgba(9,12,21,.88)); box-shadow: 0 20px 58px rgba(0,0,0,.28); }
  .film-main-info { display: grid; grid-template-columns: 1fr; gap: 1rem; align-items: stretch; }
  .film-hero-carousel { position: relative; width: 100%; height: clamp(320px, 48vw, 560px); overflow: hidden; border-radius: 22px; background: #111827; box-shadow: inset 0 1px 0 rgba(255,255,255,.08); }
  .film-hero-carousel img { width: 100%; height: 100%; display: block; object-fit: cover; object-position: center 18%; }
  .image-dots { position: absolute; left: 50%; bottom: .72rem; transform: translateX(-50%); display: flex; align-items: center; justify-content: center; gap: .32rem; padding: .28rem .42rem; border-radius: 999px; background: rgba(2,6,23,.48); backdrop-filter: blur(10px); }
  .image-dots span { width: 6px; height: 6px; border-radius: 999px; background: rgba(255,255,255,.45); transition: width .2s ease, background .2s ease; }
  .image-dots span.active { width: 16px; background: #f8c14a; }
  .film-info-copy { min-width: 0; display: grid; gap: .75rem; align-content: start; }
  .film-info-head { display: flex; align-items: flex-start; justify-content: space-between; gap: .75rem; }
  .film-info-head h2 { margin-top: .2rem; font-size: clamp(1.55rem, 3vw, 2.35rem); }
  .film-info-head button { width: 40px; height: 40px; flex: 0 0 auto; display: grid; place-items: center; border: 1px solid rgba(255,255,255,.12); border-radius: 14px; color: #fff; background: rgba(255,255,255,.07); }
  .film-meta { display: flex; flex-wrap: wrap; gap: .45rem; }
  .film-meta span { padding: .34rem .62rem; border: 1px solid rgba(255,255,255,.1); border-radius: 999px; color: rgba(255,255,255,.78); background: rgba(255,255,255,.055); font-size: .82rem; font-weight: 850; }
  .film-info-copy p { max-width: 760px; color: rgba(226,232,240,.68); line-height: 1.5; }
  .overview-block { position: relative; display: grid; gap: .18rem; align-items: start; }
  .overview-block p { display: -webkit-box; -webkit-line-clamp: 4; -webkit-box-orient: vertical; overflow: hidden; }
  .overview-block.expanded p { display: block; overflow: visible; }
  .overview-toggle { width: fit-content; min-height: 0; padding: 0; border: 0; border-radius: 0; color: rgba(226,232,240,.82); background: transparent; font: inherit; font-size: .86rem; font-weight: 750; line-height: 1.5; text-decoration: none; }
  .overview-toggle:hover { color: #fff; }
  .overview-block:not(.expanded) .overview-toggle { position: absolute; right: 0; bottom: 0; padding-left: 2.2rem; background: linear-gradient(90deg, rgba(14,18,30,0), rgba(14,18,30,.96) 34%, rgba(14,18,30,.96)); }
  .film-actions { display: flex; flex-wrap: wrap; gap: .6rem; }
  .film-actions button, .film-actions a { min-height: 42px; display: inline-flex; align-items: center; justify-content: center; gap: .36rem; padding: 0 .9rem; border: 1px solid rgba(255,255,255,.13); border-radius: 14px; color: #fff; background: rgba(255,255,255,.07); font-weight: 900; text-decoration: none; }
  .film-actions button.bookmarked { color: #111827; border-color: rgba(248,193,74,.52); background: linear-gradient(135deg,#f8c14a,#fb7185); }
  .film-actions .primary-action { color: #111827; border-color: transparent; background: linear-gradient(135deg,#f8c14a,#fb7185); }
  .film-actions .trailer-action { border-color: rgba(248,193,74,.28); color: #f8fafc; background: linear-gradient(135deg, rgba(248,193,74,.16), rgba(167,139,250,.16)); }
  .film-actions .trailer-action:disabled { opacity: .45; cursor: not-allowed; }

  .trailer-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2147483647;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(2, 6, 23, 0.86);
    backdrop-filter: blur(16px) saturate(130%);
  }
  .trailer-modal {
    width: min(1080px, 96vw);
    max-height: 92vh;
    overflow: hidden;
    border: 1px solid rgba(248, 193, 74, 0.28);
    border-radius: 24px;
    background: #020617;
    box-shadow: 0 32px 100px rgba(0,0,0,.72);
  }
  .trailer-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: .75rem;
    padding: .75rem .85rem;
    color: #fff;
    background: linear-gradient(135deg, rgba(248,193,74,.13), rgba(124,58,237,.13));
  }
  .trailer-head strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trailer-head button {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border: 1px solid rgba(255,255,255,.14);
    border-radius: 12px;
    color: #fff;
    background: rgba(255,255,255,.08);
  }
  .trailer-modal iframe {
    display: block;
    width: 100%;
    height: min(60.75vw, 608px);
    min-height: 220px;
    border: 0;
    background: #000;
  }
  @media (max-width: 720px) {
    :global(body.fhub-trailer-open) { overflow: hidden; }
    :global(body.fhub-trailer-open .fhub-bottom-nav) { display: none !important; }
    :global(body.fhub-trailer-open .fhub-topbar) { display: none !important; }
    .trailer-backdrop { padding: 0; align-items: stretch; background: #000; inset: 0; width: 100vw; height: 100dvh; }
    .trailer-modal { position: relative; width: 100vw; height: 100dvh; max-height: 100dvh; border: 0; border-radius: 0; display: block; background: #000; }
    .trailer-modal:fullscreen { width: 100vw; height: 100vh; max-height: 100vh; border-radius: 0; }
    .trailer-head { position: absolute; top: max(.55rem, env(safe-area-inset-top)); left: .55rem; right: .55rem; z-index: 2; padding: 0; background: transparent; pointer-events: none; }
    .trailer-head strong { display: none; }
    .trailer-head button { margin-left: auto; width: 44px; height: 44px; border-radius: 999px; background: rgba(2,6,23,.58); backdrop-filter: blur(12px); pointer-events: auto; }
    .trailer-modal iframe { width: 100%; height: 100dvh; min-height: 100dvh; }
  }

  .film-extra-grid { display: grid; grid-template-columns: minmax(0,1.1fr) minmax(280px,.9fr); gap: .85rem; }
  .cast-strip, .related-films { min-width: 0; display: grid; gap: .75rem; padding: .85rem; border: 1px solid rgba(255,255,255,.09); border-radius: 20px; background: rgba(255,255,255,.045); }
  .cast-list { display: flex; gap: .85rem; overflow-x: auto; padding-bottom: .15rem; scrollbar-width: none; }
  .cast-list::-webkit-scrollbar, .related-film-list::-webkit-scrollbar { display: none; }
  .cast-card { flex: 0 0 96px; display: grid; justify-items: center; gap: .35rem; text-align: center; }
  .cast-card img { width: 82px; height: 82px; object-fit: cover; border-radius: 999px; border: 2px solid rgba(255,255,255,.14); background: #111827; box-shadow: 0 10px 26px rgba(0,0,0,.24); }
  .cast-card strong { width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #fff; font-size: .78rem; }
  .cast-card small { width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgba(226,232,240,.55); font-size: .68rem; }
  .related-film-list { display: flex; gap: .7rem; overflow-x: auto; padding-bottom: .15rem; scrollbar-width: none; }
  .related-film-list button { flex: 0 0 148px; min-width: 0; padding: 0; overflow: hidden; border: 1px solid rgba(255,255,255,.09); border-radius: 16px; color: #fff; background: rgba(5,8,16,.42); text-align: left; }
  .related-film-list img { width: 100%; height: 172px; object-fit: cover; background: #111827; }
  .related-film-list div { display: grid; gap: .18rem; padding: .58rem; }
  .related-film-list strong, .related-film-list small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .related-film-list strong { font-size: .82rem; }
  .related-film-list small { color: rgba(226,232,240,.58); font-size: .72rem; }
  .related-downloads { display: grid; grid-template-rows: auto minmax(0, 1fr); gap: .75rem; min-width: 0; max-height: 520px; overflow: hidden; padding: .85rem; border: 1px solid rgba(255,255,255,.09); border-radius: 20px; background: rgba(255,255,255,.045); }
  .related-head { display: flex; align-items: center; justify-content: space-between; gap: .7rem; }
  .related-head h3 { margin-top: .15rem; font-size: 1.15rem; }
  .related-head > span { padding: .32rem .58rem; border-radius: 999px; color: #111827; background: #f8c14a; font-size: .78rem; font-weight: 950; white-space: nowrap; }
  .download-list { min-height: 0; display: grid; align-content: start; gap: .5rem; max-height: none; overflow-y: auto; overflow-x: hidden; padding: 0 .28rem .2rem 0; overscroll-behavior: contain; scrollbar-width: thin; }
  .download-list::-webkit-scrollbar { width: 8px; }
  .download-list::-webkit-scrollbar-thumb { border-radius: 999px; background: rgba(248,193,74,.38); }
  .download-list::-webkit-scrollbar-track { background: rgba(255,255,255,.04); border-radius: 999px; }
  .download-row { min-width: 0; min-height: 54px; overflow: hidden; border: 1px solid rgba(255,255,255,.09); border-radius: 14px; background: rgba(5,8,16,.42); }
  .download-toggle { box-sizing: border-box !important; width: 100% !important; min-width: 0 !important; min-height: 54px !important; height: auto !important; display: grid !important; grid-template-columns: 32px minmax(0,1fr) auto 24px !important; align-items: center !important; gap: .6rem !important; padding: .55rem .62rem !important; border: 0 !important; color: #fff !important; background: transparent !important; text-align: left !important; appearance: none !important; }
  .download-toggle > div { min-width: 0; display: block; }
  .download-toggle .material-icons:first-child { width: 32px; height: 32px; display: grid; place-items: center; border-radius: 11px; color: #111827; background: linear-gradient(135deg,#f8c14a,#fb7185); font-size: 1rem; }
  .download-toggle strong { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #fff; font-size: .86rem; line-height: 1.15; }
  .download-toggle small { display: block; min-width: 0; margin-top: .12rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgba(226,232,240,.62); font-size: .7rem; line-height: 1.15; }
  .download-status-badge { justify-self: end; padding: .22rem .42rem; border-radius: 999px; color: #052e16; background: linear-gradient(135deg,#bbf7d0,#22c55e); font-size: .66rem; font-weight: 950; white-space: nowrap; }
  .download-row.open { min-height: 0; border-color: rgba(248,193,74,.34); background: rgba(248,193,74,.055); }
  .download-row.added { border-color: rgba(34,197,94,.42); background: rgba(34,197,94,.075); }
  .download-row.added .download-toggle .material-icons:first-child { background: linear-gradient(135deg,#bbf7d0,#22c55e); }
  .download-detail { display: grid; gap: .5rem; margin: .4rem 0 .08rem 0; padding: .64rem .72rem; border: 1px solid rgba(248,193,74,.16); border-radius: 12px; color: rgba(226,232,240,.78); background: rgba(2,6,23,.62); box-shadow: inset 0 1px 0 rgba(255,255,255,.04); cursor: pointer; }
  .download-detail-row { scroll-margin: .5rem; }
  .download-detail p { margin: 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgba(248,250,252,.86); font-size: .78rem; }
  .download-tags { display: flex; flex-wrap: wrap; gap: .34rem; }
  .download-tags span { padding: .24rem .44rem; border: 1px solid rgba(255,255,255,.1); border-radius: 999px; background: rgba(255,255,255,.055); font-size: .7rem; font-weight: 850; }
  .download-actions { display: flex; flex-wrap: wrap; gap: .45rem; }
  .download-actions a, .download-actions button { min-height: 32px; display: inline-flex; align-items: center; justify-content: center; padding: 0 .62rem; border: 1px solid rgba(255,255,255,.12); border-radius: 11px; color: #fff; background: rgba(255,255,255,.07); font-size: .76rem; font-weight: 900; text-decoration: none; }
  .download-actions button { color: #111827; border-color: transparent; background: linear-gradient(135deg,#f8c14a,#fb7185); }
  .download-actions button.added { color: #052e16; background: linear-gradient(135deg,#86efac,#22c55e); }
  .download-actions button:disabled { cursor: not-allowed; opacity: .78; }
  .download-error { color: #fecaca !important; font-size: .78rem; }
  .download-empty { padding: .9rem; border: 1px dashed rgba(255,255,255,.14); border-radius: 16px; color: rgba(226,232,240,.64); background: rgba(255,255,255,.035); font-size: .88rem; }
  .confirm-backdrop{pointer-events:auto;position:fixed;inset:0;z-index:9999;display:grid;place-items:center;padding:1rem;background:rgba(2,6,23,.68);backdrop-filter:blur(10px);overflow:hidden}
  .confirm-modal{pointer-events:auto;isolation:isolate;width:min(430px,calc(100vw - 2rem));max-width:430px;min-width:0;box-sizing:border-box;display:grid;gap:.85rem;padding:1.15rem;overflow:hidden;border:1px solid rgba(248,193,74,.22);border-radius:22px;background:linear-gradient(180deg,rgba(20,26,42,.98),rgba(8,12,22,.96));box-shadow:0 28px 80px rgba(0,0,0,.45)}
  .confirm-modal *{box-sizing:border-box;min-width:0}
  .modal-icon{width:42px;height:42px;display:grid;place-items:center;border-radius:14px;color:#080a12;background:linear-gradient(135deg,#f8c14a,#a78bfa);font-size:1.35rem}
  .confirm-top{display:grid;grid-template-columns:42px minmax(0,1fr);align-items:center;gap:.7rem}.confirm-top h2{margin:0;font-size:1.08rem}.confirm-top p{margin:.1rem 0 0;font-size:.82rem;color:#aab4c3}
  .confirm-file-list,.confirm-box{width:100%;max-width:100%;padding:.58rem .65rem;overflow:hidden;border-radius:13px;background:rgba(255,255,255,.055);border:1px solid rgba(255,255,255,.08)}
  .confirm-file-list strong{display:block;width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#f8fafc;font-size:.9rem}.confirm-box{display:grid;grid-template-columns:minmax(0,88px) minmax(0,1fr);align-items:center;gap:.65rem}.confirm-box span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#aab4c3;font-size:.82rem}.confirm-box strong{justify-self:end;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#f8c14a;font-size:.95rem}
  .modal-actions{width:100%;display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:.55rem}.modal-actions button{width:100%;min-width:0;min-height:42px;border-radius:13px}.danger-confirm{color:#080a12;border:0;background:linear-gradient(135deg,#f8c14a,#a78bfa)}

  @media (max-width: 720px) {
    .confirm-backdrop {
      align-items: center;
      justify-items: center;
      padding: .75rem;
      overflow-x: hidden;
    }
    .confirm-modal {
      width: min(100%, calc(100vw - 1.5rem));
      max-width: none;
      max-height: calc(100dvh - 1.5rem);
      overflow: auto;
      box-sizing: border-box;
      gap: .68rem;
      padding: .85rem;
      border-radius: 20px;
    }
    .confirm-top {
      grid-template-columns: 38px minmax(0, 1fr);
      gap: .62rem;
      min-width: 0;
    }
    .modal-icon {
      width: 38px;
      height: 38px;
      border-radius: 13px;
      font-size: 1.18rem;
    }
    .confirm-top h2 {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 1rem;
    }
    .confirm-top p {
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
      font-size: .76rem;
      line-height: 1.35;
    }
    .confirm-file-list,
    .confirm-box {
      min-width: 0;
      padding: .55rem .58rem;
      border-radius: 12px;
    }
    .confirm-file-list strong {
      max-width: 100%;
      font-size: .82rem;
    }
    .confirm-box {
      grid-template-columns: minmax(0, .82fr) minmax(0, 1fr);
      gap: .45rem;
    }
    .confirm-box span,
    .confirm-box strong {
      min-width: 0;
      font-size: .8rem;
    }
    .modal-actions {
      grid-template-columns: 1fr;
      gap: .45rem;
    }
    .modal-actions button {
      min-height: 42px;
    }
  }

  .discover-layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(280px, 340px); min-width: 0; gap: 1rem; align-items: start; }
  .main-column, .side-column { display: grid; gap: 1rem; min-width: 0; }
  .panel { border-radius: 22px; padding: 1rem; }
  .row-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 0.9rem; }
  .row-head a, .row-head button { color: #f8c14a; border: 0; background: transparent; font-weight: 900; text-decoration: none; }
  .poster-grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); min-width: 0; gap: 0.85rem; }
  .poster-card { position: relative; min-height: 272px; overflow: hidden; border: 1px solid rgba(148, 163, 184, 0.14); border-radius: 18px; background: #111827; text-align: left; transition: transform 0.18s ease, border-color 0.18s ease; }
  .poster-card:hover { transform: translateY(-3px); border-color: rgba(248, 193, 74, 0.34); }
  .poster-card img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .poster-card::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(0,0,0,.05), rgba(0,0,0,.9)); }
  .poster-copy { position: absolute; z-index: 1; left: 0.8rem; right: 0.8rem; bottom: 0.8rem; display: grid; gap: 0.28rem; min-width: 0; }
  .poster-copy strong, .poster-copy small, .poster-copy em { display: block; min-width: 0; overflow: visible; text-overflow: clip; white-space: normal; overflow-wrap: anywhere; }
  .poster-copy strong { color: #fff; font-size: 1rem; line-height: 1.12; }
  .poster-copy small { color: rgba(226, 232, 240, 0.76); font-style: normal; line-height: 1.18; }
  .poster-copy em { color: #f8c14a; font-size: 0.78rem; line-height: 1.18; font-style: normal; font-weight: 900; }
  .load-more-row { display: flex; justify-content: center; padding: 1rem 0 .2rem; }
  .load-more-row button { min-height: 44px; padding: 0 1.25rem; border: 1px solid rgba(248,193,74,.28); border-radius: 999px; color: #111827; background: linear-gradient(135deg,#f8c14a,#a78bfa); font-weight: 950; box-shadow: 0 14px 34px rgba(0,0,0,.24); }
  .load-more-row button:disabled { opacity: .68; cursor: not-allowed; }
  .skeleton { min-height: 272px; background: linear-gradient(90deg, rgba(255,255,255,.05), rgba(255,255,255,.12), rgba(255,255,255,.05)); background-size: 220% 100%; animation: pulse 1.2s infinite linear; }
  @keyframes pulse { to { background-position: -220% 0; } }
  .empty-state { padding: 1rem; border-radius: 16px; background: rgba(255,255,255,.04); }
  .chips { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .chips button { min-height: 34px; padding: 0 0.75rem; border: 1px solid rgba(167, 139, 250, 0.18); border-radius: 999px; color: #e9d5ff; background: rgba(124, 58, 237, 0.14); }
  .top-search { display: grid; gap: 0.6rem; }
  .top-search .row-head { margin-bottom: 0.2rem; }
  .top-search > button { display: grid; grid-template-columns: 26px 52px minmax(0, 1fr) 22px; align-items: center; gap: 0.6rem; min-height: 68px; padding: 0.5rem; border: 1px solid rgba(148, 163, 184, 0.12); border-radius: 16px; color: inherit; background: rgba(255, 255, 255, 0.045); text-align: left; }
  .top-search b { color: #f8c14a; }
  .top-search img { width: 52px; height: 52px; object-fit: cover; border-radius: 12px; }
  .top-search strong, .top-search small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .top-search strong { color: #fff; }
  .top-search small { color: rgba(226, 232, 240, 0.62); }
  .queue-meter { display: flex; align-items: end; gap: 0.55rem; margin: 0.8rem 0; }
  .queue-meter strong { color: #fff; font-size: 3.1rem; line-height: 0.9; letter-spacing: -0.06em; }
  .queue-meter span { color: rgba(226, 232, 240, 0.65); font-weight: 850; }
  @media (max-width: 1360px) and (min-width: 721px) { .discover-layout { grid-template-columns: minmax(0, 1fr) 220px; gap: .72rem; } .poster-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .68rem; } .poster-card { min-height: clamp(250px, 25vw, 320px); border-radius: 16px; } .poster-copy { left: .68rem; right: .68rem; bottom: .68rem; } .poster-copy strong { font-size: .88rem; } .poster-copy small, .poster-copy em { font-size: .68rem; } .side-column { gap: .65rem; } .side-column .panel { padding: .65rem; border-radius: 18px; } .side-column .row-head { align-items: center; gap: .35rem; margin-bottom: .45rem; } .side-column .eyebrow { display: none; } .side-column h2 { font-size: 1rem; letter-spacing: -.03em; } .chips { gap: .35rem; max-height: 78px; overflow: hidden; } .chips button { min-height: 28px; max-width: 100%; padding: 0 .52rem; font-size: .68rem; } .top-search { gap: .42rem; } .top-search > button { grid-template-columns: 20px 42px minmax(0, 1fr); min-height: 52px; gap: .42rem; padding: .38rem; border-radius: 13px; } .top-search > button > .material-icons { display: none; } .top-search img { width: 42px; height: 42px; border-radius: 10px; } .top-search b { font-size: .75rem; } .top-search strong { font-size: .78rem; } .top-search small { font-size: .64rem; } }
  @media (max-width: 1120px) { .discover-hero { height: clamp(420px, 48vw, 500px); } .hero-wide-media img, .hero-wide-media::before { width: 78%; } .hero-info-card { left: 42%; right: 2rem; max-width: 560px; } .discover-layout { grid-template-columns: 1fr; } .side-column { display: none; } .poster-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
  @media (max-width: 720px) { .discover-screen { gap: 0.5rem; max-width: 100%; } .discover-hero { height: clamp(390px, 112vw, 470px); padding: .48rem; border-radius: 18px; box-shadow: none; } .hero-wide-media { inset: .48rem; overflow: hidden; border-radius: 16px; background: #050816; } .hero-wide-media::before { display: none; } .hero-wide-media::after { display: block; inset: 0; border-radius: 16px; background: linear-gradient(180deg, rgba(5,8,17,0) 0%, rgba(5,8,17,.04) 46%, rgba(5,8,17,.54) 73%, rgba(5,8,17,.96) 100%); } .hero-wide-media img { width: 100%; height: 100%; border-radius: 16px; object-fit: cover; object-position: center; filter: none; -webkit-mask-image: none; mask-image: none; box-shadow: none; } .hero-info-card { left: .78rem; right: .78rem; top: auto; bottom: .78rem; z-index: 3; max-width: none; display: grid; grid-template-columns: 1fr auto; grid-template-areas: "meta action"; align-items: end; gap: .58rem; transform: none; } .hero-info-card p { display: none; } .hero-dots { display: none; } .hero-open-action { grid-area: action; align-self: end; margin-top: 0; padding: .74rem .86rem; border-radius: 14px; font-size: .78rem; white-space: nowrap; box-shadow: 0 12px 28px rgba(0,0,0,.34), 0 10px 24px rgba(248,193,74,.16); } .hero-meta { grid-area: meta; max-width: none; gap: .34rem; margin-top: 0; align-self: end; } .hero-meta span { padding: .34rem .52rem; font-size: .68rem; background: rgba(8,12,24,.72); backdrop-filter: blur(10px); } .hero-copy { display: block; } .eyebrow, h1, .hero-copy p { display: none; } .search-box { min-height: 46px; grid-template-columns: 18px minmax(0, 1fr) 34px; gap: .45rem; margin-top: 0; padding: 0 .42rem 0 .72rem; border-radius: 14px; } .search-box input { font-size: .9rem; } .search-box button { width: 34px; height: 34px; border-radius: 11px; } .search-box .material-icons { font-size: 1.25rem; } .discovery-controls { gap: .45rem; } .type-switch { grid-template-columns: repeat(3, minmax(0, 1fr)); padding: .28rem; border-radius: 18px; } .type-switch button { min-height: 42px; grid-template-columns: minmax(0,1fr) 28px; padding: 0 .6rem; border-radius: 14px; } .type-switch strong { font-size: .98rem; } .type-switch .material-icons { width: 28px; height: 28px; border-radius: 9px; font-size: 1rem; } .filter-card { padding: .45rem; border-radius: 14px; } .filter-section { gap: 0; } .filter-label { display: none; } .filter-options { gap: .34rem; } .filter-options button { min-height: 34px; padding: 0 .64rem; font-size: .76rem; } .film-info-panel { gap: .72rem; padding: .75rem; border-radius: 18px; } .film-main-info { grid-template-columns: 1fr; gap: .72rem; } .film-hero-carousel { height: 280px; min-height: 0; border-radius: 16px; } .film-extra-grid { grid-template-columns: 1fr; gap: .65rem; } .cast-strip, .related-films, .related-downloads { padding: .65rem; border-radius: 16px; } .cast-card { flex-basis: 78px; } .cast-card img { width: 64px; height: 64px; } .related-film-list button { flex-basis: 116px; } .related-film-list img { height: 134px; } .download-toggle { grid-template-columns: 30px minmax(0,1fr) auto 24px !important; padding: .6rem !important; gap: .5rem !important; } .download-status-badge { font-size: .6rem; padding: .18rem .34rem; } .download-detail { padding: .56rem; } .film-info-head h2 { font-size: 1.25rem; } .film-info-head button { width: 34px; height: 34px; border-radius: 12px; } .film-info-copy p { font-size: .84rem; line-height: 1.45; } .overview-block p { -webkit-line-clamp: 4; } .overview-toggle { font-size: .8rem; } .overview-block:not(.expanded) .overview-toggle { padding-left: 1.8rem; } .film-meta span { font-size: .72rem; padding: .28rem .48rem; } .film-actions button, .film-actions a { min-height: 36px; border-radius: 12px; font-size: .82rem; } .panel { padding: 0.62rem; border-radius: 16px; } .row-head { margin-bottom: .55rem; } .poster-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.45rem; } .poster-card { min-height: 184px; border-radius: 12px; } .poster-copy { left: .5rem; right: .5rem; bottom: .5rem; gap: .14rem; } .poster-copy strong { font-size: 0.76rem; line-height: 1.1; } .poster-copy small, .poster-copy em { font-size: 0.62rem; line-height: 1.15; } .side-column { display: none; } }
  @media (max-width: 420px) { .film-hero-carousel { display: block; height: 235px; } .poster-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); } .poster-card { min-height: 174px; } .poster-copy { left: .45rem; right: .45rem; bottom: .45rem; } }

  .mobile-link-search { display: none; }
  .mobile-link-search-top { display: none; }
  @media (max-width: 720px) {
    .mobile-link-search-top {
      position: relative;
      z-index: 40;
      display: grid;
      grid-template-columns: 22px minmax(0, 1fr) 46px;
      gap: .52rem;
      align-items: center;
      min-height: 56px;
      margin: .02rem 0 .45rem;
      padding: .38rem .42rem .38rem .72rem;
      border: 1px solid rgba(148,163,184,.14);
      border-radius: 16px;
      background: linear-gradient(180deg,rgba(14,20,34,.94),rgba(8,12,22,.82));
      box-shadow: 0 10px 26px rgba(0,0,0,.28), inset 0 1px 0 rgba(255,255,255,.045);
      pointer-events: auto;
      touch-action: manipulation;
    }
    .mobile-link-search-top > .material-icons {
      color: rgba(248,250,252,.92);
      font-size: 1.28rem;
    }
    .mobile-link-search-top input {
      min-width: 0;
      width: 100%;
      height: 46px;
      border: 0;
      outline: 0;
      background: transparent;
      color: #f8fafc;
      font-size: 16px;
      font-weight: 850;
      line-height: 46px;
      appearance: none;
      -webkit-appearance: none;
      -webkit-user-select: text;
      user-select: text;
      pointer-events: auto;
      touch-action: auto;
    }
    .mobile-link-search-top input::placeholder { color: rgba(226,232,240,.38); font-weight: 780; }
    .mobile-link-search-top input:focus { box-shadow: none; }
    .mobile-link-search-top .check-link-button {
      width: 46px;
      min-width: 46px;
      height: 46px;
      padding: 0;
      display: grid;
      place-items: center;
      border: 0;
      border-radius: 14px;
      color: #080a12;
      background: linear-gradient(135deg,#f8c14a,#c4b5fd);
      box-shadow: 0 8px 20px rgba(167,139,250,.18);
      pointer-events: auto;
      touch-action: manipulation;
    }
    .mobile-link-search-top .check-link-button:disabled { opacity: .45; }
    .mobile-link-search-top .check-link-button .material-icons { color: #080a12; font-size: 1.22rem; }
  }


  .download-bulk-panel{display:grid;gap:.48rem;margin:0 0 .6rem 0;padding:.62rem;border:1px solid rgba(248,193,74,.22);border-radius:14px;background:rgba(248,193,74,.06);position:relative;z-index:2}
  .bulk-actions{display:flex;align-items:center;gap:.42rem;flex-wrap:wrap}.bulk-actions strong{color:#fff;font-size:.82rem;margin-right:auto}.bulk-actions button{min-height:30px;border-radius:10px;border:1px solid rgba(255,255,255,.12);background:rgba(255,255,255,.06);color:#e5e7eb;font-size:.72rem;font-weight:900;padding:0 .55rem}
  .mode-line{display:grid;grid-template-columns:1fr 1fr 32px;gap:.32rem}.mode-line button{min-height:36px;border-radius:11px;border:1px solid rgba(255,255,255,.12);background:rgba(2,6,23,.5);color:#cbd5e1;font-weight:950}.mode-line button.active{color:#111827;background:linear-gradient(135deg,#f8c14a,#fb7185);border-color:transparent}.mode-line .help-dot{color:#f8c14a;background:rgba(248,193,74,.12)}
  .series-help{margin:0;padding:.5rem .58rem;border-radius:11px;background:rgba(56,189,248,.08);color:#cfe8ff;font-size:.72rem;line-height:1.35}.series-help b{color:#fde68a}
  .bulk-download-btn{min-height:40px;border:0;border-radius:12px;background:linear-gradient(135deg,#f8c14a,#fb7185);color:#111827;font-weight:1000}.bulk-download-btn:disabled{opacity:.58}
  .download-toggle-row{display:grid;grid-template-columns:38px minmax(0,1fr);align-items:stretch}.download-check{display:grid;place-items:center;cursor:pointer}.download-check input{position:absolute;opacity:0;pointer-events:none}.download-check .material-icons{width:30px;height:30px;display:grid;place-items:center;border-radius:10px;color:#111827;background:linear-gradient(135deg,#f8c14a,#fb7185);font-size:1rem}.download-check input:checked + .material-icons{background:linear-gradient(135deg,#86efac,#22c55e)}
  .download-toggle{grid-template-columns:minmax(0,1fr) auto 24px!important}
  @media(max-width:720px){.related-downloads{grid-template-rows:auto auto minmax(0,1fr);max-height:560px}.download-bulk-panel{padding:.5rem;border-radius:12px}.bulk-actions{gap:.3rem}.bulk-actions strong{flex:1 1 100%;font-size:.78rem}.bulk-actions button{min-height:28px;font-size:.68rem;padding:0 .5rem}.mode-line{grid-template-columns:1fr 1fr 30px}.mode-line button{min-height:32px;font-size:.76rem}.bulk-download-btn{min-height:36px;font-size:.82rem}.download-toggle-row{grid-template-columns:36px minmax(0,1fr)}.download-toggle{grid-template-columns:minmax(0,1fr) 22px!important}.download-status-badge{display:none}.download-detail{margin:.36rem 0 .08rem 0}}


/* Discovery bulk download layout hard override */
.related-downloads{display:flex!important;flex-direction:column!important;grid-template-rows:none!important;max-height:560px!important;overflow:hidden!important}
.related-downloads>.related-head{flex:0 0 auto!important}
.related-downloads>.download-bulk-panel{flex:0 0 auto!important;order:2!important;margin:.1rem 0 .35rem!important;z-index:5!important}
.related-downloads>.download-list{flex:1 1 auto!important;min-height:0!important;max-height:none!important;order:3!important;overflow-y:auto!important;position:relative!important;z-index:1!important}
.download-row{position:relative!important;z-index:1!important}
.download-toggle-row{display:grid!important;grid-template-columns:42px minmax(0,1fr)!important;align-items:center!important}
.download-check{height:100%!important;min-height:54px!important;display:grid!important;place-items:center!important}
.download-check .select-box{width:24px!important;height:24px!important;display:grid!important;place-items:center!important;border-radius:8px!important;border:2px solid rgba(248,193,74,.72)!important;background:rgba(2,6,23,.78)!important;color:transparent!important;box-shadow:inset 0 1px 0 rgba(255,255,255,.08)!important}
.download-check input:checked + .select-box{border-color:transparent!important;background:linear-gradient(135deg,#86efac,#22c55e)!important;color:#052e16!important}
.download-check .select-box .material-icons{font-size:16px!important;line-height:1!important;background:transparent!important;color:inherit!important;width:auto!important;height:auto!important;border-radius:0!important}
.download-toggle{grid-template-columns:minmax(0,1fr) auto 24px!important}
.download-detail{margin:.42rem 0 .08rem 42px!important}
@media(max-width:720px){.related-downloads{max-height:580px!important}.download-bulk-panel{padding:.48rem!important}.download-toggle-row{grid-template-columns:38px minmax(0,1fr)!important}.download-check .select-box{width:22px!important;height:22px!important}.download-detail{margin:.38rem 0 .08rem 38px!important}.download-toggle{grid-template-columns:minmax(0,1fr) 22px!important}.download-status-badge{display:none!important}}

</style>
