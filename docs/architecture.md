# Architecture

How git-runner-swarm is put together, and why. For how to work on it,
see [development.md](development.md); for adding another runner image,
forge or engine, see [extending.md](extending.md).

## The whole

One container serves both halves of the panel:

- **the backend**, a Rust server that answers a JSON API under `/api`
  and drives the host's Docker engine through its socket;
- **the web panel**, a Vue application built to static files that the
  same server serves at every other path.

```
 browser ──HTTP──▶ ┌──────────── panel container ────────────┐
                   │  git-runner-swarm (Rust)                 │
                   │   /api/*   JSON API                      │──Docker API──▶ Docker engine
                   │   /*       web panel (Vue, static files) │  (unix socket)      │
                   └──────────────────────────────────────────┘                    ▼
                                                                         runner containers
                                                                         (one per runner)
```

There is no database. A runner *is* a container: its definition (URL,
name, labels, …) is written into the container's labels when the
container is created, and every list of runners is a list of the
containers carrying those labels. The panel can be stopped, upgraded or
deleted, and the runners carry on; a new panel finds them again.

## The backend: hexagonal, one crate per layer

The backend is written against **ports**, interfaces the application
defines for what it needs from outside, and everything outside is an
**adapter** of one: the Docker engine, a runner image, HTTP. Each layer
is its own crate in a Cargo workspace (`backend/`), so the compiler
enforces which way dependencies point. The domain depends on nothing, the
application only on the domain, and the adapters on the application.

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

| Crate               | Layer                          | Depends on                                     |
| ------------------- | ------------------------------ | ---------------------------------------------- |
| `swarm-domain`      | domain                         | nothing but `thiserror` and `uuid`             |
| `swarm-application` | application: use cases, ports  | `swarm-domain`                                 |
| `swarm-docker`      | driven adapter: runtime        | `swarm-application`, `bollard`                 |
| `swarm-memory`      | driven adapter: runtime        | `swarm-application`                            |
| `swarm-github`      | driven adapter: runner image   | `swarm-application`                            |
| `swarm-http`        | driving adapter: API, web      | `swarm-application`, `axum`, `tower-http`      |
| `swarm-server`      | composition root (the binary)  | all of the above, `tokio`, `tracing`           |

### `swarm-domain`: what a runner is

Every value a runner is made of is a **value object**: a type that can
only be built valid, so a `RunnerName` in hand is a valid name, and no
code past the edge checks it again. Each is built with `new` (or
`parse`) and refuses invalid input with one shared error,
`InvalidValue { field, reason }`. The API passes `field` on to the web
panel, which uses it to mark the input at fault.

| Value object        | What it holds                                   | Rules                                                                                     |
| ------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `RunnerId`          | a runner's id                                   | a UUID, generated at creation                                                             |
| `RunnerName`        | what the forge lists the runner as              | 1–64 letters, digits, `-`, `_` or `.`; compared ignoring case                            |
| `RunnerLabel`       | one extra label                                 | 1–100 letters, digits, `-`, `_` or `.`                                                   |
| `RunnerLabels`      | a runner's extra labels                         | duplicates (ignoring case) dropped; written comma-separated                               |
| `OwnerName`         | a user or organization                          | 1–64 letters, digits, `-`, `_` or `.`                                                    |
| `RepositoryName`    | a repository                                    | 1–100 letters, digits, `-`, `_` or `.`; not `.` or `..`                                  |
| `ForgeAddress`      | where a forge is                                | `http` or `https`, a hostname or IPv4 address, an optional port, no path or credentials   |
| `RunnerScope`       | what a runner serves: an owner or a repository  | read from a URL with one path segment (owner) or two (repository); `.git` and `/` dropped |
| `RegistrationToken` | the token a runner registers with               | 1–512 characters, no whitespace; its `Debug` output is redacted                           |
| `ImageReference`    | a container image                               | 1–512 characters, no whitespace                                                           |
| `Forge`             | which forge                                     | `github` (Gitea and Forgejo to come)                                                      |
| `RunnerLifecycle`   | how many jobs a runner takes                    | `persistent` (job after job) or `ephemeral` (one job)                                     |
| `CpuLimit`          | the most CPUs a runner may use                  | 0.01 to 1024 CPUs, to a thousandth; kept exactly, in thousandths                          |
| `MemoryLimit`       | the most memory a runner may use                | a size as Docker writes one (`512m`, `4g`); at least 6 MiB                                |
| `RunnerLimits`      | a runner's limits                               | a CPU limit and a memory limit, each optional                                             |
| `EngineAccess`      | whether jobs may use the runner's engine        | `isolated` (the default) or `host`                                                        |
| `LogTail`           | how many of a runner's latest log lines to read | 1 to 5000 lines, 200 unless asked                                                         |
| `LogLine`           | one line a runner logged                        | its text, and the output it went to: `stdout` or `stderr`                                 |
| `RunnerStatus`      | what its container is doing                     | `created`, `running`, `restarting`, `paused`, `exited` or `dead`                          |

