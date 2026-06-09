# FHub backend native audit

Branch: `fhub-native-rewrite`

## Goal

Move the backend from public rebrand readiness toward an independently maintained FHub service boundary.

## Audit scope

### API surface

- Review user-facing API messages.
- Replace generic legacy wording with FHub-native operational language.
- Keep responses stable for the frontend while improving naming and clarity.

### Service boundaries

- Review account handling services.
- Review task orchestration services.
- Review storage and runtime config boundaries.
- Review telemetry and status messages.

### Runtime identity

- Ensure logs, labels, health checks, and default runtime messages use FHub naming.
- Keep Docker/runtime compatibility intact.

### Legal readiness

Backend work must be considered part of the substantial internal rewrite before attribution can be removed from legal files.

## Backend clear checklist

- [x] Backend audit plan created
- [x] Backend string target groups documented
- [x] API response wording audit started through `backend/src/api/media.rs`
- [x] Exception/error message audit started through `backend/src/error.rs`
- [x] Service naming audit started through `backend/src/services/tmdb_service.rs`
- [ ] Config naming audit
- [ ] Runtime log audit
- [ ] Health/status response audit
- [ ] Final license readiness review

## Completed backend-native patches

- `backend/src/error.rs`: FHub-native error display wording without changing enum names, HTTP status codes, or response keys.
- `backend/src/api/media.rs`: FHub-native media API wording while preserving endpoints and JSON schema.
- `backend/src/api/search.rs`: FHub-native search version labels and comments while preserving scoring and enrichment behavior.
- `backend/src/api/tmdb.rs`: FHub-native metadata proxy wording while preserving routes and upstream provider behavior.
- `backend/src/services/tmdb_service.rs`: FHub-native metadata service wording and logs while preserving public struct/function names.

## Search strategy

Use focused queries instead of broad backend searches because code search can miss nested or generated routes.

Target keyword groups:

- `raise HTTPException`
- `detail=`
- `logger.`
- `health`
- `status`
- `account`
- `batch`
- `queue`
- `download`
- `settings`

## Current assessment

- Frontend native surface: approximately 83%+
- Backend native audit: approximately 65% of identified lightweight-safe patches completed
- Overall internal rewrite: approximately 91%+
- Legal clear readiness: not final until backend audit, source ingest rewrite, route integration, and final license review are complete

## Next backend sequence

1. Continue with small files first where full content can be fetched without truncation.
2. For large files such as orchestrator and smart search, avoid full-file replacement until a patch-safe workflow is available.
3. Replace legacy/generic messages with FHub-native messages in small commits.
4. Avoid changing endpoint contracts unless necessary.
5. Run or inspect CI/build once route integration work is complete.
