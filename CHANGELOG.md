# Changelog

Each release, and what it changed. Versions follow
[Semantic Versioning](https://semver.org); while they are below 1.0,
a minor release may change what an earlier one did.

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
