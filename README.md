# FHub

English | [Tiếng Việt](README.vi.md)

FHub is a NAS-focused FShare download manager with a web dashboard, segmented downloads, FShare account support, and media-friendly download organization.

## Quick install with one `docker-compose.yml`

This method uses the public prebuilt GHCR image. Users only need Docker and Docker Compose; no source build is required.

```bash
mkdir -p fhub && cd fhub
nano docker-compose.yml
mkdir -p /volume1/Video
# Edit docker-compose.yml first if your download folder is not /volume1/Video
docker compose up -d
```

If your NAS uses the legacy Compose binary, replace the last command with:

```bash
docker-compose up -d
```

Open FHub:

```text
http://NAS_IP:8584
```

On first launch, create the first admin account.

## `docker-compose.yml`

```yaml
version: '3.8'

services:
  fhub:
    image: ghcr.io/andyict/fhub-aio:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      - "8584:8484"

    volumes:
      - fhub_appdata:/appData

      # Downloaded videos/files are saved here.
      # Change only /volume1/Video to your NAS folder; keep /downloads unchanged.
      - /volume1/Video:/downloads

      # Allows the in-app Update button to pull/recreate FHub.
      # No separate updater helper container is required.
      - /var/run/docker.sock:/var/run/docker.sock

    networks:
      - fhub_net

    environment:
      - TZ=Asia/Ho_Chi_Minh
      - FHUB_APPDATA_DIR=/appData
      - FHUB_DOWNLOADS_DIR=/downloads
      - FHUB_SEGMENTS_PER_DOWNLOAD=16
      - FHUB_MAX_CONCURRENT=4
      - RUST_LOG=fhub=info,tower_http=info
      - FHUB_CONTAINER_NAME=fhub
      - FHUB_UPDATE_IMAGE=ghcr.io/andyict/fhub-aio:latest

volumes:
  fhub_appdata:

networks:
  fhub_net:
    name: fhub_net
```

## Download path

Downloaded videos/files are saved here:

```yaml
- /volume1/Video:/downloads
```

Create this folder first if it does not exist. Change only `/volume1/Video` to your real NAS folder. Keep `/downloads` unchanged.

## Updating

### Option 1: Manual update

```bash
./update.sh
```

Or, if you update from source:

```bash
git pull
docker compose pull
docker compose up -d
```

### Web update notice

In **Settings**, FHub checks GitHub/GHCR for a newer image. When one is available, the web UI can update the running container.

FHub intentionally installs from a single `docker-compose.yml` service. You do **not** need to define a separate updater service in Compose. During a web update, FHub may create a short-lived helper container from the same FHub image so the main `fhub` container can be safely pulled/recreated and health-checked.

For web updates to work, mount Docker socket into `fhub` as shown in `docker-compose.yml`:

```yaml
- /var/run/docker.sock:/var/run/docker.sock
```

Also keep these environment variables aligned with your Compose service/container name:

```yaml
- FHUB_CONTAINER_NAME=fhub
- FHUB_UPDATE_IMAGE=ghcr.io/andyict/fhub-aio:latest
```

### Option 2: Auto-update with Watchtower

Use `docker-compose.auto-update.yml` if you want FHub to check for new images automatically.

## TV Auto Track

For FShare TV-series folders, FHub can save an Auto Track bookmark and watch for new episodes:

- Toggle Auto Track next to the Download button.
- Enabling Auto Track only saves the folder; it does not immediately download the currently selected files.
- To download selected files now, still press **Download** and confirm as usual.
- Configure the scan interval in **Settings → Auto Track**.
- On later scans, new files are queued automatically while already-seen episodes are skipped.

## Health check

```bash
curl http://NAS_IP:8584/api/health
```

Expected:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```
