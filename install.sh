#!/bin/sh
# Installe la dernière version de Fonderie Stellaire.
#   curl -fsSL https://raw.githubusercontent.com/Ajustor/fonderieStellaire/master/install.sh | sh
# Variables : INSTALL_DIR (défaut ~/.local/bin), VERSION (défaut : dernière, ex. v0.1.0).
set -eu

REPO="Ajustor/fonderieStellaire"
BIN="fonderie-stellaire"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"

say() { printf '\033[38;2;255;200;80m✦\033[0m %s\n' "$1"; }
die() { printf '\033[31merreur :\033[0m %s\n' "$1" >&2; exit 1; }

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64 | Linux/amd64) target="x86_64-unknown-linux-gnu" ;;
  Darwin/x86_64) target="x86_64-apple-darwin" ;;
  Darwin/arm64 | Darwin/aarch64) target="aarch64-apple-darwin" ;;
  *) die "système non pris en charge ($os/$arch). Compile depuis les sources : https://github.com/$REPO" ;;
esac

asset="$BIN-$target.tar.gz"
if [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi

command -v curl >/dev/null 2>&1 || die "curl est requis"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "Téléchargement de $asset…"
curl -fsSL "$base/$asset" -o "$tmp/$asset" || die "téléchargement impossible ($base/$asset)"
curl -fsSL "$base/$asset.sha256" -o "$tmp/$asset.sha256" || die "somme de contrôle introuvable"

say "Vérification de l'intégrité…"
expected="$(cut -d ' ' -f 1 "$tmp/$asset.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$asset" | cut -d ' ' -f 1)"
else
  actual="$(shasum -a 256 "$tmp/$asset" | cut -d ' ' -f 1)"
fi
[ "$expected" = "$actual" ] || die "somme SHA-256 incorrecte"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"
say "Installé dans $INSTALL_DIR/$BIN"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) say "Lance le jeu avec : $BIN" ;;
  *)
    say "$INSTALL_DIR n'est pas dans ton PATH. Ajoute à ton ~/.bashrc ou ~/.zshrc :"
    printf '    export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    say "En attendant, lance : $INSTALL_DIR/$BIN"
    ;;
esac
