# syntax=docker/dockerfile:1

# The whole panel in one image: the web panel, built by Node, served by
# the backend, built by Cargo, in a distroless image holding nothing but
# the two.

FROM node:26-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

FROM rust:1.98-trixie AS backend
WORKDIR /backend
COPY backend/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/backend/target \
    cargo build --release --locked -p swarm-server \
 && cp target/release/git-runner-swarm /usr/local/bin/git-runner-swarm

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
