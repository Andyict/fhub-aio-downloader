# FHub Installation Guide

FHub is a NAS-focused FShare download manager.

## Docker Compose

```yaml
services:
  fhub:
    image: ghcr.io/nas2nd/fhub:latest
    container_name: fhub
    ports:
      - "8484:8484"
    volumes:
      - ./appData:/appData
      - ./downloads:/downloads
    restart: unless-stopped
```

## Start

```bash
docker compose up -d
```

Open `http://YOUR_SERVER_IP:8484`, create the admin account, then add your FShare account in the web UI.

## Data paths

- Config/data: `./appData`
- Downloads: `./downloads`

## Notes

Do not commit real FShare credentials, app data, logs, or downloads to git.
