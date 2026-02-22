FROM rust:1.85-alpine AS builder
ARG FEATURES="yaml"
RUN apk add --no-cache musl-dev build-base
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --features "${FEATURES}"

FROM scratch
COPY --from=builder /build/target/release/switchboard /switchboard
ENTRYPOINT ["/switchboard"]
CMD ["run"]
