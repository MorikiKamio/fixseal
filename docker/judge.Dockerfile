FROM rust:1.99.1-bookworm
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends python3 curl ca-certificates zstd clang libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN case "$TARGETARCH" in amd64) platform=linux ;; arm64) platform=linux_aarch64 ;; *) exit 1 ;; esac \
    && curl --fail --location "https://github.com/leanprover/lean4/releases/download/v4.34.1/lean-4.34.1-${platform}.tar.zst" -o /tmp/lean.tar.zst \
    && mkdir /opt/lean && tar --zstd -xf /tmp/lean.tar.zst --strip-components=1 -C /opt/lean \
    && rm /tmp/lean.tar.zst
ENV FIXSEAL_LEAN_BIN=/opt/lean/bin
ENV PATH=/opt/lean/bin:$PATH
ENV CARGO_TARGET_DIR=/work/target
WORKDIR /opt/fixseal
COPY judge/Cargo.toml judge/Cargo.lock ./judge/
COPY judge/src ./judge/src
COPY judge/tests ./judge/tests
COPY agent-contract/Cargo.toml agent-contract/Cargo.lock ./agent-contract/
COPY agent-contract/src ./agent-contract/src
COPY agent-contract/tests ./agent-contract/tests
COPY shared ./shared
COPY lean ./lean
COPY examples/fixers ./examples/fixers
COPY target ./target
COPY scripts/judge-check.sh scripts/judge_check.py ./scripts/
# Explicit provisioning is separate from judge-check: runtime has no network.
RUN cargo fetch --manifest-path judge/Cargo.toml --locked \
    && cargo fetch --manifest-path agent-contract/Cargo.toml --locked
COPY docker/judge-entrypoint.sh /usr/local/bin/judge-entrypoint
ENTRYPOINT ["bash", "/usr/local/bin/judge-entrypoint"]
