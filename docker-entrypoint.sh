#!/bin/sh
# FHub Container Entrypoint
# Tries to heal common Synology/NAS volume-permission issues on startup.
# If the mounted appData still is not writable by the app user, fall back to
# running as root so first-run setup can complete instead of restart-looping.

set -eu

APP_USER="fhub"
APP_GROUP="fhub"
APP_UID="911"
APP_GID="911"
APPDATA_DIR="${FHUB_APPDATA_DIR:-/appData}"
DOWNLOADS_DIR="${FHUB_DOWNLOADS_DIR:-${APPDATA_DIR}/downloads}"

mkdir -p "$APPDATA_DIR" \
         "$APPDATA_DIR/config" \
         "$APPDATA_DIR/data" \
         "$APPDATA_DIR/downloads" \
         "$APPDATA_DIR/logs" 2>/dev/null || true

# Best-effort ownership repair for common mounted paths.
chown -R "$APP_UID:$APP_GID" "$APPDATA_DIR" 2>/dev/null || true
chown -R "$APP_UID:$APP_GID" "$DOWNLOADS_DIR" 2>/dev/null || true
chown -R "$APP_UID:$APP_GID" /downloads 2>/dev/null || true

# Only chown the top-level media dirs (not recursively — media libraries can be huge)
if [ -d "/data/media" ]; then
    for dir in /data/media/*/; do
        chown "$APP_UID:$APP_GID" "$dir" 2>/dev/null || true
    done
fi

# Check whether the real app user can write appData. If yes, run unprivileged.
if gosu "$APP_USER:$APP_GROUP" sh -c "touch '$APPDATA_DIR/.fhub-write-test' && rm -f '$APPDATA_DIR/.fhub-write-test'" 2>/dev/null; then
    exec gosu "$APP_USER:$APP_GROUP" "$@"
fi

echo "[fhub-entrypoint] Warning: $APPDATA_DIR is not writable by $APP_USER after permission repair; continuing as root for compatibility." >&2
exec "$@"
