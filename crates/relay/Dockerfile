# syntax=docker/dockerfile:1
# Relais de synchronisation des parties (Dokploy ou tout hôte Docker).
# Construire depuis la racine du dépôt :
#   docker build -f crates/relay/Dockerfile -t fonderie-relay .
#   docker run -d -p 8080:8080 -v fonderie-relay:/data fonderie-relay

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --locked -p fonderie-relay

FROM debian:bookworm-slim
LABEL org.opencontainers.image.title="Fonderie Stellaire — relais" \
      org.opencontainers.image.description="Synchronisation chiffrée des parties entre appareils" \
      org.opencontainers.image.licenses="MIT"
RUN useradd --create-home --uid 1000 relais \
    && mkdir -p /data && chown relais:relais /data
COPY --from=build /src/target/release/fonderie-relay /usr/local/bin/fonderie-relay
USER relais
ENV PORT=8080 DATA_DIR=/data
EXPOSE 8080
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=3s CMD ["fonderie-relay", "--health"]
ENTRYPOINT ["fonderie-relay"]
