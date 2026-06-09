/**
 * Library bridge store
 *
 * Manages external library data proxied through FHub's compatibility endpoints.
 * This keeps discovery, saved titles, and automation status in one place inside FHub.
 */

import { writable, derived, get } from 'svelte/store';
import { toasts } from './toasts';

// ============================================================================
// Types — field names match backend serde serialization (camelCase)
// ============================================================================

export interface LibraryOverview {
  sonarr_connected: boolean;
  radarr_connected: boolean;
  series_count: number;
  movie_count: number;
  total_episodes: number;
  episodes_with_files: number;
  episodes_missing: number;
  movies_with_files: number;
  movies_missing: number;
  total_size_on_disk: number;
}

export interface SeriesStats {
  seasonCount?: number;
  episodeFileCount?: number;
  episodeCount?: number;
  totalEpisodeCount?: number;
  sizeOnDisk?: number;
  percentOfEpisodes?: number;
}

export interface TrackedSeries {
  id: number;
  title: string;
  tvdbId?: number;
  tmdbId?: number;
  path?: string;
  year?: number;
  overview?: string;
  status?: string;
  monitored?: boolean;
  images?: MediaImage[];
  qualityProfileId?: number;
  statistics?: SeriesStats;
}

export interface MovieCollection {
  title: string;
  tmdbId: number;
}

export interface TrackedMovie {
  id: number;
  title: string;
  tmdbId: number;
  path?: string;
  year?: number;
  overview?: string;
  status?: string;
  monitored?: boolean;
  hasFile?: boolean;
  sizeOnDisk?: number;
  images?: MediaImage[];
  qualityProfileId?: number;
  runtime?: number;
  /** Movie collection grouping (e.g. Marvel Cinematic Universe) */
  collection?: MovieCollection;
}

export interface MediaImage {
  coverType: string;
  url?: string;
  remoteUrl?: string;
}

export interface CalendarEntry {
  id: number;
  seriesId: number;
  title?: string;
  seasonNumber: number;
  episodeNumber: number;
  airDateUtc?: string;
  hasFile: boolean;
  series?: {
    id: number;
    title: string;
    images?: MediaImage[];
  };
}

export interface DiskSpace {
  path: string;
  label?: string;
  freeSpace: number;
  totalSpace: number;
}

export interface LibraryServiceStatus {
  connected: boolean;
  version?: string;
  start_time?: string;
  health_issues: Array<{
    source?: string;
    check_type?: string;
    message?: string;
  }>;
}

// ============================================================================
// Stores
// ============================================================================

export const libraryOverview = writable<LibraryOverview | null>(null);
export const calendarEntries = writable<CalendarEntry[]>([]);
export const diskSpaces = writable<DiskSpace[]>([]);
export const libraryServiceStatus = writable<{ tv?: LibraryServiceStatus; movie?: LibraryServiceStatus } | null>(null);
export const libraryBridgeLoading = writable(false);
export const libraryBridgeError = writable<string | null>(null);

// ============================================================================
// API Functions
// ============================================================================

const API_BASE = '/api/arr';
const LIBRARY_BRIDGE_TIMEOUT_MS = 5000; // 5 second timeout to prevent UI blocking

/** Fetch with timeout — aborts if external library APIs don't respond quickly */
function fetchWithTimeout(url: string, timeoutMs: number = LIBRARY_BRIDGE_TIMEOUT_MS): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  return fetch(url, { signal: controller.signal }).finally(() => clearTimeout(timer));
}

export async function fetchLibraryOverview(): Promise<LibraryOverview | null> {
  try {
    libraryBridgeLoading.set(true);
    const res = await fetchWithTimeout(`${API_BASE}/library`);
    if (!res.ok) {
      if (res.status === 503) {
        libraryBridgeError.set('External library services not configured');
        return null;
      }
      throw new Error(`HTTP ${res.status}`);
    }
    const data: LibraryOverview = await res.json();
    libraryOverview.set(data);
    libraryBridgeError.set(null);
    return data;
  } catch (e: any) {
    libraryBridgeError.set(e.message);
    toasts.error(`Media Error: ${e.message}`);
    return null;
  } finally {
    libraryBridgeLoading.set(false);
  }
}

