# FHub

[English](README.md) | Tiếng Việt

FHub là trình quản lý tải FShare dành cho NAS, có giao diện web, hỗ trợ tải nhiều luồng, quản lý tài khoản FShare và lưu file tải về theo thư mục media trên NAS.

## Tính năng chính

- **Tải FShare tối ưu cho NAS**: tải nhiều luồng, chia segment/file, queue nền và theo dõi tốc độ.
- **Check link trước khi tải**: xem danh sách file trong folder FShare, chọn file cần tải rồi mới xác nhận.
- **3 chế độ tải**: **Phim lẻ**, **Phim bộ**, và **Auto Track** ngay trong khu vực Download mode.
- **Auto Track phim bộ**: theo dõi folder FShare, ghi nhận các tập hiện có làm baseline và chỉ tự tải tập mới xuất hiện sau này.
- **Quản lý tải xuống**: pause/resume, retry, trạng thái rõ ràng và chỉ báo hoàn tất khi file thật sự tồn tại trên ổ.
- **Cập nhật trong web**: Settings hiển thị bản mới, có popup xác nhận, popup đang update và tự làm mới sau 1 phút.
- **Cài một file Compose**: chỉ cần một `docker-compose.yml`; helper update được FHub tạo tạm khi cần, không phải khai báo service riêng.
- **Giao diện Việt/Anh**: các trạng thái chính được dịch theo ngôn ngữ web.

## Cài nhanh bằng một file `docker-compose.yml`

Cách này dùng image public đã build sẵn từ GHCR. Người dùng chỉ cần Docker và Docker Compose; không cần build source.

```bash
mkdir -p fhub && cd fhub
nano docker-compose.yml
mkdir -p /volume1/Video
# Nếu thư mục lưu file của bạn không phải /volume1/Video, hãy sửa docker-compose.yml trước
docker compose up -d
```

Nếu NAS của bạn dùng binary Compose cũ, thay lệnh cuối bằng:

```bash
docker-compose up -d
```

Mở FHub:

```text
http://IP_NAS:8584
```

Lần đầu mở FHub, bạn sẽ tạo tài khoản admin đầu tiên.

## File `docker-compose.yml` mẫu

```yaml
services:
  fhub:
    image: ghcr.io/andyict/fhub-aio:latest
    container_name: fhub
    restart: unless-stopped

    ports:
      - "8584:8484"

    volumes:
      - fhub_appdata:/appData

      # Video/file tải về sẽ được lưu ở đây.
      # Chỉ đổi /volume1/Video thành thư mục trên NAS của bạn; giữ nguyên /downloads.
      - /volume1/Video:/downloads

      # Cho phép nút Update trong web tự pull/recreate container FHub.
      # Không cần container updater riêng.
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

    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:8484/api/health"]
      interval: 30s
      timeout: 5s
      retries: 5
      start_period: 20s

volumes:
  fhub_appdata:

networks:
  fhub_net:
    name: fhub_net
```

## Lưu ý đường dẫn tải về

Dòng này là nơi lưu video/file tải từ FShare:

```yaml
- /volume1/Video:/downloads
```

Nếu thư mục này chưa có thì nên tạo trước. Nếu muốn đổi thư mục, chỉ sửa `/volume1/Video` thành thư mục thật trên NAS. Giữ nguyên `/downloads`.

## Cập nhật

### Cách 1: Cập nhật thủ công

```bash
./update.sh
```

Hoặc nếu bạn cài từ source repo:

```bash
git pull
docker compose pull
docker compose up -d
```

### Cập nhật bằng nút trong web

Trong **Settings**, FHub tự kiểm tra GitHub/GHCR. Nếu có commit/image mới, web sẽ hiện nút **Update**. Khi bấm Update, FHub sẽ mở popup xác nhận; sau khi xác nhận, popup chuyển sang trạng thái đang cập nhật và trang tự làm mới sau khoảng 1 phút.

FHub được cài bằng một service trong `docker-compose.yml`. Bạn **không cần** khai báo thêm service updater riêng. Khi cập nhật qua web, FHub có thể tự tạo một helper container tạm từ chính image FHub để pull/recreate container chính an toàn và healthcheck/rollback nếu lỗi.

Để nút Update trong web hoạt động, compose cần mount Docker socket trực tiếp vào container `fhub`:

```yaml
- /var/run/docker.sock:/var/run/docker.sock
```

Đồng thời giữ đúng tên container/image update:

```yaml
- FHUB_CONTAINER_NAME=fhub
- FHUB_UPDATE_IMAGE=ghcr.io/andyict/fhub-aio:latest
```

Sau khi cập nhật `docker-compose.yml`, chạy:

```bash
docker compose up -d
```

### Cách 2: Tự động cập nhật bằng Watchtower

Dùng `docker-compose.auto-update.yml` nếu bạn muốn FHub tự kiểm tra image mới và cập nhật định kỳ.

## Auto Track phim bộ

Với link folder FShare phim bộ, FHub có thể lưu Auto Track để theo dõi tập mới:

- Chọn tab **Auto Track** trong Download mode hoặc bật từ Discovery/Auto Track.
- Bật Auto Track chỉ lưu folder theo dõi, không tự tải ngay các file đang chọn/tập hiện có.
- Muốn tải ngay vẫn bấm **Download** và xác nhận như bình thường.
- Chu kỳ quét được chỉnh trong **Settings → Auto Track**.
- Khi folder có tập mới ở lần quét sau, FHub tự thêm tập mới vào queue và tránh tải trùng tập đã ghi nhận.

## Kiểm tra sức khỏe app

```bash
curl http://IP_NAS:8584/api/health
```

Kết quả mong đợi:

```json
{"status":"ok","service":"fhub","version":"1.0.0"}
```
