FROM rust:1.87-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY sdk/ sdk/
COPY plugins/slack/ plugins/slack/
RUN cargo build --release --package grimoire-slack

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/grimoire-slack /plugin
COPY plugins/slack/grimoire.json /grimoire.json
ENTRYPOINT ["/plugin"]