export async function fetchCalendar(days: number = 14): Promise<CalendarEntry[]> {
  try {
    const start = new Date().toISOString().split('T')[0];
    const end = new Date(Date.now() + days * 86400000).toISOString().split('T')[0];
    const res = await fetchWithTimeout(`${API_BASE}/calendar?start=${start}&end=${end}`);
    if (!res.ok) return [];
    const data: CalendarEntry[] = await res.json();
    calendarEntries.set(data);
    return data;
  } catch (e: any) {
    toasts.error(`Calendar Error: ${e.message}`);
    return [];
  }
}

export async function fetchDiskSpace(): Promise<DiskSpace[]> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/storage`);
    if (!res.ok) return [];
    const data: DiskSpace[] = await res.json();
    diskSpaces.set(data);
    return data;
  } catch (e: any) {
    toasts.error(`Storage Error: ${e.message}`);
    return [];
  }
}

export async function fetchLibraryServiceStatus(): Promise<void> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/health`);
    if (!res.ok) return;
    const data = await res.json();
    libraryServiceStatus.set({ tv: data.sonarr, movie: data.radarr });
  } catch {
    // silently fail
  }
}

export async function fetchMissing(page: number = 1, pageSize: number = 10): Promise<any> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/missing?page=${page}&page_size=${pageSize}`);
    if (!res.ok) return null;
    return await res.json();
  } catch (e: any) {
    toasts.error(`Missing Items Error: ${e.message}`);
    return null;
  }
}

export async function fetchAllSeries(): Promise<TrackedSeries[]> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/series`);
    if (!res.ok) return [];
    return await res.json();
  } catch (e: any) {
    toasts.error(`Series Fetch Error: ${e.message}`);
    return [];
  }
}

export async function fetchAllMovies(): Promise<TrackedMovie[]> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/movies`);
    if (!res.ok) return [];
    return await res.json();
  } catch (e: any) {
    toasts.error(`Movies Fetch Error: ${e.message}`);
    return [];
  }
}

export async function fetchHistory(pageSize: number = 20): Promise<any> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/history?page_size=${pageSize}`);
    if (!res.ok) return null;
    return await res.json();
  } catch (e: any) {
    toasts.error(`History Fetch Error: ${e.message}`);
    return null;
  }
}

// ============================================================================
// Helpers
// ============================================================================

export function formatDiskSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return (bytes / Math.pow(k, i)).toFixed(1) + ' ' + units[i];
}

export function getSeriesPoster(series: TrackedSeries): string | null {
  const poster = series.images?.find(i => i.coverType === 'poster');
  return poster?.remoteUrl || poster?.url || null;
}

export function getSeriesBanner(series: TrackedSeries): string | null {
  const banner = series.images?.find(i => i.coverType === 'banner' || i.coverType === 'fanart');
  return banner?.remoteUrl || banner?.url || null;
}

export function getMoviePoster(movie: TrackedMovie): string | null {
  const poster = movie.images?.find(i => i.coverType === 'poster');
  return poster?.remoteUrl || poster?.url || null;
}

// ============================================================================
// Cross-reference helpers — bridge TMDB ↔ external library services
// ============================================================================

const TMDB_IMAGE_BASE = 'https://image.tmdb.org/t/p';

/** Get a TMDB poster URL from a tmdbId (constructs the path via API lookup) */
export function tmdbPosterUrl(posterPath: string | null, size: 'w185' | 'w342' | 'w500' = 'w342'): string | null {
  if (!posterPath) return null;
  return `${TMDB_IMAGE_BASE}/${size}${posterPath}`;
}

/** Find a tracked series by TMDB ID from a pre-fetched list */
export function findSeriesInList(allSeries: TrackedSeries[], tmdbId: number): TrackedSeries | null {
  return allSeries.find(s => s.tmdbId === tmdbId) || null;
}

/** Find a tracked movie by TMDB ID from a pre-fetched list */
export function findMovieInList(allMovies: TrackedMovie[], tmdbId: number): TrackedMovie | null {
  return allMovies.find(m => m.tmdbId === tmdbId) || null;
}

/** Fetch tracked episodes for a given external library series ID */
export async function fetchTrackedEpisodesBySeriesId(seriesId: number): Promise<TrackedEpisode[]> {
  try {
    const res = await fetchWithTimeout(`${API_BASE}/episodes?series_id=${seriesId}`);
    if (!res.ok) return [];
    return await res.json();
  } catch {
    return [];
  }
}

export interface TrackedEpisode {
  id: number;
  seasonNumber: number;
  episodeNumber: number;
  title?: string;
  airDateUtc?: string;
  hasFile: boolean;
  overview?: string;
}
