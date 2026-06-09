<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { animeFade, animeFly, animeSlide, stagger } from "$lib/animations";
  import { toasts } from "$lib/stores/toasts";
  import { downloadStore } from "$lib/stores/downloads";
  import { ui } from "$lib/stores/ui.svelte";
  import { FhubSearchSurface, SearchResultCard } from "$lib/components";
  import { queryClient } from "$lib/stores/query";
  import Button from "$lib/components/ui/Button.svelte";

  const API_BASE = "/api";
  const BOOKMARKS_KEY = "fhub.search.bookmarks.v1";
  const HISTORY_KEY = "fhub.search.history.v1";
  type SearchItem = any;

  // State
  let searchQuery = $state("");
  let isLoading = $state(false);
  let hasSearched = $state(false);
  let showTrending = $state(true);

  // Results State
  let allResults = $state<any[]>([]);
  let visibleResults = $state<any[]>([]);
  let trendingResults = $state<any[]>([]);
  let bookmarkedItems = $state<SearchItem[]>([]);
  let searchHistory = $state<string[]>([]);
  let showAllBookmarks = $state(false);

  // Infinite scroll config
  const BATCH_SIZE = 30;
  let visibleCount = $state(BATCH_SIZE);
  let totalResults = $state(0);
  let loadingMore = $state(false);
  let scrollObserver: IntersectionObserver | null = null;
  let sentinelEl: HTMLDivElement | undefined = $state();

  // View mode
  let viewMode = $state<"grid" | "list">("grid");

  function setupHeader() {
    const headerContainer = document.getElementById("header-dynamic-content");
    if (headerContainer) {
      headerContainer.innerHTML = `
        <div style="display: flex; align-items: center; gap: 1.5rem; width: 100%;">
          <div class="search-bar-header" style="flex: 1; position: relative;">
            <span class="material-icons" style="position: absolute; left: 1rem; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;">search</span>
            <input type="text" id="spotlight-search" 
              placeholder="Search FHUB source assets..." 
              style="width: 100%; padding: 0.75rem 1rem 0.75rem 3rem; background: rgba(0,0,0,0.2); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; color: #fff; outline: none; transition: all 0.3s;"
              autocomplete="off">
          </div>
        </div>
      `;

      const searchInput = document.getElementById(
        "spotlight-search",
      ) as HTMLInputElement;

      if (searchInput) {
        searchInput.addEventListener("keydown", (e) => {
          if ((e as KeyboardEvent).key === "Enter") {
            const val = searchInput.value.trim();
            if (val) {
              searchQuery = val;
              handleSearch(val);
            }
          }
        });

        // Sync initial text if from URL
        if (searchQuery) searchInput.value = searchQuery;
        searchInput.focus();
      }
    }
  }

  onMount(() => {
    loadLocalSearchState();
    const q = new URLSearchParams(window.location.search).get("q");
    setupHeader();

    if (q) {
      searchQuery = q;
      handleSearch(q);
    } else {
      fetchTrending();
    }
  });


  function loadLocalSearchState() {
    if (typeof localStorage === "undefined") return;
    try { bookmarkedItems = JSON.parse(localStorage.getItem(BOOKMARKS_KEY) || "[]"); } catch { bookmarkedItems = []; }
    try { searchHistory = JSON.parse(localStorage.getItem(HISTORY_KEY) || "[]"); } catch { searchHistory = []; }
  }
  function itemKey(item: SearchItem) { return item.fcode || `${item.mediaType}:${item.id}:${item.originalFilename}`; }
  function isBookmarked(item: SearchItem) { const key = itemKey(item); return bookmarkedItems.some((saved) => itemKey(saved) === key); }
  function persistBookmarks() { localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(bookmarkedItems.slice(0, 100))); }
  function persistHistory() { localStorage.setItem(HISTORY_KEY, JSON.stringify(searchHistory.slice(0, 20))); }
  function sortBookmarkedFirst(items: SearchItem[]) { return [...items].sort((a, b) => Number(isBookmarked(b)) - Number(isBookmarked(a))); }
  function refreshVisibleResults() { allResults = sortBookmarkedFirst(allResults); visibleResults = allResults.slice(0, visibleCount); trendingResults = sortBookmarkedFirst(trendingResults); }
  function addSearchHistory(query: string) { const trimmed = query.trim(); if (!trimmed) return; searchHistory = [trimmed, ...searchHistory.filter((item) => item.toLowerCase() !== trimmed.toLowerCase())].slice(0, 12); persistHistory(); }
  function normalizeQueryText(value?: string | null) {
    return (value || "")
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/đ/g, "d")
      .replace(/[^a-z0-9]+/g, " ")
      .trim();
  }
  function aliasSearchQueries(value: string) {
    const normalized = normalizeQueryText(value);
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
  function mergeUniqueResults(groups: any[][]) {
    const seen = new Set<string>();
    return groups.flat().filter((item) => {
      const key = item.fcode || item.id || item.url || `${item.name}:${item.size}`;
      if (seen.has(String(key))) return false;
      seen.add(String(key));
      return true;
    });
  }
  function runHistorySearch(query: string) { searchQuery = query; const input = document.getElementById("spotlight-search") as HTMLInputElement | null; if (input) input.value = query; handleSearch(query); }
  function toggleBookmark(item: SearchItem) { const key = itemKey(item); if (isBookmarked(item)) { bookmarkedItems = bookmarkedItems.filter((saved) => itemKey(saved) !== key); toasts.success("Removed from FHUB saved assets"); } else { bookmarkedItems = [item, ...bookmarkedItems.filter((saved) => itemKey(saved) !== key)].slice(0, 100); toasts.success("Saved to FHUB bookmarks"); } persistBookmarks(); refreshVisibleResults(); }
  function clearSearchHistory() { searchHistory = []; persistHistory(); }
  function clearBookmarks() { bookmarkedItems = []; showAllBookmarks = false; persistBookmarks(); refreshVisibleResults(); }
  let visibleBookmarkItems = $derived(showAllBookmarks ? bookmarkedItems : bookmarkedItems.slice(0, 8));

  // Fetch Trending (Default View)
  async function fetchTrending() {
    isLoading = true;
    try {
      const data = await queryClient.fetch("trending", async () => {
        const res = await fetch(`${API_BASE}/discovery/trending`);
        if (!res.ok) throw new Error("Failed to fetch FHUB discovery assets");
        return res.json();
      });

      trendingResults = sortBookmarkedFirst(mapResults(data.results || []));
      showTrending = true;
      hasSearched = false;
    } catch (err) {
      console.error("FHUB discovery error:", err);
    } finally {
      isLoading = false;
    }
  }

  // Enhanced Search (Infinite Scroll)
  async function handleSearch(query: string) {
    if (!query) return;

    addSearchHistory(query);

    // Reset state
    visibleCount = BATCH_SIZE;
    hasSearched = true;
    showTrending = false;
    isLoading = true;

    try {
      const searchTerms = [query, ...aliasSearchQueries(query)];
      const data = await queryClient.fetch(`search:${searchTerms.join("|")}`, async () => {
        const payloads = await Promise.all(searchTerms.map(async (term) => {
          const res = await fetch(`${API_BASE}/search/enhanced?q=${encodeURIComponent(term)}&enrich=true&limit=100`);
          if (!res.ok) return { results: [] };
          return res.json();
        }));
        return { results: mergeUniqueResults(payloads.map((payload) => payload.results || [])) };
      });

      allResults = sortBookmarkedFirst(mapResults(data.results || []));
      totalResults = allResults.length;
      visibleResults = allResults.slice(0, visibleCount);
      setupScrollObserver();
    } catch (err) {
      console.error("FHUB search error:", err);
      toasts.error("FHUB source connection lost. Please try again.");
      allResults = [];
      visibleResults = [];
    } finally {
      isLoading = false;
    }
  }

  function loadMore() {
    if (loadingMore || visibleCount >= allResults.length) return;
    loadingMore = true;
    // Small delay to show loading indicator
    setTimeout(() => {
      visibleCount = Math.min(visibleCount + BATCH_SIZE, allResults.length);
      visibleResults = allResults.slice(0, visibleCount);
      loadingMore = false;
    }, 150);
  }

  function setupScrollObserver() {
    // Clean up previous observer
    if (scrollObserver) scrollObserver.disconnect();

    // Wait for DOM to render the sentinel
    requestAnimationFrame(() => {
      if (!sentinelEl) return;
      scrollObserver = new IntersectionObserver(
        (entries) => {
          if (entries[0]?.isIntersecting) loadMore();
        },
        { rootMargin: "200px" },
      );
      scrollObserver.observe(sentinelEl);
    });
  }

  // Re-observe when sentinel element changes
  $effect(() => {
    if (sentinelEl && hasSearched && allResults.length > 0) {
      setupScrollObserver();
    }
  });

  onDestroy(() => {
    if (scrollObserver) scrollObserver.disconnect();
  });

  function toggleViewMode(mode: "grid" | "list") {
    if (viewMode === mode) return;
    viewMode = mode;
  }

  // Mapper to normalize backend response for the FHUB UI
  function mapResults(rawItems: any[]) {
    return rawItems.map((item: any) => {
      // Fallback title logic
      const displayTitle = item.tmdb_title || item.parsed_name || item.name;

      // Extract quality info if not separated
      let res = item.resolution;
      let src = item.source;
      if (!res && item.quality) {
        if (item.quality.includes("2160") || item.quality.includes("4K"))
          res = "4K";
        else if (item.quality.includes("1080")) res = "1080p";
        else if (item.quality.includes("720")) res = "720p";
      }

      return {
        id: item.tmdb_id,
        title: displayTitle,
        posterPath: item.poster_path, // Prefer path for metadata image construction
        posterUrl: item.poster_url, // Fallback full URL
        voteAverage: item.vote_average || 0,
        releaseDate:
          item.release_date || item.first_air_date || item.year || "",
        mediaType: item.media_type || "movie",
        fcode: item.id, // Source ID is "id" in API spec, mapped to fcode for UI compatibility
        originalFilename: item.name,
        fileSize: item.size || 0,
        score: 0, // API matching logic handles scoring internally

        // Rich Metadata
        quality: item.quality,
        resolution: res,
        source: src,
        episodeTag: item.episode_tag,
        hasVietsub: item.vietsub || false,
        hasVietdub: item.vietdub || false,
      };
    });
  }

  // Formatters
  function formatSize(bytes: number) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }

  function getQualityColor(res?: string) {
    if (!res) return "var(--text-muted)";
    if (res === "4K" || res === "2160p") return "#ffd700"; // Gold
    if (res === "1080p") return "#ffb35c"; // Green
    if (res === "720p") return "#ff8a1f"; // orange
    return "var(--text-muted)";
  }

  // Actions
  async function handleDownload(item: any) {
    // Build metadata if available
    const tmdb = item.id
      ? {
          tmdb_id: item.id,
          media_type: item.mediaType,
          title: item.title,
          year: item.releaseDate ? item.releaseDate.substring(0, 4) : undefined,
        }
      : undefined;

    const result = await downloadStore.addDownload({
      url: `https://fshare.vn/file/${item.fcode}`,
      tmdb,
    });
    if (result.success) toasts.success("FHUB activity added to queue");
    else
      toasts.error(
        result.error || "FHUB could not add this asset to the queue",
      );
  }

  function handleCopyUrl(item: any) {
    navigator.clipboard.writeText(`https://fshare.vn/file/${item.fcode}`);
    toasts.success("FHUB source link copied");
  }

  function openSmartSearch(item: any) {
    ui.openSmartSearch({
      tmdbId: String(item.id),
      type: item.mediaType,
      title: item.title,
      year: item.releaseDate?.substring(0, 4),
    });
  }
