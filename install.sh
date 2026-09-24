#!/usr/bin/env sh
set -eu
repo=talkincode/FilmMap
version=${FILMMAP_VERSION:-latest}
os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)
case "$arch" in x86_64|amd64) arch=amd64;; aarch64|arm64) arch=arm64;; *) echo "unsupported architecture: $arch" >&2; exit 1;; esac
case "$os" in darwin) target=darwin;; linux) target=linux;; *) echo "unsupported OS: $os" >&2; exit 1;; esac
if [ "$version" = latest ]; then tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1); else tag=$version; fi
[ -n "$tag" ] || { echo "could not determine release version" >&2; exit 1; }
base="https://github.com/$repo/releases/download/$tag"
name="filmmap-$target-$arch.tar.gz"
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT HUP INT TERM
curl -fsSL "$base/$name" -o "$tmp/$name"
curl -fsSL "$base/checksums.txt" -o "$tmp/checksums.txt"
expected=$(awk -v f="$name" '$2==f {print $1}' "$tmp/checksums.txt")
[ -n "$expected" ] || { echo "checksum missing for $name" >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$name" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$tmp/$name" | awk '{print $1}')
else
  echo "sha256sum or shasum is required to verify the download" >&2
  exit 1
fi
[ "$actual" = "$expected" ] || { echo "checksum mismatch" >&2; exit 1; }
tar -xzf "$tmp/$name" -C "$tmp"
install_dir=${FILMMAP_INSTALL_DIR:-"$HOME/.local/bin"}
mkdir -p "$install_dir"
install "$tmp/filmmap" "$install_dir/filmmap"
for tool in "$tmp"/filmmap-*.sh; do
  [ -f "$tool" ] || continue
  install "$tool" "$install_dir/$(basename "$tool")"
done
printf 'Installed to %s/filmmap\n' "$install_dir"
