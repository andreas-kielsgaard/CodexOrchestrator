FROM rust:1-bookworm AS build
WORKDIR /build
COPY crates/orchid-coordinator/Cargo.toml crates/orchid-coordinator/Cargo.lock ./
COPY crates/orchid-coordinator/src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release --bin orchid-coordinator && cp target/release/orchid-coordinator /build/orchid-coordinator

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home coordinator && mkdir /data && chown coordinator:coordinator /data
COPY --from=build /build/orchid-coordinator /usr/local/bin/orchid-coordinator
USER coordinator
EXPOSE 41800
ENTRYPOINT ["orchid-coordinator"]
CMD ["--data", "/data", "--bind", "0.0.0.0:41800", "--token-file", "/run/secrets/coordinator_token"]
