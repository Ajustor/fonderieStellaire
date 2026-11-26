# syntax=docker/dockerfile:1
# Image du jeu : docker run -it --rm -v fonderie:/data ghcr.io/ajustor/fonderiestellaire
# Le jeu est un TUI : il faut absolument -it (terminal interactif).

FROM rust:1-slim-bookworm AS build
WORKDIR /src
# Couche de dépendances mise en cache : on compile d'abord avec des sources vides.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/application/Cargo.toml crates/application/
COPY crates/infrastructure/Cargo.toml crates/infrastructure/
COPY crates/presentation/Cargo.toml crates/presentation/
COPY crates/bin/Cargo.toml crates/bin/
# Les autres membres du workspace (Android, bindgen, relais) ne sont pas
# compilés ici, mais Cargo a besoin de leurs manifestes.
COPY crates/android/Cargo.toml crates/android/
COPY crates/uniffi-bindgen/Cargo.toml crates/uniffi-bindgen/
COPY crates/relay/Cargo.toml crates/relay/
RUN for c in domain application infrastructure presentation android; do \
        mkdir -p crates/$c/src && echo "" > crates/$c/src/lib.rs; \
    done \
    && for c in bin uniffi-bindgen relay; do \
        mkdir -p crates/$c/src && echo "fn main() {}" > crates/$c/src/main.rs; \
    done \
    && cargo build --release --locked -p fonderie-stellaire \
    && rm -rf crates/*/src
COPY crates crates
# Autre relais de synchronisation (vide : relais officiel).
ARG FONDERIE_SYNC_URL=""
# Force la recompilation de nos crates (les mtimes des sources factices sont plus récents).
# La clé de chiffrement des sauvegardes arrive en secret BuildKit (vide en local).
RUN --mount=type=secret,id=save_key touch crates/*/src/*.rs     && FONDERIE_SAVE_KEY="$(cat /run/secrets/save_key 2>/dev/null || true)" FONDERIE_SYNC_URL="$FONDERIE_SYNC_URL" cargo build --release --locked -p fonderie-stellaire

FROM debian:bookworm-slim
LABEL org.opencontainers.image.title="Fonderie Stellaire" \
      org.opencontainers.image.description="Un idle game cosmique dans ton terminal" \
      org.opencontainers.image.licenses="MIT"
RUN useradd --create-home --uid 1000 joueur \
    && mkdir -p /data && chown joueur:joueur /data
COPY --from=build /src/target/release/fonderie-stellaire /usr/local/bin/fonderie-stellaire
USER joueur
# dirs::data_dir() suit XDG_DATA_HOME : la sauvegarde va dans /data/fonderie-stellaire/.
# Dans un conteneur, le jeu signale les nouvelles versions : on met à jour l'image.
ENV XDG_DATA_HOME=/data \
    COLORTERM=truecolor \
    TERM=xterm-256color \
    LANG=C.UTF-8
VOLUME ["/data"]
ENTRYPOINT ["fonderie-stellaire"]
