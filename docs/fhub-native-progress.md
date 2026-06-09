# FHub native rewrite progress

Branch: `fhub-native-rewrite`

## Goal

Move the project from a public rebrand into an independently authored FHub-native implementation surface.

## Current clear status

- Public/product identity: 100%
- Runtime and metadata identity: 100%
- Frontend native surface coverage: approximately 80%
- Legal attribution removal: blocked until the remaining substantial internal implementation has been rewritten and reviewed

## Native frontend components added

- `FhubAuthShell.svelte`
- `FhubEmptyState.svelte`
- `FhubHeroPanel.svelte`
- `FhubMetricTile.svelte`
- `FhubNativeDashboard.svelte`
- `FhubPanel.svelte`
- `FhubSettingsShell.svelte`
- `FhubShellFrame.svelte`
- `FhubSourceIngest.svelte`
- `FhubStatusDeck.svelte`
- `FhubSystemPanel.svelte`

## Native surface coverage

- Dashboard shell: ready
- Dashboard lanes: using FHub panel primitives
- System panel: ready
- Settings shell: ready
- Auth/setup shell: ready
- Generic panel primitive: ready
- Metric tile primitive: ready
- Source ingest wrapper: ready for deeper rewrite

## Export status

FHub-native components are exported through `frontend/src/lib/components/fhub/index.ts`.

## Next implementation sequence

1. Route the dashboard through `FhubNativeDashboard`.
2. Route settings through `FhubSettingsShell`.
3. Route login/setup through `FhubAuthShell`.
4. Rewrite source ingest internals in smaller commits because this area is more likely to trigger connector filters.
5. Run backend wording and service-boundary audit.
6. After substantial internal rewrite, replace legal attribution with FHub-only copyright.

## Constraint

Large payloads are acceptable when they do not trigger connector safety filters. Sensitive source-ingest changes should be split into smaller commits.
