FROM rust:1.95-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY sdk/ sdk/
COPY plugins/guardrails/ plugins/guardrails/
RUN cargo build --release --package grimoire-guardrails

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/grimoire-guardrails /plugin
COPY plugins/guardrails/grimoire.json /grimoire.json
ENTRYPOINT ["/plugin"]
