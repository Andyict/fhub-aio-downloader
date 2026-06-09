# Stable Discovery Rollback Note

Date: 2026-05-13 UTC

## Stable 8484 Discovery build

Use this image as the known-good rollback target if Discovery breaks again:

```bash
fhub-test-8584:distinct-8484-discover-runesstate
```

Known-good container mapping:

```bash
container: fhub-test-8584
port: 0.0.0.0:8484->8484/tcp
volume: fhub_appdata:/appData
env: FHUB_APPDATA_DIR=/appData
```

8584 reference container must remain unchanged:

```bash
container: fhub-test-8584-alt
image: fhub-test-8584:logo-fix
port: 0.0.0.0:8584->8484/tcp
```

## Rollback command for 8484 only

```bash
docker rm -f fhub-test-8584 >/dev/null 2>&1 || true
docker run -d --name fhub-test-8584 --restart unless-stopped -p 8484:8484 \
  -e FHUB_APPDATA_DIR=/appData \
  -e RUST_LOG=fhub=info,tower_http=info \
  -v fhub_appdata:/appData \
  fhub-test-8584:distinct-8484-discover-runesstate
```

## Smoke test after rollback

```bash
docker exec fhub-test-8584 sh -lc 'curl -sS -i --max-time 12 "http://127.0.0.1:8484/api/discovery/popular-today?type=movie&page=1&limit=2" | sed -n "1,12p"'
```

Expected:

- `HTTP/1.1 200 OK`
- JSON body contains `results`
- JSON body contains `poster_url`

## Notes

- This is the version the user confirmed as working after Discovery came back.
- Do not deploy future clear/branding batches over 8484 without build + smoke test.
- Do not touch 8584; it is the reference/test image.
2026-05-13T01:54:47Z 8584 previous image: fhub-test-8584:logo-fix
