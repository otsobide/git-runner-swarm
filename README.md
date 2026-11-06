# git-runner-swarm

A web panel to create and manage self-hosted CI runners as Docker
containers: create a runner from the registration token your forge
gives you, then see every runner, grouped by the user or organization it
serves, and start, stop or delete it.

GitHub Actions runners for now, run from GitHub's official image
(`ghcr.io/actions/actions-runner`); Gitea and Forgejo runners are meant
to follow, as adapters of their own.

The whole panel, its Rust backend and its Vue frontend, is one
container, given the host's Docker socket.

## Running it

```sh
docker compose up -d
```

Then open <http://localhost:8080>.

To create a runner you need a registration token: on GitHub, under the
repository's (or organization's) **Settings › Actions › Runners › New
self-hosted runner**. It expires an hour after GitHub shows it. In the
panel, choose **New runner** and give it:

- the URL of the repository (`https://github.com/owner/repository`) for
  a runner of that repository, or of the user or organization
  (`https://github.com/owner`) for a runner of all its repositories;
- the token;
- a name, unique among the runners of that repository or owner;
- labels for `runs-on` to pick it by, beyond the `self-hosted`, OS and
  architecture labels GitHub gives every runner;
- whether it takes **job after job**, or **one job** and then stops
  (an ephemeral runner, so no job ever runs where another ran).

The panel pulls the runner's image if Docker does not have it, and
starts it. Workflows then select it with `runs-on: self-hosted` (or its
labels).

### Configuration

Environment variables, each optional:

| Variable           | Default                                 | What it is                                                             |
| ------------------ | --------------------------------------- | ---------------------------------------------------------------------- |
| `GRS_LISTEN`       | `0.0.0.0:8080`                          | The address and port to listen on.                                     |
| `GRS_WEB_DIR`      | `/srv/web` in the image; none otherwise | The built web panel. Without one, the server is the API alone.         |
| `GRS_RUNTIME`      | `docker`                                | `docker`, or `memory` to try the panel with nothing running.           |
| `GRS_GITHUB_IMAGE` | `ghcr.io/actions/actions-runner:latest` | The tag, or a mirror, of GitHub's image that GitHub runners run.       |
| `GRS_STOP_TIMEOUT` | `30`                                    | Seconds a runner is given to stop cleanly before it is killed.         |
| `DOCKER_HOST`      | the local socket                        | Where Docker is, as for the Docker CLI.                                |
| `RUST_LOG`         | `info`                                  | How much the panel logs.                                               |

## Before you expose it

- **Whoever reaches the panel controls the host's Docker**, which is
  root on the host. The panel has no login yet: the compose file binds
  it to localhost, and anything wider belongs behind a proxy that
  authenticates.
- **A runner's token is in its container's environment.** Its jobs do
  not see it (it is unset before the runner starts), but anyone who can
  `docker inspect` the container can, until it expires an hour after it
  was issued.
- **Self-hosted runners and public repositories** do not mix well: a
  pull request from a fork can run its own code on your runner. GitHub
  advises self-hosted runners for private repositories only; if you use
  one for a public repository, require approval for workflows from
  outside contributors, and prefer one-job runners.

## How it is built

Hexagonal: the application is written against ports, and everything it
talks to (Docker, a runner image, HTTP) is an adapter of one. The
backend is a Cargo workspace with one crate per piece, so the compiler
holds which way dependencies point.

```
                 ┌──────────────── swarm-server ────────────────┐
                 │   reads the configuration, puts it together   │
                 └───────────────────────────────────────────────┘
  driving                                                         driven
  ┌────────────┐     ┌─────────────────────────────────┐     ┌─────────────────┐
  │ swarm-http │ ──▶ │ swarm-application               │ ──▶ │ swarm-docker    │ RunnerRuntime
  │  API + web │     │  RunnerService (the use cases)  │     │ swarm-memory    │
  └────────────┘     │  ports: RunnerRuntime,          │     ├─────────────────┤
                     │         RunnerImage             │ ──▶ │ swarm-github    │ RunnerImage
                     │ swarm-domain                    │     └─────────────────┘
                     │  value objects, Runner, filters │
                     └─────────────────────────────────┘
```

