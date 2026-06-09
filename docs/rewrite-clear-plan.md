# FHub rewrite clear plan

This plan tracks the work required to make FHub an independently authored product surface while preserving legal safety during the transition.

## Goal

Move FHub from a rebranded fork surface to a FHub-native codebase and product identity.

## Rule

Do not remove required third-party license attribution until the affected code has been replaced or substantially rewritten.

## Rewrite scope

### Public product surface

- [x] App shell
- [x] Sidebar and navigation
- [x] Header and status HUD
- [x] Auth/login/setup screens
- [x] Settings/profile screens
- [x] Download dashboard
- [x] FShare server panel
- [x] Empty states and toast copy
- [x] PWA/app install metadata
- [x] Demo/preview pages

### Code structure

- [x] Frontend package identity
- [x] FHub design-system layer
- [x] Documentation and deployment examples
- [x] Public metadata and container labels
- [ ] Deep internal module rewrite where needed

### Backend surface

- [x] Public runtime metadata
- [x] Docker/runtime naming
- [ ] Deep backend service rewrite where needed
- [ ] Final logs/errors wording sweep

## Completed

- FHub cinematic theme layer
- FHub app metadata
- PWA manifest polish
- Docker labels and README namespace cleanup
- Package/package-lock identity
- NOTICE cleanup
- License text fixed and legally preserved
- FHub logo and cinematic brand surface
- App shell, auth/setup, settings, dashboard, and FShare panel public surface polish

## Remaining for legal 100% removal

The public product surface is FHub-native. To remove all upstream attribution safely, complete a code-origin audit and rewrite any substantial internal logic that remains derived from upstream.

## Final pass checklist

1. Keep required attribution only in legal files until internal rewrite is complete.
2. Do not expose upstream naming in product UI, docs, container labels, package metadata, PWA metadata, or runtime user-facing copy.
3. Track remaining work as internal architecture cleanup rather than public branding cleanup.
