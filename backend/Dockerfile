# One Dockerfile builds any of the four services — pass `--build-arg
# SERVICE=identity|commerce|notifications|media`. Building from the
# workspace root (context = backend/) so the shared crate and every
# service's Cargo.lock entry are visible to the build.
#
#   docker build -f Dockerfile --build-arg SERVICE=identity -t stocklink/identity:dev .

ARG SERVICE

FROM rust:1-slim-bookworm AS builder
ARG SERVICE
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin stocklink-${SERVICE}

FROM debian:bookworm-slim AS runtime
ARG SERVICE
# Baked into the final image as a fixed env var — the entrypoint below is
# the only thing that needs to know which binary this image runs.
ENV SERVICE_BIN=stocklink-${SERVICE}

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd --no-create-home --shell /bin/false stocklink

COPY --from=builder /build/target/release/stocklink-${SERVICE} /app/service
# Each service's own migrations, embedded at compile time via `sqlx::migrate!`
# already — copied here too only so `/app/migrations` is inspectable in the
# running container; the binary doesn't read this directory at runtime.
COPY crates/${SERVICE}/migrations /app/migrations

USER stocklink
WORKDIR /app
ENTRYPOINT ["/app/service"]
