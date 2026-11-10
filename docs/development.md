# Development

How to work on git-runner-swarm. How it is built is in
[architecture.md](architecture.md); how it is tested, in
[testing.md](testing.md).

## What you need

- **Rust 1.98** or newer (edition 2024), with `rustfmt` and `clippy`.
- **Node 26** (`web/.node-version`), with npm.
- **Docker**, with Compose v2 and BuildKit: for the image, the Docker
  tests and the end-to-end tests.
- **make**.

Optional:

- **Google Chrome**, to run the end-to-end tests in it
  (`E2E_BROWSER_CHANNEL=chrome`) rather than downloading Playwright's
  Chromium (`cd e2e && npx playwright install chromium`).
- **[act](https://github.com/nektos/act)**, to run the CI workflow
  locally (`make ci`).

## The repository

```
backend/                  the Rust workspace
  Cargo.toml              workspace, shared dependencies and lints
  crates/swarm-domain/    value objects, Runner, filters
  crates/swarm-application/  use cases and ports
  crates/swarm-docker/    the Docker runtime adapter
  crates/swarm-memory/    the in-memory runtime adapter
  crates/swarm-github/    GitHub's official image adapter
  crates/swarm-http/      the API and the web panel's server
  crates/swarm-server/    the binary: configuration and wiring
web/                      the Vue web panel
e2e/                      end-to-end tests (Playwright), and the screenshots
  fake-runner/            a stand-in for GitHub's runner image
  screenshots/            what the documentation's screenshots show
docs/                     this documentation, and its images
Dockerfile                the panel's image: web, backend, distroless
compose.yaml              running the panel on this host
Makefile                  tests, CI and screenshots
.github/workflows/ci.yml  the CI workflow
CHANGELOG.md              what each release changed
```

## Running it while working on it

The backend, against runners kept in memory, so nothing touches Docker:

```sh
cd backend
GRS_RUNTIME=memory cargo run -p swarm-server     # API on :8080
```

The web panel, served by Vite's dev server with hot reload, which hands
`/api` on to the backend on port 8080:

```sh
cd web
npm ci
npm run dev                                      # http://localhost:5173
```

### Against real Docker, without GitHub

The backend talks to the local Docker when run without
`GRS_RUNTIME=memory` (your user needs access to the Docker socket). So
that runners do not try to register with GitHub, give it the stand-in
runner image the end-to-end tests use. It is laid out like GitHub's,
and pretends to register: any token works, except `expired`, which
fails as GitHub would refuse it.

```sh
docker build -t git-runner-swarm-e2e-runner e2e/fake-runner
cd backend
GRS_GITHUB_IMAGE=git-runner-swarm-e2e-runner:latest cargo run -p swarm-server
```

Delete the runners you create when you are done: they are real
containers, and outlive the server.

### The image

```sh
docker build -t otsobide/git-runner-swarm .   # for this machine's platform
docker compose up -d                           # then run it, as a user would
```

Cargo's registry and target directory are BuildKit caches, so a rebuild
after a small change is much quicker than the first build.

The image is built for linux/amd64 and linux/arm64, from either: both
build stages run on the platform building, and the backend is
cross-compiled, so nothing is emulated. To build both, with a builder
that can hold more than one platform:

```sh
docker buildx create --name multiarch --driver docker-container
docker buildx build --builder multiarch --platform linux/amd64,linux/arm64 .
```

## Conventions

### Rust

- **Value objects** for every value of the domain, built valid or not
  at all, with `InvalidValue { field, reason }` as the one error of
  validation. Code past the edges takes value objects, not strings.
- **Secrets** (tokens, secret environment variables) have a `Debug`
  that shows no value, so they cannot leak through a log line or a
  panic message.
- **Errors** with `thiserror`. Messages are in lower case, with no full
  stop, and say what happened in the user's terms: `there is no runner
  …`, `a runner named … already serves …`.
- **Lints**, in the workspace's `Cargo.toml`: `unsafe` is forbidden,
  every public type is `Debug`, and clippy's pedantic lints are on. CI denies
  every warning.
- **Formatting** is `rustfmt`'s default.
- **Documentation**: every public item has a doc comment, saying what it
  is and why it is so, not repeating its signature.
- **Tests** of a module live at its foot, in `mod tests`; tests across
  modules, or against Docker or a process, live in the crate's `tests/`.
- **Dependencies** are workspace dependencies, declared once in
  `backend/Cargo.toml`; `Cargo.lock` is committed and used (`--locked`).

### The web panel

- **TypeScript**, strict, checked by `vue-tsc` (`npm run typecheck`).
- **Single-file components** with `<script setup>`.
- **The gateway is the only way out**: components and state never call
  `fetch`; they use the `RunnersGateway` they are given, which tests
  replace with `FakeGateway`.
- **Colours come from tokens** (`styles/tokens.css`), defined for both
  light and dark; a component never writes a colour of its own. Colour
  means status: the lamps, and nothing else, are saturated.
- **Accessible**: every control has a label, the status is always in
  words, and everything works from a keyboard.
- **Dependencies** are pinned to exact versions; `package-lock.json` is
  committed and used (`npm ci`).

### Writing

Everything in the repository is in English: code, comments,
documentation, commit messages and the panel's text. The panel speaks
to the person using it in plain words, names actions by what they do
(**Create runner**, **Stop**, **Delete**), and says in an error what
went wrong and what to do about it.

## Git flow

- `main` holds releases; `develop`, what the next one will be.
- Every change is made on a `feature/<what>` branch off `develop`, and
  merged back with `git merge --no-ff`, so the history shows each
  feature as one.
- A release is a `release/<version>` branch off `develop`, where the
  changelog (and the version, if it was not changed yet) is updated. It
  is merged into `main` with `--no-ff`, tagged `v<version>` (an
  annotated tag), and merged back into `develop`.
- A fix that cannot wait for the next release is a `hotfix/<version>`
  branch off `main`, finished the same way.

Versions follow [Semantic Versioning](https://semver.org). The version
is the backend workspace's, in `backend/Cargo.toml`
(`[workspace.package] version`).

### Commits

- **Small and whole**: one change per commit, which builds and passes
  its tests on its own.
- **Subject**: `scope: what the commit does`, in lower case and without
  a full stop, the scope being the part it changes: `domain`,
  `application`, `docker`, `github`, `memory`, `http`, `server`, `web`,
  `e2e`, `ci`, `build` or `docs`.
- **Body**: why the change was made, what it changes for whom, and
  anything a reviewer would otherwise have to work out, wrapped at 72
  columns.
- **No trailers** naming tools or co-authors.

## Releasing

1. `git switch -c release/X.Y.Z develop`
2. Add the release to `CHANGELOG.md`, and set the version in
   `backend/Cargo.toml` if it is not set already (then
   `cargo check --workspace` to update `Cargo.lock`). Commit.
3. `git switch main && git merge --no-ff release/X.Y.Z`
4. `git tag -a vX.Y.Z -m "git-runner-swarm X.Y.Z"`
5. `git switch develop && git merge --no-ff release/X.Y.Z`
6. `git branch -d release/X.Y.Z`
7. `git push origin main develop`, and wait for CI to pass on both.
8. `git push origin vX.Y.Z`: the `release` workflow
   (`.github/workflows/release.yml`) builds the image for amd64 and
   arm64, pushes it to Docker Hub as `X.Y.Z`, `X.Y` and `latest`, then
   pulls it back and checks it serves on both platforms.

The workflow logs in to Docker Hub as `otsobide` with the repository
secret `DOCKERHUB_TOKEN`, a Docker Hub access token with read and write
access. A failed publish is retried by re-running the workflow, on the
same tag.

## Screenshots

The screenshots in `docs/images` are taken by `make screenshots`: it
builds the web panel, serves it, answers its API with the sample
runners in `e2e/screenshots/sample.ts`, and takes them in dark mode,
on a desktop and on a phone. After changing how the panel looks, run it
(with `E2E_BROWSER_CHANNEL=chrome` to use Chrome), look the images over,
and commit them.
