<script lang="ts">
  import { goto } from "$app/navigation";
  import Badge from "../ui/Badge.svelte";
  import Button from "../ui/Button.svelte";
  import { toasts } from "$lib/stores/toasts";

  interface Props {
    id?: number;
    title: string;
    posterPath?: string | null;
    posterUrl?: string | null;
    voteAverage?: number;
    releaseDate?: string;
    mediaType?: "movie" | "tv";
    fcode: string;
    originalFilename: string;
    fileSize: number;
    score: number;
    quality?: string;
    resolution?: string;
    source?: string;
    episodeTag?: string;
    hasVietsub?: boolean;
    hasVietdub?: boolean;
    bookmarked?: boolean;
    onDownload?: () => void;
    onCopyUrl?: () => void;
    onToggleBookmark?: () => void;
    onClick?: () => void;
  }

  let {
    id,
    title,
    posterPath,
    posterUrl,
    voteAverage = 0,
    releaseDate = "",
    mediaType = "movie",
    fcode,
    originalFilename,
    fileSize,
    score,
    quality,
    resolution,
    source,
    episodeTag,
    hasVietsub = false,
    hasVietdub = false,
    bookmarked = false,
    onDownload,
    onCopyUrl,
    onToggleBookmark,
    onClick,
  }: Props = $props();

  function getPosterUrl(): string {
    if (posterPath) return `https://image.tmdb.org/t/p/w342${posterPath}`;
    if (posterUrl) return posterUrl;
    return "/images/placeholder-poster.svg";
  }

  function getYear(date: string): string {
    return date?.substring(0, 4) || "";
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }

  function handleCardClick() {
    if (onClick) onClick();
    else if (id) goto(`/${mediaType}/${id}`);
  }

  function handleDownload(e: Event) {
    e.stopPropagation();
    onDownload?.();
  }

  function handleCopyUrl(e: Event) {
    e.stopPropagation();
    if (onCopyUrl) onCopyUrl();
    else {
      const url = `https://fshare.vn/file/${fcode}`;
      navigator.clipboard
        .writeText(url)
        .then(() => toasts.success(`Link copied — ${url}`))
        .catch(() => toasts.error("Failed to copy link to clipboard"));
    }
  }

  function handleToggleBookmark(e: Event) {
    e.stopPropagation();
    onToggleBookmark?.();
  }
</script>

<div
  class="search-result-card-v3"
  class:is-bookmarked={bookmarked}
  role="button"
  tabindex="0"
  onclick={handleCardClick}
  onkeydown={(e) => e.key === "Enter" && handleCardClick()}
