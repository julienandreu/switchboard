FROM rust:1.85-slim AS builder
ARG FEATURES="yaml"
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --features "${FEATURES}"

FROM scratch
COPY --from=builder /build/target/release/switchboard /switchboard
ENTRYPOINT ["/switchboard"]
CMD ["run"]
