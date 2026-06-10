# FHub

[English](README.md) | Tiếng Việt

FHub là trình quản lý tải FShare dành cho NAS, có giao diện web, hỗ trợ tải nhiều luồng, quản lý tài khoản FShare và lưu file tải về theo thư mục media trên NAS.

## Cài nhanh bằng Docker Compose

Cách này dùng image public đã build sẵn từ GHCR. Người dùng chỉ cần Docker và Docker Compose.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
mkdir -p /volume1/Video
# Nếu thư mục lưu file của bạn không phải /volume1/Video, hãy sửa docker-compose.yml trước
docker compose up -d
```

Mở FHub:

```text
http://IP_NAS:8584
```

Lần đầu mở FHub, bạn sẽ tạo tài khoản admin đầu tiên.

## File `docker-compose.yml` mẫu

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
      - fhub_appdata:/appData

      # Video/file tải về sẽ được lưu ở đây.
      # Chỉ đổi /volume1/Video thành thư mục trên NAS của bạn; giữ nguyên /downloads.
      - /volume1/Video:/downloads

    environment:
      - TZ=Asia/Ho_Chi_Minh
      - FHUB_APPDATA_DIR=/appData
      - FHUB_DOWNLOADS_DIR=/downloads
      - FHUB_SEGMENTS_PER_DOWNLOAD=16
      - FHUB_MAX_CONCURRENT=4
      - RUST_LOG=fhub=info,tower_http=info

volumes:
  fhub_appdata:
```

## Lưu ý đường dẫn tải về

Dòng này là nơi lưu video/file tải từ FShare:

```yaml
- /volume1/Video:/downloads
```

Nếu thư mục này chưa có thì nên tạo trước. Nếu muốn đổi thư mục, chỉ sửa `/volume1/Video` thành thư mục thật trên NAS. Giữ nguyên `/downloads`.

## Cập nhật

```bash
git pull
docker compose pull
docker compose up -d
```

## Kiểm tra sức khỏe app

```bash
curl http://IP_NAS:8584/api/health
```

Kết quả mong đợi:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```
