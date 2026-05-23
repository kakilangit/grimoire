FROM rust:1.95-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY sdk/ sdk/
COPY plugins/ollama/ plugins/ollama/
RUN cargo build --release --package grimoire-ollama

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/grimoire-ollama /plugin
COPY plugins/ollama/grimoire.json /grimoire.json
ENTRYPOINT ["/plugin"]
