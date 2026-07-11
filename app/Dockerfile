FROM rust:1.96.1-alpine3.24 AS builder

WORKDIR /build

RUN apk add mise lld clang mold musl-dev

ARG TARGET="aarch64-unknown-linux-musl"
RUN rustup target add "${TARGET}"

COPY Cargo.lock Cargo.toml config.yaml ./
COPY .sqlx .sqlx
COPY src src

ARG SQLX_OFFLINE=true
RUN --mount=type=cache,target=/build/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked --target "${TARGET}" && \
    cp "/build/target/${TARGET}/release/app" app


FROM scratch AS final

WORKDIR /app
COPY --from=builder /build/app ./ztpir
COPY config.yaml config.yaml

ENTRYPOINT ["/app/ztpir"]
