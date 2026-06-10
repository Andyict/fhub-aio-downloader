# FHub

English | [Tiếng Việt](README.vi.md)

FHub is a NAS-focused FShare download manager with a web dashboard, segmented downloads, FShare account support, and media-friendly download organization.

## Features

- FShare account/download management
- High-speed segmented downloads
- Web dashboard for NAS/mobile use
- Multi-account support
- Docker / Docker Compose deployment
- Configurable download path and performance settings

## Quick install with Docker Compose

This method pulls a prebuilt public image from GHCR, so users only need Docker and Compose.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
mkdir -p ./appData
mkdir -p /volume1/Video
# Edit docker-compose.yml first if your download folder is not /volume1/Video
docker compose up -d
```

Open FHub:

```text
http://NAS_IP:8584
```

Examples:

```text
http://192.168.1.10:8584
http://localhost:8584
```

## Important path notes

FHub uses two main container paths:

| Purpose | Host path example | Container path | Notes |
| --- | --- | --- | --- |
| App data | Docker named volume `fhub_appdata` or `/volume1/docker/fhub/appData` | `/appData` | Stores database, config, accounts, settings. Back this up. |
| Downloads/media | `/volume1/Video` | `/downloads` | Where downloaded files are saved. Change the host side to your real NAS folder. |

In Docker Compose volume syntax:

```yaml
- HOST_PATH:CONTAINER_PATH
```

Only change the **left side** unless you know what you are doing.

Recommended:

```yaml
volumes:
  # Persistent FHub data. Keep the container path /appData.
  - fhub_appdata:/appData

  # Download/media folder on your NAS. Change only the left side.
  # Synology example: /volume1/Video
  # QNAP example:     /share/Multimedia
  # Linux example:    /mnt/media
  - /volume1/Video:/downloads
```

If your NAS folder is different, for example `/volume2/Movies`, use:

```yaml
- /volume2/Movies:/downloads
```

Do **not** change `/downloads` unless you also change `FHUB_DOWNLOADS_DIR`.

## Included `docker-compose.yml`

```yaml
version: '3.8'

services:
  fhub:
    image: ghcr.io/andyict/fhub-aio-downloader:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      # HOST_PORT:CONTAINER_PORT
      # Open FHub at http://NAS_IP:8584
      # If 8584 is already used, change only the left side, e.g. "8585:8484".
      - "8584:8484"

    volumes:
      # Persistent app data: database, config, accounts, settings.
      # Named volume is easiest for Docker users.
      - fhub_appdata:/appData

      # Downloads/media folder on the host NAS.
      # IMPORTANT: change /volume1/Video to your real folder if needed.
      # Keep the container path /downloads.
      - /volume1/Video:/downloads

    environment:
      # Timezone used by logs/scheduler.
      - TZ=Asia/Ho_Chi_Minh

      # Internal container paths. Keep these aligned with the volume mappings above.
      - FHUB_APPDATA_DIR=/appData
      - FHUB_DOWNLOADS_DIR=/downloads

      # Public-safe download defaults.
      # Stable for small/older NAS devices. Stronger NAS can tune these upward later.
      - FHUB_SEGMENTS_PER_DOWNLOAD=16
      - FHUB_MAX_CONCURRENT=4

      # Logging.
      - RUST_LOG=fhub=info,tower_http=info

    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8484/api/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s

volumes:
  fhub_appdata:
```

## Clean test compose

If you want a fresh install without old app data, use:

```yaml
# docker-compose.test.yml
version: '3.8'

services:
  fhub:
    image: ghcr.io/andyict/fhub-aio-downloader:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      - "8584:8484"

    volumes:
      - fhub_appdata_test:/appData
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
  fhub_appdata_test:
```

Run it with:

```bash
docker compose -f docker-compose.test.yml up -d
```

## First-time setup

1. Open `http://NAS_IP:8584`.
2. Create the first admin account if this is a fresh install.
3. Sign in.
4. Add your own FShare account.
5. Confirm the download path is `/downloads` inside FHub.
6. Start a small test download first.

## Updating

```bash
cd fhub-aio-downloader
git pull
docker compose pull
docker compose up -d
```

## Download performance tuning

Default public settings are conservative:

```yaml
FHUB_SEGMENTS_PER_DOWNLOAD=16
FHUB_MAX_CONCURRENT=4
```

These are chosen to avoid overloading weaker NAS devices or FShare sessions.

Suggested presets:

| Preset | Segments | Concurrent downloads | Notes |
| --- | ---: | ---: | --- |
| Stable | 8–12 | 2 | Old/small NAS, safer network use |
| Balanced | 16 | 3–4 | Recommended public default |
| Fast | 32 | 6 | Stronger NAS/network |
| Turbo | 48–64 | 8–10 | Use carefully; may trigger FShare/session/network limits |

If downloads fail, stall, or the NAS becomes slow, reduce both values.

## Synology notes

For Synology DSM:

```bash
mkdir -p /volume1/docker/fhub/appData
mkdir -p /volume1/Video
```

Then either use the named volume from the default compose, or replace app data with a visible host folder:

```yaml
volumes:
  - /volume1/docker/fhub/appData:/appData
  - /volume1/Video:/downloads
```

Make sure the Docker container has write permission to the host download folder.

## Health check

After starting:

```bash
curl http://NAS_IP:8584/api/health
```

Expected:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```

## Source notice

FHub is distributed under the MIT license. See `LICENSE` and `NOTICE.md`.