>
  <div class="card-inner">
    <img src={getPosterUrl()} alt={title} loading="lazy" />
    <div class="card-shine"></div>

    <button
      class="bookmark-fab"
      class:active={bookmarked}
      onclick={handleToggleBookmark}
      title={bookmarked ? "Remove from Saved" : "Save for later"}
    >
      <span class="material-icons">{bookmarked ? "star" : "star_outline"}</span>
    </button>

    {#if episodeTag}
      <div class="episode-badge">{episodeTag}</div>
    {/if}

    <div class="quality-tags">
      {#if resolution}
        <Badge text={resolution} variant="quality" size="sm" />
      {/if}
      {#if source}
        <Badge text={source} variant="source" size="sm" />
      {/if}
      {#if hasVietsub}
        <Badge text="Vietsub" variant="language" size="sm" color="#ff6b6b" />
      {/if}
      {#if hasVietdub}
        <Badge text="Vietdub" variant="language" size="sm" color="#ffa500" />
      {/if}
    </div>

    <div class="card-overlay">
      <div class="overlay-top">
        <h3 class="card-title">{title}</h3>
        <div class="card-meta">
          {#if releaseDate}
            <span class="meta-year">{getYear(releaseDate)}</span>
          {/if}
          <span class="meta-rating"
            ><span class="material-icons rating-icon">star</span>
            {voteAverage?.toFixed(1) || "N/A"}</span
          >
        </div>
        <div class="filename" title={originalFilename}>{originalFilename}</div>
      </div>

      <div class="overlay-bottom">
        <div class="file-info">
          <span class="material-icons info-icon">storage</span>
          <span class="file-size">{formatSize(fileSize)}</span>
          {#if bookmarked}
            <span class="saved-pill">Saved</span>
          {/if}
        </div>

        <div class="card-actions">
          <Button size="sm" icon="download" onclick={handleDownload}>Download</Button>
          <Button
            size="sm"
            variant="ghost"
            icon={bookmarked ? "star" : "star_outline"}
            onclick={handleToggleBookmark}
            title={bookmarked ? "Remove from Saved" : "Save for later"}
          ></Button>
          <Button
            size="sm"
            variant="ghost"
            icon="link"
            onclick={handleCopyUrl}
            title="Copy URL"
          ></Button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .search-result-card-v3 {
    position: relative;
    aspect-ratio: 2/3;
    width: 100%;
    cursor: pointer;
    transition: transform 0.35s ease;
    z-index: 1;
  }

  .card-inner {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 18px;
    overflow: hidden;
    background: #0a0f1e;
    border: 1px solid rgba(255, 255, 255, 0.06);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
    transition: all 0.35s ease;
  }

  .search-result-card-v3:hover {
    transform: translateY(-6px) scale(1.02);
    z-index: 10;
  }

  .search-result-card-v3:hover .card-inner {
    border-color: rgba(255, 138, 31, 0.35);
    box-shadow: 0 20px 50px -10px rgba(255, 138, 31, 0.2);
  }

  .search-result-card-v3.is-bookmarked .card-inner {
    border-color: rgba(255, 215, 82, 0.4);
    box-shadow:
      0 18px 45px -14px rgba(255, 215, 82, 0.18),
      inset 0 0 0 1px rgba(255, 215, 82, 0.12);
  }

  .card-inner img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.6s ease;
  }

  .search-result-card-v3:hover img {
    transform: scale(1.08);
  }

  .card-shine {
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, transparent 0%, rgba(255, 255, 255, 0.06) 50%, transparent 100%);
    transform: translateX(-100%);
    transition: transform 0.7s ease;
    z-index: 1;
  }

  .search-result-card-v3:hover .card-shine {
    transform: translateX(100%);
  }

  .bookmark-fab {
    position: absolute;
    top: 0.85rem;
    right: 0.85rem;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.16);
    background: rgba(7, 10, 18, 0.72);
    backdrop-filter: blur(10px);
    color: rgba(255, 255, 255, 0.88);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 7;
    cursor: pointer;
    transition: all 0.25s ease;
  }

  .bookmark-fab:hover {
    transform: scale(1.08);
    border-color: rgba(255, 215, 82, 0.4);
  }

  .bookmark-fab.active {
    color: #ffd54a;
    background: rgba(52, 40, 5, 0.82);
    border-color: rgba(255, 215, 82, 0.38);
    box-shadow: 0 0 18px rgba(255, 215, 82, 0.18);
  }

  .bookmark-fab .material-icons {
    font-size: 1.1rem;
  }

  .episode-badge {
    position: absolute;
    top: 0.95rem;
    left: 0.95rem;
    background: rgba(138, 43, 226, 0.88);
    color: #fff;
    font-size: 0.76rem;
    font-weight: 800;
    padding: 0.4rem 0.55rem;
    border-radius: 8px;
    z-index: 5;
  }

  .quality-tags {
    position: absolute;
    top: 3.5rem;
    left: 0.95rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    z-index: 5;
    align-items: flex-start;
  }

  .card-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 1rem;
    background: linear-gradient(to top, rgba(10, 15, 30, 0.98) 0%, rgba(10, 15, 30, 0.76) 45%, transparent 100%);
    z-index: 2;
  }

  .card-title {
    margin: 0 0 0.35rem;
    font-size: 1rem;
    font-weight: 800;
    color: #fff;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.72);
    margin-bottom: 0.45rem;
  }

  .meta-year {
    color: var(--color-primary, #ff8a1f);
    font-weight: 700;
    font-family: var(--font-mono, monospace);
  }

  .rating-icon {
    font-size: 0.82rem;
    vertical-align: middle;
    color: #f59e0b;
  }

  .filename {
    font-size: 0.72rem;
    color: rgba(255, 255, 255, 0.55);
    font-family: var(--font-mono, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .overlay-bottom {
    margin-top: 0.75rem;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.58);
    margin-bottom: 0.85rem;
  }

  .info-icon {
    font-size: 14px;
  }

  .file-size {
    font-family: var(--font-mono, monospace);
    font-weight: 600;
  }

  .saved-pill {
    padding: 0.18rem 0.45rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 215, 82, 0.32);
    background: rgba(255, 215, 82, 0.12);
    color: #ffd54a;
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .card-actions {
    display: flex;
    gap: 0.45rem;
    align-items: center;
  }

  .card-actions :global(.fhub-btn:first-child) {
    flex: 1;
  }
</style>
