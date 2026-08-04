FROM rust:1.97.1-alpine3.24 AS builder

WORKDIR /build

COPY Cargo.lock Cargo.toml ./
COPY .sqlx .sqlx
COPY config config
COPY src src

ARG SQLX_OFFLINE=true
RUN --mount=type=cache,target=/build/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked && \
    cp "target/release/app" app


FROM scratch AS final

WORKDIR /app
USER 10001:10001

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder --chown=10001:10001 /build/app ./ztpir
COPY --chown=10001:10001 config config

ENV ZTPIR_ENVIRONMENT="production"
ENTRYPOINT ["/app/ztpir"]
