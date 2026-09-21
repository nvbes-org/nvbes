#!/usr/bin/env bash
# ==============================================================================
# nvbes Automated Cache Storage Manager & Guardrail
# Monitors and auto-prunes:
#   1. Remote GitHub Actions cache
#   2. Local Docker BuildKit / Container cache
#   3. Rust target/ directory artifacts
#   4. Nx local cache
#   5. pnpm store & temp files
# ==============================================================================

set -euo pipefail

DISK_MIN_FREE_GB=20
TARGET_MAX_GB=25

log() {
  echo -e "\033[1;34m[cache-manager]\033[0m $*"
}

warn() {
  echo -e "\033[1;33m[cache-manager:warn]\033[0m $*"
}

# --- 1. Disk Space Check ---
get_free_disk_gb() {
  df -g /System/Volumes/Data 2>/dev/null | awk 'NR==2 {print $4}' || df -k . | awk 'NR==2 {print int($4/1024/1024)}'
}

FREE_GB=$(get_free_disk_gb)
log "Available host disk space: ${FREE_GB} GB (minimum threshold: ${DISK_MIN_FREE_GB} GB)"

# --- 2. Rust target/ Directory Management ---
if [[ -d "target" ]]; then
  TARGET_SIZE_MB=$(du -sm target | awk '{print $1}')
  TARGET_SIZE_GB=$((TARGET_SIZE_MB / 1024))
  log "Current target/ directory size: ${TARGET_SIZE_GB} GB (${TARGET_SIZE_MB} MB)"

  if [[ "$TARGET_SIZE_GB" -gt "$TARGET_MAX_GB" || "$FREE_GB" -lt "$DISK_MIN_FREE_GB" ]]; then
    warn "target/ size (${TARGET_SIZE_GB} GB) exceeds limit or disk low (${FREE_GB} GB free). Auto-cleaning release debug artifacts..."
    find target -name "*.d" -type f -delete 2>/dev/null || true
    find target -name "*.rlib" -mtime +3 -type f -delete 2>/dev/null || true
    find target/release/deps -type f -mtime +3 -delete 2>/dev/null || true
    find target/debug/deps -type f -mtime +3 -delete 2>/dev/null || true
  fi
fi

# --- 3. Docker & BuildKit Cache Garbage Collection ---
if command -v docker >/dev/null 2>&1; then
  log "Enforcing Docker & BuildKit cache bounds (reserved-space: 15GB)..."
  docker image prune -f --filter "until=72h" >/dev/null 2>&1 || true
  docker builder prune -f --reserved-space 15GB >/dev/null 2>&1 || docker builder prune -f >/dev/null 2>&1 || true
fi

# --- 4. Nx Cache Pruning ---
if [[ -d ".nx/cache" ]]; then
  NX_SIZE_MB=$(du -sm .nx/cache 2>/dev/null | awk '{print $1}' || echo "0")
  if [[ "$NX_SIZE_MB" -gt 3000 ]]; then
    log "Nx cache is ${NX_SIZE_MB} MB (> 3GB), pruning old task outputs..."
    find .nx/cache -type f -mtime +3 -delete 2>/dev/null || true
  fi
fi

# --- 5. GitHub Actions Remote Cache Cleanup ---
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  log "Inspecting remote GitHub Actions cache..."
  CACHE_COUNT=$(gh cache list --limit 100 2>/dev/null | wc -l | tr -d ' ')
  if [[ "$CACHE_COUNT" -gt 0 ]]; then
    log "Found ${CACHE_COUNT} remote cache entries. Deleting obsolete caches..."
    gh cache delete --all 2>/dev/null || true
    log "Remote GitHub Actions cache cleaned."
  else
    log "Remote GitHub Actions cache is empty (0 bytes)."
  fi
fi

FINAL_FREE_GB=$(get_free_disk_gb)
log "Cache auto-management completed. Free space: ${FINAL_FREE_GB} GB."
