# syntax=docker/dockerfile:1

# The whole panel in one image: the web panel, built by Node, served by
# the backend, built by Cargo, in a distroless image holding nothing but
# the two.
#
# Built for linux/amd64 and linux/arm64, from either: both build stages
# run on the platform building the image, so nothing is emulated, and
# the backend is cross-compiled for the platform the image is for.

FROM --platform=$BUILDPLATFORM node:26-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

FROM --platform=$BUILDPLATFORM rust:1.98-trixie AS backend
ARG TARGETARCH
WORKDIR /backend
# The Rust target of the platform the image is for, and the linker for
# it: the system's own when that is the platform building, a cross one
# otherwise. No dependency compiles C, so a linker is all it takes.
RUN case "$TARGETARCH" in \
      amd64) echo x86_64 > /target-arch ;; \
      arm64) echo aarch64 > /target-arch ;; \
      *) echo "the panel is not built for $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && arch="$(cat /target-arch)" \
 && rustup target add "$arch-unknown-linux-gnu" \
 && if ! command -v "$arch-linux-gnu-gcc" >/dev/null; then \
      apt-get update \
      && apt-get install -y --no-install-recommends \
           "gcc-$(echo "$arch" | tr _ -)-linux-gnu" "libc6-dev-$TARGETARCH-cross" \
      && rm -rf /var/lib/apt/lists/*; \
    fi
COPY backend/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/backend/target,sharing=locked \
    arch="$(cat /target-arch)" \
 && export "CARGO_TARGET_$(echo "$arch" | tr a-z A-Z)_UNKNOWN_LINUX_GNU_LINKER=$arch-linux-gnu-gcc" \
 && cargo build --release --locked -p swarm-server --target "$arch-unknown-linux-gnu" \
 && cp "target/$arch-unknown-linux-gnu/release/git-runner-swarm" /usr/local/bin/git-runner-swarm

# The C runtime the binary links against, and nothing else: no shell, no
# package manager. It runs as root, as reaching the Docker socket it is
# given takes, which is root on the host either way.
FROM gcr.io/distroless/cc-debian13
COPY --from=backend /usr/local/bin/git-runner-swarm /usr/local/bin/git-runner-swarm
COPY --from=web /web/dist /srv/web
ENV GRS_LISTEN=0.0.0.0:8080 \
    GRS_WEB_DIR=/srv/web
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/git-runner-swarm"]
