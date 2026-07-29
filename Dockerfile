# syntax=docker/dockerfile:1.7

FROM rust:1.96.1-bookworm@sha256:a339861ae23e9abb272cea45dfafde21760d2ce6577a70f8a926153677902663 AS builder

ARG FERRITE_BUILD_SHA=unknown
WORKDIR /src

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    FERRITE_BUILD_SHA="$FERRITE_BUILD_SHA" \
    cargo build --release --locked -p ferrite-server --bin ferrite-server && \
    install -D -m 0755 target/release/ferrite-server /out/ferrite-server

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:66aa873a4a14fb164aa01296058efd8253744606d72715e45acface073359faa

ARG FERRITE_BUILD_SHA=unknown
ARG FERRITE_VERSION=dev

LABEL org.opencontainers.image.title="Ferrite" \
      org.opencontainers.image.description="CPU-native GGUF inference server" \
      org.opencontainers.image.source="https://github.com/vicotrbb/ferrite" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="$FERRITE_VERSION" \
      org.opencontainers.image.revision="$FERRITE_BUILD_SHA"

COPY --from=builder --chown=nonroot:nonroot /out/ferrite-server /usr/local/bin/ferrite-server

USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/ferrite-server"]
