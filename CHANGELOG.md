# Changelog

Each release, and what it changed. Versions follow
[Semantic Versioning](https://semver.org); while they are below 1.0,
a minor release may change what an earlier one did.

## 0.2.0, 2026-11-10

### Added

- **Limits**: a runner can be limited in the CPUs (`2`, `0.5`) and the
  memory (`4g`, `512m`) it and its jobs may use, from the form or the
  API (`cpus`, `memory`). They are Docker's own limits on its container,
  and its row says what they are.
- **The host's Docker for a runner's jobs**, only when asked: the host's
  socket is mounted where the Docker CLI looks for it, and the runner's
  user joins the group owning it, so jobs can build images and run
  containers. The form warns what it costs, and the runner's row says it
  has it. `GRS_RUNNER_DOCKER_SOCKET` says where the socket is on the
  host, for rootless Docker.
- **Logs**: each runner's row opens its last 200 lines, kept current,
  and the API answers them at `GET /api/runners/{id}/logs`.

### Compatible

- Runners created by 0.1 are read as they were: without limits, and
  without Docker.

## 0.1.1, 2026-11-09

### Fixed

- A runner's registration token is never written to the panel's logs.
  With `RUST_LOG=debug` or more, the Docker client logged it, in the
  container it was creating; it is now held to `info` whatever
  `RUST_LOG` says.
- In dark mode, the page behind the new-runner form is darkened, rather
  than washed out.
- The panel no longer says a user's URL makes a runner of all the
  user's repositories: GitHub has such runners for organizations only.

### Documentation

- The README is a quick start, with screenshots of the panel in dark
  mode.
- `docs/` explains usage, configuration, security, the API, the
  architecture, development, testing and extending the panel in depth.
- `make screenshots` takes the documentation's screenshots, from sample
  runners.

## 0.1.0, 2026-11-07

The first release: GitHub Actions runners, created and managed from a
web panel, as Docker containers.

### Runners

- Create a GitHub runner from a registration token, for a repository or
  for every repository of a user or organization, with labels, and
  either taking job after job or one job and then stopping.
- Runners run GitHub's official image (`ghcr.io/actions/actions-runner`),
  pulled when Docker does not have it, or another tag or mirror of it
  (`GRS_GITHUB_IMAGE`).
- Start, stop and delete runners; a runner keeps running when the panel
  stops, and is found again when it starts.
- No two runners of one repository or owner share a name.

### The web panel

- Runners grouped by the user or organization they serve, each with its
  status, what it serves, its labels and how long ago it was created.
- Filters by owner, repository, status and label, and a search of names
  and URLs.
- Refreshed every five seconds; usable from a keyboard, a screen reader
  and a phone.

### The API

- `/api/runners` and `/api/images`, as the README describes, with
  failures naming the value to blame.

### Running it

- One container, the Rust backend and the Vue panel together, given the
  host's Docker socket; configured by `GRS_*` environment variables.

### Known limitations

- Deleting a runner leaves it listed on GitHub, offline, until GitHub
  drops it.
- A persistent runner that cannot register is restarted over and over
  until it is stopped or deleted.
- No login: the panel is for localhost, or behind a proxy that
  authenticates.
- No Gitea or Forgejo runners yet.