Built from these:

- `RunnerDefinition`: what a runner was created as (id, forge, scope,
  name, labels, lifecycle, limits, engine access), and the rule across runners: two runners
  **conflict** when they would register under the same name in the same
  scope, since the forge would let the second replace the first.
- `Runner`: a definition plus what the runtime says of it now: image,
  status, exit code, creation time.
- `RunnerFilter`: which runners a list asks for (forge, owner,
  repository, scope kind, status, label, free text), and whether one
  runner matches.

### `swarm-application`: the use cases and their ports

`RunnerService` holds one method per use case: `create`, `list`,
`get`, `start`, `stop`, `delete` and `logs`. It needs two things from outside,
declared as ports (`swarm_application::ports`):

```rust
/// Where runners run and are kept.
#[async_trait]
pub trait RunnerRuntime: Send + Sync {
    async fn ensure_image(&self, image: &ImageReference) -> Result<(), RuntimeError>;
    async fn launch(&self, runner: &RunnerDefinition, spec: &LaunchSpec) -> Result<Runner, RuntimeError>;
    async fn list(&self) -> Result<Vec<Runner>, RuntimeError>;
    async fn find(&self, id: RunnerId) -> Result<Option<Runner>, RuntimeError>;
    async fn start(&self, id: RunnerId) -> Result<(), RuntimeError>;
    async fn stop(&self, id: RunnerId) -> Result<(), RuntimeError>;
    async fn remove(&self, id: RunnerId) -> Result<(), RuntimeError>;
    async fn logs(&self, id: RunnerId, tail: LogTail) -> Result<Vec<LogLine>, RuntimeError>;
}

/// How to start a container of one image so the runner in it registers.
pub trait RunnerImage: Send + Sync {
    fn key(&self) -> &str;
    fn description(&self) -> &str;
    fn forge(&self) -> Forge;
    fn image(&self) -> &ImageReference;
    fn launch_spec(&self, runner: &RunnerDefinition, token: &RegistrationToken) -> LaunchSpec;
}
```

