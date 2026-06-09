# FHub internal clear audit

## Current status

FHub public/product identity is already clean and FHub-native across UI, app metadata, Docker labels, package metadata, PWA manifest, README, NOTICE, and visible runtime copy.

## Remaining work for legal 100 percent clear

Legal 100 percent clear means the repository should not contain a substantial portion of upstream-derived internal implementation. This requires code-origin rewrite, not only rebranding.

## Areas to rewrite next

### Frontend

- Route shell implementation details and comments
- Dashboard component internals
- Server-side source ingest panel internals
- Settings/auth/setup component internals
- Store naming where derived from legacy structure
- Remaining inline CSS comments and class semantics

### Backend

- User-facing API error strings
- Service module naming and comments
- Download queue orchestration internals where legacy-derived
- Config/runtime docs embedded in code

### Legal files

- Keep upstream attribution only in LICENSE until the internal implementation has been substantially rewritten.
- After the internal rewrite is complete, replace LICENSE with FHub-only copyright text.

## Execution rule

Large rewrites should be split into small build-safe commits. Each commit should either:

1. Replace one component surface with FHub-native implementation, or
2. Rewrite one backend service/message group, or
3. Remove one class of legacy internal comments/naming.

## Percent estimate

- Public/product clear: 100 percent
- Runtime/metadata clear: 100 percent
- Internal implementation clear: 45 percent
- Legal-clear readiness: 65 to 70 percent

## Next commit target

Rewrite the FShare source ingest panel in smaller patch segments, then proceed to settings/auth internals.
