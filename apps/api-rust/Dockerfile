FROM rust:1.98-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --locked --release -p astra-api --bin astra-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home astra
COPY --from=builder /build/target/release/astra-api /usr/local/bin/astra-api
USER astra
ENV HOST=0.0.0.0 PORT=3334 NODE_ENV=production
EXPOSE 3334
HEALTHCHECK --interval=30s --timeout=12s --start-period=10s \
  CMD curl --fail --silent http://127.0.0.1:${PORT}/health | grep -q '"status":"ok"' || exit 1
ENTRYPOINT ["astra-api"]