A `LaunchSpec` is everything an image needs its container started with:
the image, the command, the environment (each variable plain or
**secret**, a secret's value redacted from `Debug` output and logs), and
a restart policy. The runtime knows nothing of any image, and the
images know nothing of Docker.

`RunnerImages` holds the images runners can be created from, by key,
and picks a forge's default when a request names none.

`RunnerError` says why a use case failed: no such runner, the name is
taken in that scope, no such image, an image of another forge, no image
for the forge, or a `RuntimeError` (the engine unreachable, an image it
could not pull, a container it could not create or start).

`testing.rs` holds a fake runtime and a fake image for the use cases'
own unit tests.

### `swarm-docker`: runners as Docker containers

The `RunnerRuntime` adapter for a Docker engine, through `bollard`.

- **Connection**: as the Docker CLI connects, at `DOCKER_HOST` or the
  local socket; `ping` is called at start, so a panel that cannot reach
  Docker says so once and exits.
- **Images**: pulled only when the engine lacks them, by name and tag
  (`latest` when a reference has no tag, since asking for a bare name
  pulls every tag), or by digest.
- **Containers**: named `git-runner-swarm-<id>`, created with the spec's
  command, environment and restart policy, with Docker's `init` as
  PID 1 (to reap what jobs leave behind and pass signals on), and with
  the runner's definition in its labels (below). A container that fails
  to start is removed, so a failed create leaves nothing behind.
- **Lists**: every container labelled `git-runner-swarm.managed=true`;
  one whose labels cannot be read back is skipped, with a warning.
- **State**: Docker's container state, as a `RunnerStatus`; the exit code
  is kept for `exited`, `restarting` and `dead` containers, so a runner
  that keeps failing shows why.
- **Stop**: `docker stop` with the configured timeout (`GRS_STOP_TIMEOUT`).
  Starting a running runner or stopping a stopped one is not an error.
- **Remove**: forced, with the container's anonymous volumes.
- **Errors**: a 404 from the engine is a runner that does not exist;
  failing to connect is `Unreachable`; anything else is `Failed`.

#### What a runner's container carries

| Label                        | Value                                                           |
| ---------------------------- | --------------------------------------------------------------- |
| `git-runner-swarm.managed`   | `true`: what marks a container as a runner of the panel's       |
| `git-runner-swarm.id`        | the runner's id                                                 |
| `git-runner-swarm.forge`     | `github`                                                        |
| `git-runner-swarm.url`       | the URL it serves, as normalized: `https://github.com/owner/repository` |
| `git-runner-swarm.name`      | its name                                                        |
| `git-runner-swarm.labels`    | its extra labels, comma-separated                               |
| `git-runner-swarm.lifecycle` | `persistent` or `ephemeral`                                     |
| `git-runner-swarm.cpus`      | its CPU limit, `1.5`; missing for none (since 0.2)              |
| `git-runner-swarm.memory`    | its memory limit, `4g`; missing for none (since 0.2)            |
| `git-runner-swarm.engine`    | `host` when its jobs are given Docker; missing if not (since 0.2) |

Labels are fixed when a container is created, so a runner's definition
never changes after creation: to change one, delete the runner and
create it again. Labels added since the first release are optional, so
a container an earlier panel made is still read, as the runner it was.

A runner's limits are applied as Docker's own: the CPU limit as the
container's `NanoCpus`, the memory limit as its `Memory`.

A runner given the engine has the engine's socket bound into its
container at `/var/run/docker.sock` (`Binds`), and joins the group
owning it (`GroupAdd`). The runtime is told both as an `EngineSocket`:
the socket's path on the host (`GRS_RUNNER_DOCKER_SOCKET`), which the
engine resolves, and the group, which `swarm-server` reads from the
socket the panel itself reaches Docker at.

### `swarm-github`: GitHub's official runner image

`GitHubOfficialImage` turns a runner's definition into a launch spec for
`ghcr.io/actions/actions-runner`. That image is built for Actions Runner
Controller and has no entrypoint that registers a runner, so the adapter
gives its containers a short bash script as their command:

1. If the container has never registered (no `.runner` file), it runs
   `config.sh --unattended --replace --work _work --url … --token …
   --name …`, plus `--labels` and `--ephemeral` when they apply.
2. It unsets `RUNNER_TOKEN`, so the jobs the runner runs (which inherit
   its environment) do not see it.
3. It `exec`s `run.sh`, so the runner is the process `docker stop`
   signals, and goes offline cleanly.

Registering only on the first start matters: the token expires an hour
after GitHub shows it, and a container started again later finds its
registration in its own filesystem and does not need it. `--replace`
lets a runner recreated under a name GitHub still lists (offline) take
that name over.

| Variable           | Value                                                    |
| ------------------ | -------------------------------------------------------- |
| `RUNNER_URL`       | the scope's URL                                          |
| `RUNNER_NAME`      | the runner's name                                        |
| `RUNNER_LABELS`    | its extra labels, comma-separated                        |
| `RUNNER_EPHEMERAL` | `true` or `false`                                        |
| `RUNNER_TOKEN`     | the registration token (a secret variable)               |

Restart policy: `unless-stopped` for a persistent runner, so it comes
back after a crash or a reboot; none for an ephemeral one, which is done
once it exits.

### `swarm-memory`: runners kept in memory

A `RunnerRuntime` that keeps runners in a map and runs nothing: to try
the panel without Docker (`GRS_RUNTIME=memory`), and to test the API
without an engine.

### `swarm-http`: the API and the web panel

An `axum` router: the API under `/api` (see [api.md](api.md)) and, given
the web panel's directory, its files at every other path. Paths that are
not files get `index.html`, so the panel loads at any URL. Requests and
responses are DTOs of their own (`dto.rs`), translated into value
objects at the edge. Unknown fields in a body or a query are refused, and
`ApiError` maps each failure to a status and a body naming the field at
fault (see [api.md](api.md#errors)).

### `swarm-server`: the composition root

The binary, `git-runner-swarm`. The only code that knows every adapter.
It reads the configuration from the environment (`config.rs`, see
[configuration.md](configuration.md)), builds the runtime and the
images, hands them to a `RunnerService`, and serves the router. Logs go
to stderr, in colour only on a terminal. On SIGTERM or Ctrl-C it
finishes the requests under way and exits; runners are left running.

## How a runner is created

1. The web panel posts the form to `POST /api/runners`.
2. `swarm-http` turns the body into a `CreateRunner` command, building
   each value object. An invalid value is answered `422`, naming the
   field.
3. `RunnerService::create` picks the image (the one asked for, or the
   forge's default) and makes a `RunnerDefinition` with a new id.
4. It lists the existing runners, and refuses the new one (`409`) if
   another has the same name in the same scope, ignoring case.
5. The image adapter turns the definition and the token into a
   `LaunchSpec`.
6. The runtime pulls the image if Docker lacks it, creates the container
   with the spec and the definition's labels, and starts it.
7. The container's script registers the runner with GitHub and runs it.
   The API answers `201` with the runner as Docker now reports it.

From then on the panel only reads containers. Every list of runners
asks Docker afresh.

## The web panel

`web/` is a Vue 3 application in TypeScript, built by Vite, and laid out
the same way as the backend:

```
web/src/
  core/runner.ts            the API's types, and what the panel makes of a runner:
                            its health, its status in words, grouping by owner, filters as a query
  core/gateway.ts           the port: RunnersGateway, and GatewayError (message, status, field)
  adapters/httpGateway.ts   the port over the HTTP API
  state/useRunners.ts       the panel's state: runners, filters, images, busy runners, notices
  components/               the page's parts (below)
  testing/fakeGateway.ts    an in-memory gateway for tests
  styles/                   tokens.css (colours, type, spacing) and base.css
  App.vue, main.ts          the page, and the one place the HTTP gateway is chosen
```

- **State.** `useRunners` polls the API every five seconds. When
  filters are set, it fetches every runner (for the summary and the
  filters' options) and the matching ones (for the racks). A generation
  counter drops answers that arrive after the filters changed. Actions
  mark their runner busy until they finish; a failure becomes a notice.
- **Components.** `SwarmStrip` (every runner's lamp and the counts),
  `FilterBar`, `OwnerRack` (one owner's runners), `RunnerRow` (one
  runner, with its actions and an inline delete confirmation),
  `StatusLamp`, `CreateRunnerPanel` (the form, as a dialog that
  marks the field the API blames), and `RunnerLogs` (a runner's latest
  log lines, kept current by `state/useRunnerLogs.ts` while open).
- **Design.** Colour means status and nothing else: the lamps are the
  only saturated things on the page, so a runner in trouble is the first
  thing seen. Everything else is steel and ink, in tokens
  (`styles/tokens.css`) defined twice, for light and for dark, chosen by
  the system's `prefers-color-scheme`. Fonts (Atkinson Hyperlegible Next
  and Mono) are bundled, so the panel loads nothing from elsewhere.
- **Accessibility.** Every field has a label; the status is always in
  words beside its lamp, which is hidden from screen readers; the form
  moves focus to the first field, closes on Escape and hands focus back;
  changes are announced; animations stop under reduced motion.

## The image

The `Dockerfile` has three stages: Node builds the web panel, Rust
builds the server (with Cargo's registry and target directory as build
caches), and both land in `gcr.io/distroless/cc-debian13`, which holds
the C runtime the binary needs and nothing else: no shell, no package
manager. The image is about 54 MB.

It is built for linux/amd64 and linux/arm64. The two build stages run on
the platform doing the build, whichever the image is for: the web panel
is the same files on every platform, and the server is cross-compiled
for the image's (`--target`, with a cross linker; no dependency compiles
C). Only the last stage, which runs nothing, is of the image's platform,
so building for arm64 on amd64 emulates nothing and takes as long as
building for amd64. Releases are published to Docker Hub by the
`release` workflow (see [development.md](development.md#releasing)). It runs as root, because reaching the
Docker socket requires it, and the socket is root on the host either
way (see [security.md](security.md)).

## Decisions, and what they cost

- **Docker as the only store.** Nothing to back up or migrate, and the
  panel can never disagree with Docker about which runners exist. The
  cost is that a runner's definition is fixed at creation, and the
  panel only knows what labels can hold.
- **No forge credentials.** The panel is given a registration token per
  runner, never a GitHub token. So it cannot remove a runner's
  registration when the runner is deleted, nor fetch tokens itself:
  GitHub drops offline runners on its own (a day for an ephemeral one,
  two weeks for another), or they are removed by hand.
- **A script for the official image.** GitHub's image is the one GitHub
  maintains and updates; giving it a command keeps it unmodified, rather
  than shipping and maintaining an image of the panel's own.
- **No login.** The panel is meant for localhost, or behind a proxy that
  authenticates; see [security.md](security.md).
