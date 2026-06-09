#!/bin/bash
# Fix staging data: update old batch progress and sync How Dare You to FHub
# Run from local machine (connects to pve-remote -> LXC 112 -> fhub container)

set -e

echo "=== Fixing Staging Data ==="

# 1. Fix old batch progress: set downloaded=size for COMPLETED tasks where downloaded is 0
echo ""
echo "[1/3] Fixing batch progress for completed tasks..."
ssh root@pve-remote 'pct exec 112 -- docker exec fhub /bin/sh -c "
  # Install sqlite3 temporarily
  apk add --no-cache sqlite >/dev/null 2>&1 || true
  
  echo \"Before fix:\"
  sqlite3 /config/fhub.db \"SELECT filename, state, size, downloaded FROM downloads WHERE state='COMPLETED' AND (downloaded=0 OR downloaded IS NULL) LIMIT 10\"
  
  # Fix: set downloaded=size for completed tasks with 0/NULL downloaded
  sqlite3 /config/fhub.db \"UPDATE downloads SET downloaded=size, progress=100.0 WHERE state='COMPLETED' AND (downloaded=0 OR downloaded IS NULL)\"
  
  echo \"\"
  echo \"Fixed rows: \$(sqlite3 /config/fhub.db \"SELECT changes()\")\"
  
  echo \"\"
  echo \"Batch progress after fix:\"
  sqlite3 /config/fhub.db \"SELECT batch_name, SUM(downloaded) as dl, SUM(size) as total, ROUND(CAST(SUM(downloaded) AS FLOAT)/CAST(SUM(size) AS FLOAT)*100, 1) as progress FROM downloads WHERE batch_id IS NOT NULL GROUP BY batch_id, batch_name\"
"'

# 2. Create hardlinks for "How Dare You" episodes to FHub-Import
echo ""
echo "[2/3] Creating hardlinks for 'How Dare You' episodes..."
ssh root@pve-remote 'pct exec 112 -- docker exec fhub /bin/sh -c "
  # Create FHub-Import directory structure
  mkdir -p \"/downloads/FHub-Import/How Dare You/Season 01\"
  
  # Create hardlinks for each episode
  for f in \"/downloads/How Dare You!_/Season 01/\"*.mkv; do
    if [ -f \"\$f\" ]; then
      basename=\$(basename \"\$f\")
      # Clean filename: remove ? from name for FHub compatibility
      clean_name=\$(echo \"\$basename\" | sed \"s/!?/!/g\")
      target=\"/downloads/FHub-Import/How Dare You/Season 01/\$clean_name\"
      if [ ! -f \"\$target\" ]; then
        ln \"\$f\" \"\$target\" && echo \"  Hardlinked: \$clean_name\" || echo \"  FAILED: \$clean_name\"
      else
        echo \"  Already exists: \$clean_name\"
      fi
    fi
  done
  
  echo \"\"
  echo \"Hardlink verification:\"
  ls -la \"/downloads/FHub-Import/How Dare You/Season 01/\"
"'

# 3. Trigger FHub rescan for the series
echo ""
echo "[3/3] Triggering FHub rescan..."
ssh root@pve-remote 'pct exec 112 -- docker exec fhub /bin/sh -c "
  apk add --no-cache curl >/dev/null 2>&1 || true
  
  # Get FHub connection info from the app
  SONFHUB_URL=\$(sqlite3 /config/fhub.db \"SELECT value FROM settings WHERE key='\''fhub_url'\''\" 2>/dev/null)
  SONFHUB_KEY=\$(sqlite3 /config/fhub.db \"SELECT value FROM settings WHERE key='\''fhub_api_key'\''\" 2>/dev/null)
  
  if [ -z \"\$SONFHUB_URL\" ] || [ -z \"\$SONFHUB_KEY\" ]; then
    echo \"Could not find FHub connection info in settings\"
    echo \"URL: \$SONFHUB_URL\"
    echo \"Key: \${SONFHUB_KEY:0:5}...\"
    exit 1
  fi
  
  # Strip quotes if present
  SONFHUB_URL=\$(echo \"\$SONFHUB_URL\" | tr -d '\"')
  SONFHUB_KEY=\$(echo \"\$SONFHUB_KEY\" | tr -d '\"')
  
  echo \"FHub URL: \$SONFHUB_URL\"
  
  # Trigger DownloadedEpisodesScan command for series ID 60 (How Dare You)
  echo \"Sending RescanSeries command for series ID 60...\"
  RESULT=\$(curl -s -X POST \"\${SONFHUB_URL}/api/v3/command\" \
    -H \"X-Api-Key: \${SONFHUB_KEY}\" \
    -H \"Content-Type: application/json\" \
    -d '{\"name\":\"RescanSeries\",\"seriesId\":60}')
  echo \"FHub response: \$RESULT\"
  
  echo \"\"
  echo \"Sending DownloadedEpisodesScan command...\"
  RESULT=\$(curl -s -X POST \"\${SONFHUB_URL}/api/v3/command\" \
    -H \"X-Api-Key: \${SONFHUB_KEY}\" \
    -H \"Content-Type: application/json\" \
    -d \"{\\\"name\\\":\\\"DownloadedEpisodesScan\\\",\\\"path\\\":\\\"/downloads/FHub-Import/How Dare You/Season 01\\\"}\")
  echo \"FHub response: \$RESULT\"
"'

echo ""
echo "=== Done ==="
