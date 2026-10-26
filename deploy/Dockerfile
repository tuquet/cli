# Multi-Stage Build for Tuquet Rust CLI Daemon
FROM rust:alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    ca-certificates \
    perl \
    make \
    cmake \
    git \
    curl \
    && update-ca-certificates

WORKDIR /usr/src/tuquet

# Copy workspace dependencies required by cli/Cargo.toml (browser & worker)
COPY browser ./browser
COPY worker ./worker
COPY cli ./cli

WORKDIR /usr/src/tuquet/cli

# Configure cargo to use git fetcher / disable strict revocation
RUN mkdir -p /root/.cargo && \
    echo '[http]' > /root/.cargo/config.toml && \
    echo 'check-revoke = false' >> /root/.cargo/config.toml && \
    echo '[net]' >> /root/.cargo/config.toml && \
    echo 'git-fetch-with-cli = true' >> /root/.cargo/config.toml

ENV CARGO_HTTP_CHECK_REVOKE=false
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV SWAGGER_UI_DOWNLOAD_URL="file:///usr/src/tuquet/cli/deploy/v5.17.14.zip"

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/tuquet/cli/target \
    cargo build --release --bin tq \
    && cp /usr/src/tuquet/cli/target/release/tq /usr/local/bin/tq

# Production Minimal Runtime Stage
FROM alpine:3.20 AS runtime

RUN apk add --no-cache \
    ca-certificates \
    libssl3 \
    curl \
    tzdata

# Create dedicated non-root user
RUN adduser -D -u 1000 tuquet \
    && mkdir -p /home/tuquet/.tq \
    && chown -R tuquet:tuquet /home/tuquet

COPY --from=builder /usr/local/bin/tq /usr/local/bin/tq

USER tuquet
WORKDIR /home/tuquet

ENV TQ_HOME="/home/tuquet/.tq"
ENV TQ_HOST="0.0.0.0"
ENV TQ_PORT="8765"

EXPOSE 8765

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:8765/api/v1/health || exit 1

ENTRYPOINT ["tq"]
CMD ["worker", "start", "-H", "0.0.0.0", "-p", "8765"]
