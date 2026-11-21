# Footnote in a Linux container, for hosts that cannot run local builds
# (Windows Smart App Control / WDAC blocks Cargo build scripts: os error 4551).
#
#   docker build -t footnote:local .
#   docker run --rm -i -v footnote-state:/state -v "$PWD:/workspace" footnote:local status

# trixie (glibc 2.41, GCC 14): the static ONNX Runtime that ort-sys downloads
# fails to link on bookworm (undefined __isoc23_strtoll).
FROM rust:1-trixie AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release && cp target/release/footnote /usr/local/bin/footnote

FROM debian:trixie-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates git libssl3t64 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/footnote /usr/local/bin/footnote
# Memory DB, indexes and downloaded models all live here: mount a volume on it.
ENV FOOTNOTE_STATE_DIR=/state
VOLUME /state
WORKDIR /workspace
ENTRYPOINT ["footnote"]
CMD ["--help"]
