# fhub-fshare V1 local fork

Đây là bản V1 đã được chốt chạy tốt trên `8484`.

## Trạng thái phát hành nội bộ
- `8484` = bản final V1 đang dùng

## Mục tiêu bản fork
- giữ cơ chế resolve Fshare của FHub
- thay/tối ưu bước download cho Fshare để đạt tốc độ cao hơn bản gốc
- giữ hành vi fallback an toàn để app vẫn chạy ổn

## Điều đã chốt ở V1
- bản đang chạy production nội bộ là image local tag `fhub-aria2:main-safe`
- hướng custom multi-range đã được gia cố bằng verify part/final-size trước khi báo completed
- các chỉnh sửa liên quan progress/completed trong downloader đã được giữ lại trong source hiện tại

## Cài lại nhanh bằng Docker image local/exported image
Nếu đã có file image tar:

```bash
docker load < fhub-v1-final-2026-04-25.tar
```

Chạy lại:

```bash
docker run -d \
  --name fhub \
  --restart unless-stopped \
  --network fhub_default \
  -p 8484:8484 \
  -e TZ=Asia/Ho_Chi_Minh \
  -e RUST_LOG=info \
  -e FHUB_APPDATA_DIR=/appData \
  -v /volume1/video:/downloads:rw \
  -v /volume1/docker/fhub/data:/appData:rw \
  fhub-aria2:v1-final-2026-04-25
```

## Build lại từ source
Tại thư mục repo:

```bash
docker build -t fhub-aria2:v1-final-2026-04-25 .
```

## Gợi ý lưu trữ phát hành
Nên lưu đủ 3 lớp:
- source code + git commit/tag
- docker image đã tag rõ ràng
- file backup offline `.tar`
