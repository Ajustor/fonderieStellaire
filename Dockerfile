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
RUN for c in domain application infrastructure presentation; do \
        mkdir -p crates/$c/src && echo "" > crates/$c/src/lib.rs; \
    done \
    && mkdir -p crates/bin/src && echo "fn main() {}" > crates/bin/src/main.rs \
    && cargo build --release --locked \
    && rm -rf crates/*/src
COPY crates crates
# Force la recompilation de nos crates (les mtimes des sources factices sont plus récents).
RUN touch crates/*/src/*.rs && cargo build --release --locked

FROM debian:bookworm-slim
LABEL org.opencontainers.image.title="Fonderie Stellaire" \
      org.opencontainers.image.description="Un idle game cosmique dans ton terminal" \
      org.opencontainers.image.licenses="MIT"
RUN useradd --create-home --uid 1000 joueur \
    && mkdir -p /data && chown joueur:joueur /data
COPY --from=build /src/target/release/fonderie-stellaire /usr/local/bin/fonderie-stellaire
USER joueur
# dirs::data_dir() suit XDG_DATA_HOME : la sauvegarde va dans /data/fonderie-stellaire/.
# Pas de mise à jour automatique dans un conteneur : on met à jour l'image.
ENV FONDERIE_NO_UPDATE=1 \
    XDG_DATA_HOME=/data \
    COLORTERM=truecolor \
    TERM=xterm-256color \
    LANG=C.UTF-8
VOLUME ["/data"]
ENTRYPOINT ["fonderie-stellaire"]
