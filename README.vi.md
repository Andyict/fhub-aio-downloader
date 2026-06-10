# FHub

[English](README.md) | Tiếng Việt

FHub là trình quản lý tải FShare dành cho NAS, có giao diện web, hỗ trợ tải nhiều luồng, quản lý tài khoản FShare và lưu file tải về theo thư mục media trên NAS.

## Cài nhanh bằng Docker Compose

Cách này dùng image public đã build sẵn từ GHCR. Người dùng chỉ cần Docker và Docker Compose.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
docker compose up -d
```

Mở FHub:

```text
http://IP_NAS:8584
```

## File `docker-compose.yml` mẫu sạch

Bản này dùng volume app data riêng `fhub_appdata_clean`. Nếu là lần đầu hoặc đã reset volume, FHub sẽ hiện màn tạo tài khoản admin đầu tiên.

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
      # Dữ liệu app sạch cho lần chạy đầu tiên.
      # Reset bằng: docker compose down -v
      - fhub_appdata_clean:/appData

      # Thư mục lưu file tải về trên NAS. Chỉ sửa bên trái dấu ":" nếu cần.
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

## Reset để test mới tinh

Nếu vào thẳng app và không thấy màn tạo admin đầu tiên, nghĩa là volume app data cũ vẫn còn. Reset bằng:

```bash
docker compose down -v
docker compose up -d
```

Sau khi reset, mở lại:

```text
http://IP_NAS:8584
```

Lần đầu sẽ tạo tài khoản admin.

## Lưu ý đường dẫn tải về

Dòng này là nơi lưu file tải từ FShare:

```yaml
- /volume2/homes/vanthinh194/Phim:/downloads
```

Nếu muốn đổi thư mục, chỉ sửa bên trái dấu `:`. Giữ nguyên `/downloads`.

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
