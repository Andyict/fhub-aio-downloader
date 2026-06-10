# FHub

English | [Tiếng Việt](README.vi.md)

FHub is a NAS-focused FShare download manager with a web dashboard, segmented downloads, FShare account support, and media-friendly download organization.

## Quick install with Docker Compose

This method uses a public prebuilt GHCR image. Users only need Docker and Docker Compose.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
docker compose up -d
```

Open FHub:

```text
http://NAS_IP:8584
```

## Clean `docker-compose.yml`

This is the only recommended compose file. It uses a clean app data volume named `fhub_appdata_clean`. On a fresh volume, FHub should show the first admin account setup screen.

```yaml
version: '3.8'

services:
  fhub:
    image: ghcr.io/andyict/fhub-aio-downloader:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      - "8584:8484"

    volumes:
      # Clean app data volume for fresh first-run/admin setup.
      # Reset with: docker compose down -v
      - fhub_appdata_clean:/appData

      # Downloads/media folder on the NAS. Change only the left side if needed.
      - /volume2/homes/vanthinh194/Phim:/downloads

    environment:
      - TZ=Asia/Ho_Chi_Minh
      - FHUB_APPDATA_DIR=/appData
      - FHUB_DOWNLOADS_DIR=/downloads
      - FHUB_SEGMENTS_PER_DOWNLOAD=16
      - FHUB_MAX_CONCURRENT=4
      - RUST_LOG=fhub=info,tower_http=info

    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8484/api/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s

volumes:
  fhub_appdata_clean:
```

## Reset for a fresh test

If FHub opens straight into the app and does not show the first admin setup screen, old app data is still present. Reset it with:

```bash
docker compose down -v
docker compose up -d
```

Then open:

```text
http://NAS_IP:8584
```

## Download path

Downloaded files are saved here:

```yaml
- /volume2/homes/vanthinh194/Phim:/downloads
```

Change only the left side of `:` if needed. Keep `/downloads` unchanged.

## Updating

```bash
git pull
docker compose pull
docker compose up -d
```

## Health check

```bash
curl http://NAS_IP:8584/api/health
```

Expected:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```
