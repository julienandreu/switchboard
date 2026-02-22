FROM rust:1.85-alpine AS builder
ARG FEATURES="full"
ARG GIT_HASH="unknown"
ARG GIT_SHORT="unknown"
ARG GIT_BRANCH="unknown"
RUN apk add --no-cache musl-dev build-base
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src/ src/
ENV SWITCHBOARD_GIT_HASH_OVERRIDE=${GIT_HASH}
ENV SWITCHBOARD_GIT_SHORT_OVERRIDE=${GIT_SHORT}
ENV SWITCHBOARD_GIT_BRANCH_OVERRIDE=${GIT_BRANCH}
RUN cargo build --release --features "${FEATURES}"

FROM scratch
COPY --from=builder /build/target/release/switchboard /switchboard
ENTRYPOINT ["/switchboard"]
CMD ["run"]
