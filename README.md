# FHub

English | [Tiếng Việt](README.vi.md)

FHub is a NAS-focused FShare download manager with a web dashboard, segmented downloads, FShare account support, and media-friendly download organization.

## Quick install with Docker Compose

This method uses a public prebuilt GHCR image. Users only need Docker and Docker Compose.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
mkdir -p /volume1/Video
# Edit docker-compose.yml first if your download folder is not /volume1/Video
docker compose up -d
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

In **Settings**, FHub checks GitHub for a newer image. When one is available, the web UI shows a compact update notice and copies the Docker update command for you.

FHub intentionally ships as a single-container app. It does not require a separate updater helper container for normal use.

### Option 2: Auto-update with Watchtower

Use `docker-compose.auto-update.yml` if you want FHub to check for new images automatically.

## Health check

```bash
curl http://NAS_IP:8584/api/health
```

Expected:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```
