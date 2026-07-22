# syntax=docker/dockerfile:1.7

FROM --platform=linux/amd64 rust:1.97.1-bookworm@sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src

RUN cargo build --release --locked --bin vibebus

FROM --platform=linux/amd64 debian:bookworm-slim@sha256:63a496b5d3b99214b39f5ed70eb71a61e590a77979c79cbee4faf991f8c0783e AS runtime

ARG VIBEBUS_VERSION=0.10.0
ARG VIBEBUS_SOURCE_REVISION=unknown
LABEL org.opencontainers.image.title="VibeBus" \
      org.opencontainers.image.description="Local structured fact bus for independent Codex tasks" \
      org.opencontainers.image.version="${VIBEBUS_VERSION}" \
      org.opencontainers.image.revision="${VIBEBUS_SOURCE_REVISION}" \
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
