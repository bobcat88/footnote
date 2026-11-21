# syntax=docker/dockerfile:1.7

# Footnote in a hardened Linux container, for hosts that cannot run local builds
# (Windows Smart App Control / WDAC blocks Cargo build scripts: os error 4551) and for a shared hub (#392).
#
#   docker build -t footnote:local .
#   docker run --rm --network none --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m --cap-drop=ALL \
#     --security-opt=no-new-privileges:true -v footnote-state:/state -v "$PWD:/workspace:ro" footnote:local status
#
# Long-lived daemon for several agents: compose.yaml. Base images are pinned by digest (Dependabot refreshes them).

# trixie (glibc 2.41, GCC 14): the static ONNX Runtime that ort-sys downloads
# fails to link on bookworm (undefined __isoc23_strtoll).
FROM rust:1-trixie@sha256:223cc32ddca8f3ee90c436dbc94b6b5f052ea7f9653c5201e9612f08ad7d21b8 AS build

ARG CARGO_FEATURES=""

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY crates crates

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/root/.cache/ort.pyke.io,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    set -eu; \
    if [ -n "${CARGO_FEATURES}" ]; then \
        cargo build --locked --release --bin footnote --features "${CARGO_FEATURES}"; \
    else \
        cargo build --locked --release --bin footnote; \
    fi; \
    cp target/release/footnote /usr/local/bin/footnote

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2

ARG FOOTNOTE_UID=10001
ARG FOOTNOTE_GID=10001
ARG FOOTNOTE_STATE_DIR=/state
ARG FOOTNOTE_WORKSPACE=/workspace
ARG FOOTNOTE_WORKSPACES=/workspaces

LABEL org.opencontainers.image.title="Footnote" \
      org.opencontainers.image.description="Local, read-only memory, document retrieval, code graph, and token-lean tooling for AI coding agents" \
      org.opencontainers.image.source="https://github.com/bobcat88/footnote" \
      org.opencontainers.image.licenses="Apache-2.0"

# Runtime dependencies: TLS roots and OpenSSL for `models install` / `docs add`; git for repository scope detection
# (`footnote hook`) and project git status (`footnote projects`). No curl/wget or other network utilities.
#
# Git ownership trust (`safe.directory`) covers only the workspace mount points, never `*`. Mounted repositories
# belong to another UID, so git would refuse them otherwise. Footnote's git queries turn off `core.fsmonitor`
# and skip `status` for a repository that defines its own filter driver, so a repository's `.git/config`
# cannot start a program next to the state volume.
# Workspace mount points stay root-owned: the footnote user only reads them.
RUN set -eu; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates git libssl3t64; \
    rm -rf /var/lib/apt/lists/*; \
    for d in "${FOOTNOTE_WORKSPACE}" "${FOOTNOTE_WORKSPACES}"; do \
        git config --system --add safe.directory "${d}"; \
        git config --system --add safe.directory "${d}/*"; \
    done; \
    groupadd --gid "${FOOTNOTE_GID}" --system footnote; \
    useradd --uid "${FOOTNOTE_UID}" --gid "${FOOTNOTE_GID}" --system --home-dir /home/footnote --create-home --shell /usr/sbin/nologin footnote; \
    install -d -m 0700 -o "${FOOTNOTE_UID}" -g "${FOOTNOTE_GID}" "${FOOTNOTE_STATE_DIR}"; \
    install -d -m 0755 "${FOOTNOTE_WORKSPACE}" "${FOOTNOTE_WORKSPACES}"

COPY --from=build --chmod=0755 /usr/local/bin/footnote /usr/local/bin/footnote
COPY --chmod=0644 LICENSE NOTICE THIRD-PARTY-NOTICES.md /usr/share/doc/footnote/

# Memory DB, indexes and downloaded models all live here: mount a named volume on it.
ENV HOME=/home/footnote \
    FOOTNOTE_STATE_DIR=${FOOTNOTE_STATE_DIR} \
    TMPDIR=/tmp

VOLUME ["${FOOTNOTE_STATE_DIR}"]
WORKDIR ${FOOTNOTE_WORKSPACE}

USER ${FOOTNOTE_UID}:${FOOTNOTE_GID}

ENTRYPOINT ["footnote"]
CMD ["--help"]
