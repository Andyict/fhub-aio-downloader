# Synology Container Manager Quick Install

This is the easiest install path for non-SSH users.

## 1) Prepare folders in File Station

Create these folders:

- `/docker/fhub`
- `/docker/fhub/appData`
- optional media/download folder such as `/video`

On most Synology systems these become:

- `/volume1/docker/fhub`
- `/volume1/docker/fhub/appData`
- `/volume1/video`

> `/video` and `/volume1/video` are examples. Replace them with the real media/download folder on your NAS.

## 2) Create `docker-compose.yml`

Inside `/docker/fhub`, create a file named `docker-compose.yml` with this content:

```yaml
version: '3.8'

services:
  fhub:
    image: ghcr.io/nas2nd/fhub:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      # host_port:container_port
      # Open FHub at http://NAS_IP:8584
      - "8584:8484"

    volumes:
      # Persistent FHub data: config, database, accounts, settings
      - /volume1/docker/fhub/appData:/appData

      # Download folder. Change /volume1/video to your real NAS folder.
      - /volume1/video:/appData/downloads

    environment:
      # Timezone
      - TZ=Asia/Ho_Chi_Minh

      # App data location inside container
      - FHUB_APPDATA_DIR=/appData

      # Normal log level
      - RUST_LOG=fhub=info,tower_http=info

    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8484/api/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
```

## 3) Understand the YAML

### Port mapping

```yaml
ports:
  - "8584:8484"
```

This means:

```text
NAS/browser port : FHub container port
```

- Open app at `http://NAS_IP:8584`.
- Keep the right side as `8484` because that is the FHub internal port.
- If `8584` is already used, change only the left side, for example:

```yaml
ports:
  - "8585:8484"
```

Then open `http://NAS_IP:8585`.

### App data volume

```yaml
- /volume1/docker/fhub/appData:/appData
```

This keeps FHub data outside the container.

It stores:

- database
- settings
- account configuration
- other persistent app files

Do not delete this folder unless you want to reset FHub.

### Download folder volume

```yaml
- /volume1/video:/appData/downloads
```

This maps your NAS folder into FHub.

- Left side: your real NAS folder.
- Right side: the path FHub sees inside the container.

Examples:

```yaml
- /volume1/downloads:/appData/downloads
- /volume1/video/FHub:/appData/downloads
- /volume2/media/downloads:/appData/downloads
```

### Environment variables

```yaml
environment:
  - TZ=Asia/Ho_Chi_Minh
  - FHUB_APPDATA_DIR=/appData
  - RUST_LOG=fhub=info,tower_http=info
```

- `TZ`: timezone.
- `FHUB_APPDATA_DIR`: where FHub stores app data inside the container.
- `RUST_LOG`: log detail level. Use the default unless debugging.

### Healthcheck

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8484/api/health"]
```

Container Manager uses this to know if FHub is running correctly.

## 4) Deploy in Container Manager

- Open **Container Manager**.
- Go to **Project**.
- Click **Create**.
- Choose **Create project from existing docker-compose.yml**.
- Select `/docker/fhub/docker-compose.yml`.
- Name it `fhub`.
- Deploy.

## 5) Open the app

Open:

```text
http://NAS_IP:8584
```

If you changed the host port, use that port instead.

Example:

```text
http://NAS_IP:8585
```

## 6) First-time setup

- If this is a fresh install with no users yet, create the initial admin account first.
- Sign in with that admin account.
- Enter your own FShare VIP account.
- Choose the download path.
- Save settings.

## Notes

- `appData` stores config and database.
- `/appData/downloads` is where downloads appear inside the container.
- For Synology users, this image-based install is preferred over local build because it avoids SSH and compile steps.
- Newer images try to auto-fix common NAS permission issues on startup. If Synology folder permissions are still restrictive, the container will fall back to root instead of restart-looping on first boot.
