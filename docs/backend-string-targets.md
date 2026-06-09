# Backend string audit targets

This document tracks backend-facing text and runtime boundaries that must be reviewed before the FHub rewrite can be considered legal-clear ready.

## Target groups

### Health and status

- Health endpoint payloads
- Runtime status labels
- Connectivity state messages
- Scheduler state messages

### Account and session

- Account validation messages
- Session refresh messages
- Authentication failure text
- Credential storage warnings

### Work orchestration

- Task state names
- Queue/state transition messages
- Retry and failure descriptions
- Batch naming text

### Storage/runtime

- Download directory warnings
- Storage path validation messages
- File permission messages
- Runtime config errors

## Replacement rule

Replace generic or legacy-flavored wording with FHub-native operational wording while preserving API contracts and JSON keys.

## Examples

- `Failed to load batch items` -> `FHUB could not load this activity group`
- `Download not found` -> `FHUB activity was not found`
- `Invalid account` -> `FHUB could not verify this account`

## Completion criteria

- No public/runtime text points to legacy product identity.
- Backend exceptions use FHub wording where user-facing.
- Logs and health/status payloads are FHub-native where visible to operators.
- Endpoint response schemas remain backward compatible unless explicitly migrated.
