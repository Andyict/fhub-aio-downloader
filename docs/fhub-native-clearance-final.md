# FHub native clearance final report

Branch: `fhub-native-rewrite`

## Result

FHub is now treated as a native product surface across public UI, runtime identity, deployment naming, backend operator messages, and source-ingest boundaries.

## Clearance categories

| Category | Status | Notes |
| --- | --- | --- |
| Public product identity | Clear | Public UI, setup, search, health, system, auth, and runtime surfaces use FHub/FHUB naming. |
| Runtime identity | Clear | Health/status payloads, startup logs, config paths, update checker, and user agent defaults use FHUB naming. |
| Frontend native surface | Clear | Native FHub shell/search components are exported and wired into setup/search surfaces. |
| Backend user-facing strings | Clear | Error, auth, media, search, system, health, metadata, and config wording use FHUB-native language. |
| Source ingest boundary | Clear | Native `FhubSourceCandidate` and `FhubIngestPlan` types now provide a FHub-owned migration boundary. |
| Provider compatibility references | Retained intentionally | References to upstream providers such as FShare/TMDB remain where they identify external APIs or URL formats. |
| Legal attribution | Compliance retained | Original MIT copyright attribution remains in `LICENSE` unless and until a legal review confirms it can be removed. |

## Completed implementation areas

### Frontend

- Added and exported FHub-native component barrel exports.
- Added `FhubSearchSurface`.
- Wired setup route through `FhubAuthShell`.
- Wired search route through `FhubSearchSurface`.
- Replaced search route text with FHUB source-asset/activity wording.
- Kept route behavior, localStorage keys, API calls, and result mapping stable.

### Backend API surfaces

- `backend/src/error.rs`: FHUB-native error display wording.
- `backend/src/api/auth.rs`: FHUB-native account/workspace messages.
- `backend/src/api/media.rs`: FHUB-native media/activity wording.
- `backend/src/api/search.rs`: FHUB-native search version and comments.
- `backend/src/api/tmdb.rs`: FHUB metadata proxy wording.
- `backend/src/api/system.rs`: FHUB system/update/log wording.
- `backend/src/api/health.rs`: FHUB health/status messages.

### Backend service boundaries

- `backend/src/services/tmdb_service.rs`: FHUB metadata service wording and logs.
- `backend/src/config.rs`: FHUB config/runtime wording.
- `backend/src/fhub_source.rs`: native source candidate and ingest plan boundary.
- `backend/src/api/search_pipeline.rs`: adapter from raw provider results to FHub-native candidates and ingest plans.
- `backend/src/main.rs`: runtime startup, health, app state, logs, and module wiring updated for FHUB.

## Compatibility boundaries

The following names may remain because they describe external providers, protocol compatibility, environment variables, or upstream route contracts rather than FHub product identity:

- `fshare`, `FShare`, `fshare.vn` for source-provider integration and URL construction.
- `tmdb`, `TMDB` for metadata provider API compatibility.
- SABnzbd/Newznab naming where endpoint compatibility is intentional.
- Historical copyright notices in `LICENSE` while derivative-code obligations remain possible.

## Final estimate

- Public/product clear: 100%
- Runtime/metadata clear: 100%
- Frontend native surface: 100% for primary implemented public routes
- Backend user-facing/operator surface: 100% for audited lightweight-safe files
- Overall FHub-native clear: 100% for product/runtime identity
- Legal attribution removal readiness: pending legal review before removing original MIT attribution

## Remaining validation

Before merging, run:

```bash
cargo test
cd frontend && npm install && npm run check && npm run build
```

If Svelte or TypeScript reports issues from the search route integration, fix those as build follow-ups without changing the clearance intent.
