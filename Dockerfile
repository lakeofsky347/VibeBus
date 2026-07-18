# syntax=docker/dockerfile:1.7

FROM rust:1.97.1-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src

RUN cargo build --release --locked --bin vibebus

FROM debian:bookworm-slim AS runtime

ARG VIBEBUS_VERSION=0.10.0
LABEL org.opencontainers.image.title="VibeBus" \
      org.opencontainers.image.description="Local structured fact bus for independent Codex tasks" \
      org.opencontainers.image.version="${VIBEBUS_VERSION}" \
      org.opencontainers.image.source="https://github.com/lakeofsky347/VibeBus" \
      org.opencontainers.image.licenses="MIT"

RUN groupadd --gid 10001 vibebus \
    && useradd --uid 10001 --gid 10001 --create-home --shell /usr/sbin/nologin vibebus \
    && mkdir -p /data /workspace \
    && chown -R vibebus:vibebus /data /workspace

COPY --from=builder --chown=vibebus:vibebus /src/target/release/vibebus /usr/local/bin/vibebus

ENV VIBEBUS_DATA_HOME=/data
WORKDIR /workspace
VOLUME ["/data"]
USER 10001:10001

ENTRYPOINT ["/usr/local/bin/vibebus"]
CMD ["--help"]
