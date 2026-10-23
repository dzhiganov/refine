# Building the Tauri app on Linux without sudo

Tauri needs WebKitGTK and friends. Without root, unpack the packages into a local sysroot:

```bash
apt-get -s install --no-install-recommends libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev pkg-config libssl-dev librsvg2-dev libxdo-dev \
  | awk '/^Inst/ {print $2}' > pkgs.txt
apt-get download $(cat pkgs.txt)
SR=~/.local/share/refine-sysroot; mkdir -p $SR
for d in *.deb; do dpkg-deb -x "$d" $SR; done
# Re-point dangling .so symlinks at the system's runtime libraries:
cd $SR && find . -xtype l | while read l; do t=$(readlink "$l"); case "$t" in /*) a=$t;; *) a="/$(dirname "${l#./}")/$t";; esac; [ -e "$a" ] && ln -sf "$a" "$l"; done
```

Then `source scripts/linux-sysroot-env.sh` (sets `PKG_CONFIG_*`, `LIBRARY_PATH`,
`LD_LIBRARY_PATH`) before `cargo check -p refine-app`. With sudo, just install the packages instead.