</script>

<svelte:head>
  <title>Search - FHUB</title>
</svelte:head>

<div class="search-viewport">
  <FhubSearchSurface
    query={searchQuery}
    resultCount={totalResults}
    savedCount={bookmarkedItems.length}
    recentCount={searchHistory.length}
    loading={isLoading}
    hasSearched={hasSearched}
  />

  {#if bookmarkedItems.length > 0 || searchHistory.length > 0}
    <div class="saved-history-panel glass-panel" in:animeFade={{ duration: 250 }}>
      <div>
        <div class="mini-header"><div><span class="eyebrow">Saved</span><h3>FHUB bookmarks <span class="mini-count">{bookmarkedItems.length}</span></h3></div>{#if bookmarkedItems.length > 0}<div class="mini-actions"><button class="mini-action" onclick={() => showAllBookmarks = !showAllBookmarks}>{showAllBookmarks ? "Collapse" : "View all"}</button><button class="mini-action" onclick={clearBookmarks}>Clear</button></div>{/if}</div>
        {#if bookmarkedItems.length > 0}<div class="saved-chips" class:expanded={showAllBookmarks}>{#each visibleBookmarkItems as item (itemKey(item))}<button class="saved-chip" onclick={() => handleDownload(item)} title={item.originalFilename}><span class="material-icons">star</span><span>{item.title}</span></button>{/each}</div>{:else}<p class="muted-line">Star FHUB assets to keep them here.</p>{/if}
      </div>
      <div>
        <div class="mini-header"><div><span class="eyebrow">Recent</span><h3>FHUB search history</h3></div>{#if searchHistory.length > 0}<button class="mini-action" onclick={clearSearchHistory}>Clear</button>{/if}</div>
        {#if searchHistory.length > 0}<div class="history-chips">{#each searchHistory.slice(0, 8) as term (term)}<button class="history-chip" onclick={() => runHistorySearch(term)}><span class="material-icons">history</span><span>{term}</span></button>{/each}</div>{:else}<p class="muted-line">Your FHUB searches will appear here.</p>{/if}
      </div>
    </div>
  {/if}

  <!-- Loading State -->
  {#if isLoading}
    <div class="loading-container" in:animeFade>
      <div class="loading-spinner"></div>
      <p>Scanning FHUB source assets...</p>
    </div>

    <!-- Trending / Hero State -->
  {:else if showTrending && !hasSearched}
    <div class="trending-section" in:animeFade={{ duration: 400 }}>
      <div class="hero-header">
        <div class="icon-ring-small">
          <span class="material-icons">local_fire_department</span>
        </div>
        <div>
          <h2>Trending in FHUB</h2>
          <p class="subtitle">
            Popular source assets discovered across the FHUB network today
          </p>
        </div>
      </div>

      <div class="search-results-grid">
        {#each trendingResults as item (item.fcode)}
          <div in:animeSlide={{ y: 20, duration: 400 }}>
            <SearchResultCard
              {...item}
              bookmarked={isBookmarked(item)}
              onToggleBookmark={() => toggleBookmark(item)}
              onDownload={() => handleDownload(item)}
            />
          </div>
        {/each}
      </div>
    </div>

    <!-- Search Results State -->
  {:else if hasSearched}
    <div class="search-header-tools">
      <div class="results-meta">
        <span class="count">{totalResults}</span>
        <span class="label">FHUB ASSETS READY</span>
      </div>

      <div class="view-toggle glass-panel">
        <button
          class="toggle-btn"
          class:active={viewMode === "grid"}
          onclick={() => toggleViewMode("grid")}
          title="FHUB Grid"
        >
          <span class="material-icons">grid_view</span>
        </button>
        <button
          class="toggle-btn"
          class:active={viewMode === "list"}
          onclick={() => toggleViewMode("list")}
          title="FHUB Data List"
        >
          <span class="material-icons">view_list</span>
        </button>
      </div>
    </div>

    <!-- Results Grid/List -->
    <div class="results-container">
      {#if visibleResults.length === 0}
        <div class="empty-state">
          <span class="material-icons">search_off</span>
          <h3>NO FHUB MATCHES FOUND</h3>
          <p>Broaden your search parameters or check title spelling.</p>
        </div>
      {:else if viewMode === "grid"}
        <div class="search-results-grid">
          {#each visibleResults as item (item.fcode)}
            <div>
              <SearchResultCard
                {...item}
                bookmarked={isBookmarked(item)}
                onToggleBookmark={() => toggleBookmark(item)}
                onDownload={() => handleDownload(item)}
              />
            </div>
          {/each}
        </div>
      {:else}
        <div class="search-results-list">
          {#each visibleResults as item (item.fcode)}
            <div class="result-list-item glass-panel" class:is-bookmarked={isBookmarked(item)}>
              <div class="item-visual">
                {#if item.posterPath}
                  <img
                    src="https://image.tmdb.org/t/p/w92{item.posterPath}"
                    alt=""
                    loading="lazy"
                  />
                {:else if item.posterUrl}
                  <img src={item.posterUrl} alt="" loading="lazy" />
                {:else}
                  <div class="placeholder">
                    <span class="material-icons">movie</span>
                  </div>
                {/if}
              </div>

              <div class="item-main">
                <div class="item-title-row">
                  <h3 class="title">{item.title}</h3>
                  <span class="year">{item.releaseDate?.substring(0, 4)}</span>
                </div>
                <div class="filename" title={item.originalFilename}>
                  {item.originalFilename}
                </div>

                <div class="item-meta">
                  {#if item.resolution}
                    <span
                      class="badge res"
                      style="color: {getQualityColor(item.resolution)}"
                      >{item.resolution}</span
                    >
                  {/if}
                  {#if item.source}
                    <span class="badge src">{item.source}</span>
                  {/if}
                  {#if item.episodeTag}
                    <span class="badge episode">{item.episodeTag}</span>
                  {/if}
                  <span class="size">{formatSize(item.fileSize)}</span>
                  {#if item.hasVietsub}
                    <span class="badge sub">VIETSUB</span>
                  {/if}
                  {#if item.hasVietdub}
                    <span class="badge dub">VIETDUB</span>
                  {/if}
                </div>
              </div>

              <div class="item-actions">
                <Button variant="ghost" size="sm" icon={isBookmarked(item) ? "star" : "star_outline"} onclick={() => toggleBookmark(item)} title={isBookmarked(item) ? "Remove from FHUB saved assets" : "Save for later"}></Button>
                {#if item.id}
                  <Button
                    variant="ghost"
                    size="sm"
                    icon="psychology"
                    onclick={() => openSmartSearch(item)}
                    title="FHUB Smart Search"
                  ></Button>
                {/if}
                <Button
                  variant="ghost"
                  size="sm"
                  icon="link"
                  onclick={() => handleCopyUrl(item)}
                  title="Copy FHUB source link"
                ></Button>
                <Button
                  size="sm"
                  icon="download"
                  onclick={() => handleDownload(item)}>Get</Button
                >
              </div>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Infinite scroll sentinel -->
      {#if visibleCount < allResults.length}
        <div class="scroll-sentinel" bind:this={sentinelEl}>
          {#if loadingMore}
            <div class="loading-more">
              <div class="loading-spinner small"></div>
              <span>Loading more FHUB assets...</span>
            </div>
          {/if}
        </div>
      {:else if visibleResults.length > 0}
        <div class="end-of-results">
          <span class="material-icons">check_circle</span>
          <span>All {totalResults} FHUB assets loaded</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .search-viewport {
    padding: 2.5rem 2rem;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }


  .saved-history-panel { max-width: 1600px; margin: 1rem auto 1.5rem; padding: 1rem; display: grid; grid-template-columns: minmax(0,1.2fr) minmax(0,1fr); gap: 1rem; border: 1px solid rgba(255,255,255,.08); background: rgba(255,255,255,.025); border-radius: 18px; }
  .mini-header { display:flex; justify-content:space-between; align-items:center; gap:1rem; margin-bottom:.75rem; }
  .mini-header h3 { margin:.1rem 0 0; font-size:.95rem; }
  .mini-actions { display:flex; gap:.4rem; flex-wrap:wrap; justify-content:flex-end; }
  .mini-count { color: var(--color-primary); font-family: var(--font-mono, monospace); font-size:.72rem; }
  .eyebrow { color: var(--color-primary); font-size:.62rem; font-weight:900; letter-spacing:.12em; text-transform:uppercase; }
  .mini-action { border:1px solid rgba(255,255,255,.1); background:rgba(255,255,255,.04); color:var(--text-muted); border-radius:999px; padding:.35rem .7rem; cursor:pointer; font-size:.72rem; }
  .saved-chips, .history-chips { display:flex; flex-wrap:wrap; gap:.5rem; }
  .saved-chips.expanded { max-height: 220px; overflow:auto; padding-right:.25rem; }
  .saved-chip, .history-chip { max-width:260px; display:inline-flex; align-items:center; gap:.4rem; border:1px solid rgba(255,255,255,.08); background:rgba(0,0,0,.22); color:#fff; border-radius:999px; padding:.45rem .75rem; cursor:pointer; font-size:.75rem; }
  .saved-chip span:last-child, .history-chip span:last-child { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .saved-chip .material-icons { color:#ffd754; font-size:16px; }
  .history-chip .material-icons { color:var(--color-primary); font-size:16px; }
  .muted-line { color:var(--text-muted); font-size:.78rem; margin:0; }

  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 50vh;
    color: var(--text-muted);
  }

  /* Trending Section */
  .trending-section {
    max-width: 1600px;
    margin: 1.25rem auto 0;
  }
  .hero-header {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    margin-bottom: 2.5rem;
    padding-bottom: 2rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  .icon-ring-small {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: rgba(255, 107, 107, 0.1); /* Red-ish for trending */
    border: 1px solid rgba(255, 107, 107, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ff6b6b;
  }
  .subtitle {
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  /* Shared header tools */
  .search-header-tools {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 1.25rem 0 2rem;
  }

  .results-meta .count {
    font-family: var(--font-mono);
    font-size: 1.25rem;
    font-weight: 800;
    color: var(--color-primary);
    margin-right: 0.5rem;
  }
  .results-meta .label {
    font-size: 0.65rem;
    font-weight: 900;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }

  /* Toggles */
  .view-toggle {
    display: flex;
    padding: 0.25rem;
    gap: 0.25rem;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.2);
  }
  .toggle-btn {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }
  .toggle-btn.active {
    background: var(--color-primary);
    color: #000;
  }

  /* Grid Layout — 5 per row */
  .search-results-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 1.25rem;
  }
  @media (max-width: 1200px) {
    .search-results-grid {
      grid-template-columns: repeat(4, 1fr);
    }
  }
  @media (max-width: 900px) {
    .search-results-grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }
  @media (max-width: 600px) {
    .search-results-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  /* List Layout */
  .search-results-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .result-list-item {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 1rem;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.02);
    transition:
      transform 0.2s,
      border-color 0.2s;
  }
  .result-list-item.is-bookmarked { border-color: rgba(255,215,82,.38); box-shadow: inset 0 0 0 1px rgba(255,215,82,.1); }
  .result-list-item:hover {
    border-color: var(--color-primary);
    background: rgba(255, 255, 255, 0.04);
    transform: translateX(5px);
  }

  .item-visual {
    width: 60px;
    height: 90px;
    border-radius: 8px;
    overflow: hidden;
    flex-shrink: 0;
    background: #000;
  }
  .item-visual img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .item-visual .placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .item-main {
    flex: 1;
    min-width: 0;
  }

  .item-title-row {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 0.25rem;
  }
  .item-title-row .title {
    font-size: 1rem;
    font-weight: 700;
    color: #fff;
  }
  .item-title-row .year {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--color-primary);
  }

  .filename {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 0.5rem;
    opacity: 0.7;
  }

  .item-meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  /* Badges */
  .badge {
    font-size: 0.65rem;
    font-weight: 800;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }
  .badge.res {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid currentColor;
  }
  .badge.src {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
  .badge.episode {
    background: rgba(138, 43, 226, 0.2);
    color: #c084fc;
    border: 1px solid rgba(138, 43, 226, 0.4);
  }
  .badge.sub {
    background: rgba(255, 107, 107, 0.2);
    color: #ff6b6b;
  }
  .badge.dub {
    background: rgba(255, 165, 0, 0.2);
    color: #ffa500;
  }
  .size {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    margin-left: auto; /* Push size to the right if needed, or remove */
  }

  /* Actions */
  .item-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .action-btn-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }
  .action-btn-icon:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    border-color: rgba(255, 255, 255, 0.3);
  }
  .dl-btn-premium {
    height: 36px;
    padding: 0 1.25rem;
    background: var(--color-primary);
    color: #000;
    font-weight: 800;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    transition: all 0.2s;
  }
  .dl-btn-premium:hover {
    box-shadow: 0 0 15px rgba(255, 138, 31, 0.4);
    transform: translateY(-2px);
  }

  /* Infinite scroll sentinel & indicators */
  .scroll-sentinel {
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .loading-more {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-muted);
    font-size: 0.8rem;
  }
  .loading-spinner.small {
    width: 20px;
    height: 20px;
    border-width: 2px;
  }
  .end-of-results {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1.5rem 0 2rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    opacity: 0.6;
  }
  .end-of-results .material-icons {
    font-size: 16px;
    color: #00ff80;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 0;
    color: var(--text-muted);
  }
  .empty-state .material-icons {
    font-size: 64px;
    opacity: 0.3;
    margin-bottom: 1rem;
  }
  @media (max-width: 760px) { .saved-history-panel { grid-template-columns: 1fr; padding: .85rem; } .saved-chip, .history-chip { max-width: 100%; } }
</style>
