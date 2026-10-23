#!/usr/bin/env bash
# Build and run the Windows app from WSL. The source lives in WSL; this mirrors it to a Windows
# folder (native filesystem, fast builds) and drives cargo / the Tauri CLI through PowerShell.
#
#   scripts/win.sh sync      mirror the source to Windows
#   scripts/win.sh build     sync + debug build
#   scripts/win.sh run       sync + run the debug app against the WSL dev server (start `pnpm dev` first)
#   scripts/win.sh bundle    pnpm build (in WSL) + sync + release build with installers
#
# REFINE_WIN_DIR overrides the Windows folder (default: %USERPROFILE%\dev\refine).
set -euo pipefail
cd "$(dirname "$0")/.."

win_home=$(cmd.exe /c 'echo %USERPROFILE%' 2>/dev/null | tr -d '\r')
win_dir=${REFINE_WIN_DIR:-"$win_home\\dev\\refine"}
wsl_dir=$(wslpath -u "$win_dir")

ps() { (cd /mnt/c && powershell.exe -NoProfile -Command "\$ErrorActionPreference='Stop'; Set-Location '$win_dir'; $1"); }

sync() {
  mkdir -p "$wsl_dir"
  rsync -a --delete \
    --exclude target --exclude node_modules --exclude .git --exclude 'src-tauri/gen' \
    ./ "$wsl_dir/"
}

case "${1:-}" in
  sync) sync ;;
  build) sync; ps 'cargo build -p refine-app' ;;
  run) sync; ps 'cargo run -p refine-app' ;;
  bundle)
    pnpm build
    sync
    # The frontend is already built in WSL (Windows' Node may be too old for Vite).
    ps "npx --yes @tauri-apps/cli@2 build --config '{\"build\":{\"beforeBuildCommand\":\"\"}}'"
    echo "Installers: $win_dir\\target\\release\\bundle"
    ;;
  *) sed -n '2,10p' "$0"; exit 1 ;;
esac
