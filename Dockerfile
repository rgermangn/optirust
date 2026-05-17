# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev gcc
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/src/target \
  RUSTFLAGS='-C target-feature=+crt-static' \
  cargo build --release --target x86_64-unknown-linux-musl && \
  cp target/x86_64-unknown-linux-musl/release/thinflux /usr/local/bin/thinflux

FROM scratch
LABEL org.opencontainers.image.title="ThinFlux"
LABEL org.opencontainers.image.description="High-performance assets optimizer CLI"
COPY --from=builder /usr/local/bin/thinflux /thinflux
ENTRYPOINT [ "/thinflux" ]