- **`swarm-domain`**: what a runner is, made of value objects
  (`RunnerName`, `RunnerScope`, `RegistrationToken`, …) that are valid
  once built, and the rules across runners: no two in one scope under
  one name, which runners a filter matches.
- **`swarm-application`**: the use cases (create, list, get, start,
  stop, delete) and the two ports they need. `RunnerRuntime` is where
  runners run and are kept; `RunnerImage` is how to start a container
  of one image so the runner in it registers.
- **`swarm-docker`**: runners as Docker containers. A runner's
  definition is kept in its container's labels, so Docker is the only
  place runners are kept, and the panel lists exactly the containers
  there are.
- **`swarm-github`**: GitHub's official runner image, which has no
  entrypoint of its own; the adapter gives its containers a script that
  registers the runner on its first start and runs it on every start.
- **`swarm-memory`**: runners kept in memory, to try the panel or test
  the API without Docker.
- **`swarm-http`**: the JSON API under `/api`, and the web panel at
  every other path.

The web panel (`web/`) is built the same way: `core/` holds the API's
types and what the panel shows of a runner, `core/gateway.ts` is the
port the panel's state (`state/useRunners.ts`) works through, and
`adapters/httpGateway.ts` implements it over the API.

### Adding another runner image

A Gitea or Forgejo runner, or an image of your own, is an adapter of
`RunnerImage`: a crate beside `swarm-github` whose type says which
image it runs and turns a runner's definition and token into a
`LaunchSpec` (the command, environment and restart policy its
container starts with), plus a line in `images()` in `swarm-server`.
A new forge is also a variant of `Forge`. Nothing else changes: not the
use cases, not the Docker runtime, not the API.

### The API

| Method and path                | What it does                                                                                     |
| ------------------------------ | ------------------------------------------------------------------------------------------------ |
| `GET /api/health`              | Whether the panel is up.                                                                         |
| `GET /api/images`              | The images runners can be created from.                                                          |
| `GET /api/runners`             | The runners, filtered by `forge`, `owner`, `repository`, `scope`, `status`, `label` and `q`.     |
| `POST /api/runners`            | Creates a runner and starts it: `url`, `name`, `token`, and optionally `labels`, `lifecycle`, `image`. |
| `GET /api/runners/{id}`        | A runner.                                                                                        |
| `POST /api/runners/{id}/start` | Starts it.                                                                                       |
| `POST /api/runners/{id}/stop`  | Stops it.                                                                                        |
| `DELETE /api/runners/{id}`     | Deletes it.                                                                                      |

Failures are answered as `{"error": "why", "field": "which"}`, the field
there when one value is to blame.

## Developing

```sh
# Backend: unit and integration tests, then those that need Docker too.
cd backend
cargo test --workspace
cargo test --workspace -- --include-ignored

# The panel against runners kept in memory.
GRS_RUNTIME=memory cargo run -p swarm-server

# Web panel: tests, and a dev server that hands /api to the one above.
cd web
npm ci
npm test
npm run dev
```

Branches follow git flow: `main` holds releases, `develop` what is next,
and each change is a `feature/*` branch off `develop`.

## Not done yet

- **Deleting a runner leaves it listed on GitHub**, offline, until
  GitHub drops it (a day for a one-job runner, two weeks otherwise):
  removing its registration takes a GitHub credential the panel is not
  given.
- **A runner that cannot register is retried**: a persistent runner
  given an expired token exits, and its restart policy starts it again,
  over and over. The panel shows it restarting, with the code it exited
  with; stopping or deleting it ends the loop.
- **No login**, as above.
- **Gitea and Forgejo**.
