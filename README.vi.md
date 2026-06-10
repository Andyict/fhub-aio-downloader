# FHub

[English](README.md) | Tiếng Việt

FHub là trình quản lý tải FShare dành cho NAS, có giao diện web, hỗ trợ tải nhiều luồng, quản lý tài khoản FShare và lưu file tải về theo thư mục media trên NAS.

## Tính năng

- Quản lý tài khoản/link tải FShare
- Tải tốc độ cao nhiều segment
- Giao diện web dùng tốt trên NAS/điện thoại
- Hỗ trợ nhiều tài khoản
- Cài đặt bằng Docker / Docker Compose
- Có thể cấu hình thư mục lưu file tải về và hiệu năng tải

## Cài nhanh bằng Docker Compose

Cách này dùng image public đã build sẵn từ GHCR. Người dùng chỉ cần Docker và Docker Compose, không cần tự build source.

```bash
git clone https://github.com/Andyict/fhub-aio-downloader.git
cd fhub-aio-downloader
mkdir -p ./appData
mkdir -p /volume1/Video
# Nếu thư mục lưu file của bạn không phải /volume1/Video, hãy sửa docker-compose.yml trước
docker compose up -d
```

Mở FHub tại:

```text
http://IP_NAS:8584
```

Ví dụ:

```text
http://192.168.1.10:8584
http://localhost:8584
```

## Lưu ý quan trọng về đường dẫn

FHub dùng 2 đường dẫn chính trong container:

| Mục đích | Ví dụ đường dẫn trên máy/NAS | Đường dẫn trong container | Ghi chú |
| --- | --- | --- | --- |
| Dữ liệu app | Docker named volume `fhub_appdata` hoặc `/volume1/docker/fhub/appData` | `/appData` | Lưu database, cấu hình, tài khoản, settings. Nên backup. |
| File tải về/media | `/volume1/Video` | `/downloads` | Nơi lưu file tải từ FShare. Chỉ sửa đường dẫn bên trái. |

Trong Docker Compose, cú pháp volume là:

```yaml
- DUONG_DAN_TREN_NAS:DUONG_DAN_TRONG_CONTAINER
```

Thông thường chỉ sửa **bên trái dấu `:`**.

Khuyến nghị:

```yaml
volumes:
  # Dữ liệu FHub. Giữ nguyên /appData.
  - fhub_appdata:/appData

  # Thư mục lưu file tải về trên NAS. Chỉ sửa bên trái.
  # Synology ví dụ: /volume1/Video
  # QNAP ví dụ:     /share/Multimedia
  # Linux ví dụ:    /mnt/media
  - /volume1/Video:/downloads
```

Nếu muốn lưu vào thư mục khác, ví dụ `/volume2/Movies`, sửa thành:

```yaml
- /volume2/Movies:/downloads
```

Không đổi `/downloads` trừ khi bạn cũng đổi biến `FHUB_DOWNLOADS_DIR`.

## File `docker-compose.yml` mẫu

```yaml
version: '3.8'

services:
  fhub:
    image: ghcr.io/andyict/fhub-aio-downloader:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      # HOST_PORT:CONTAINER_PORT
      # Mở FHub tại http://IP_NAS:8584
      # Nếu 8584 bị trùng, chỉ đổi bên trái, ví dụ "8585:8484".
      - "8584:8484"

    volumes:
      # Dữ liệu app: database, cấu hình, tài khoản, settings.
      - fhub_appdata:/appData

      # Thư mục lưu file tải về trên NAS.
      # QUAN TRỌNG: đổi /volume1/Video thành thư mục thật của bạn nếu cần.
      # Giữ nguyên /downloads.
      - /volume1/Video:/downloads

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
  fhub_appdata:
```

## Bản test sạch

Nếu muốn cài mới tinh, không giữ data cũ, dùng file:

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

Chạy:

```bash
docker compose -f docker-compose.test.yml up -d
```

Nếu đã chạy trước đó mà vào thẳng app không thấy màn tạo admin đầu tiên, nghĩa là còn dùng dữ liệu cũ. Hãy reset volume test:

```bash
docker compose -f docker-compose.test.yml down -v
```

rồi chạy lại:

```bash
docker compose -f docker-compose.test.yml up -d
```

## Thiết lập lần đầu

1. Mở `http://IP_NAS:8584`.
2. Tạo tài khoản admin đầu tiên nếu là lần cài mới.
3. Đăng nhập.
4. Thêm tài khoản FShare của bạn.
5. Kiểm tra thư mục tải trong FHub là `/downloads`.
6. Tải thử một file nhỏ trước.

## Cập nhật

```bash
cd fhub-aio-downloader
git pull
docker compose pull
docker compose up -d
```

## Tinh chỉnh tốc độ tải

Mặc định public đang để an toàn:

```yaml
FHUB_SEGMENTS_PER_DOWNLOAD=16
FHUB_MAX_CONCURRENT=4
```

Các mức gợi ý:

| Mức | Segments | Số lượt tải đồng thời | Ghi chú |
| --- | ---: | ---: | --- |
| Ổn định | 8–12 | 2 | NAS yếu/cũ, an toàn hơn |
| Cân bằng | 16 | 3–4 | Khuyến nghị mặc định |
| Nhanh | 32 | 6 | NAS/mạng khỏe hơn |
| Turbo | 48–64 | 8–10 | Dùng cẩn thận, có thể bị giới hạn session/mạng/FShare |

Nếu tải lỗi, bị đứng, hoặc NAS chậm, hãy giảm cả hai thông số trên.

## Ghi chú cho Synology

Tạo sẵn thư mục nếu cần:

```bash
mkdir -p /volume1/docker/fhub/appData
mkdir -p /volume1/Video
```

Nếu muốn lưu app data ra thư mục nhìn thấy trên NAS thay vì named volume:

```yaml
volumes:
  - /volume1/docker/fhub/appData:/appData
  - /volume1/Video:/downloads
```

Đảm bảo container Docker có quyền ghi vào thư mục tải về.

## Kiểm tra sức khỏe app

Sau khi chạy:

```bash
curl http://IP_NAS:8584/api/health
```

Kết quả mong đợi:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```

## Ghi chú source

FHub được phát hành theo giấy phép MIT. Xem `LICENSE` và `NOTICE.md`.
