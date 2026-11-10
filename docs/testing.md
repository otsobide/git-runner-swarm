# Testing

Every level of the system is tested, from a single value object to the
panel's container driven in a browser against a real Docker engine.

## Running the tests

| Command             | Runs                                                                                  | Needs          |
| ------------------- | ------------------------------------------------------------------------------------- | -------------- |
| `make test`         | All of the below, as CI runs them.                                                    | Docker         |
| `make test/backend` | `cargo fmt --check`, `cargo clippy` (warnings denied), every backend test.            | Docker         |
| `make test/web`     | The web panel's typecheck, unit and integration tests, and build.                     | —              |
| `make test/e2e`     | The end-to-end tests, then the web panel's contract test against the same panel.      | Docker, a browser |
| `make ci`           | The CI workflow itself, on this machine, with [act](https://github.com/nektos/act).   | Docker, act    |

`E2E_BROWSER_CHANNEL=chrome make test` runs the end-to-end tests in an
installed Google Chrome, rather than the Chromium Playwright downloads
(`cd e2e && npx playwright install chromium`).

## The levels

| Level       | Where                                   | What it holds                                                                                                                         |
| ----------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Unit        | each backend crate's `src/`             | Value objects and their rules; the use cases against fakes of their ports; each adapter's translation, one function at a time.        |
| Unit        | `web/src/**/*.test.ts`                  | What the panel makes of a runner (`core/`), the HTTP gateway against a fake `fetch`, the panel's state, and each component.           |
| Integration | each backend crate's `tests/`           | The use cases with the real adapters; the Docker runtime against a real engine; the official image's script run in bash; the API through its router; the server's binary run as a process. |
| Integration | `web/src/panel.integration.test.ts`     | The whole panel, mounted with a fake gateway, and driven through the page alone, as a person would.                                    |
| Contract    | `web/src/adapters/*.contract.test.ts`   | The web panel's types and gateway, held to what a running API answers.                                                                |
| End to end  | `e2e/tests/`                            | The panel's container, built from the Dockerfile and given Docker, driven in a browser, and the runners' containers looked inside.    |

### The backend

```sh
cd backend
cargo test --workspace                       # everything but what needs Docker
cargo test --workspace -- --include-ignored  # everything
cargo test -p swarm-docker -- --include-ignored  # one crate
```

Tests that need a Docker engine are marked `#[ignore]`, so `cargo test`
passes anywhere; `make test/backend` and CI run them with
`--include-ignored`. They run `busybox:1.37` containers that sleep,
which is enough to see a container created, started, stopped, listed and
removed, and they remove every container they create.

Worth knowing about:

- `swarm-application/tests/use_cases.rs` runs the use cases with the
  real GitHub image adapter and the in-memory runtime.
- `swarm-github/tests/registration_script.rs` runs the official image's
  script in bash, with stand-ins for `config.sh` and `run.sh`: that it
  registers once, passes labels and `--ephemeral` only when asked, and
  hides the token from the runner.
- `swarm-docker/tests/docker_runtime.rs` runs the Docker runtime against
  the engine.
- `swarm-http/tests/` drives the router with requests, against the
  in-memory runtime.
- `swarm-server/tests/server.rs` starts the binary as a process: that it
  reads its environment, refuses bad settings and an unreachable Docker,
  stops cleanly on SIGTERM, and never logs a runner's token.

### The web panel

```sh
cd web
npm ci
npm run typecheck
npm test                     # unit and integration tests
npm run test:contract        # the contract test, given a panel (below)
```

Tests run in Vitest with jsdom and Vue Test Utils. Components and the
whole panel are given a `FakeGateway` (`src/testing/fakeGateway.ts`), an
in-memory stand-in for the API that answers copies, as JSON over HTTP
would.

The contract test runs the real HTTP gateway against a running panel,
and checks what it answers has the shape the panel's types say. It is
skipped unless `GRS_API` names the panel's API:

```sh
GRS_API=http://127.0.0.1:8080/api npm run test:contract
```

It only creates (and deletes) runners of owners starting `contract-`.

### End to end

```sh
cd e2e
npm ci
npm test
```

Before the tests, Playwright:

1. builds the stand-in runner image (`e2e/fake-runner`) as
   `git-runner-swarm-e2e-runner:latest`;
2. builds the panel from the repository's `Dockerfile` and starts it with
   `e2e/compose.yaml` (Compose project `grs-e2e`, on
   `127.0.0.1:18090`), given the host's Docker and told to run GitHub
   runners from the stand-in image.

After them, it deletes the runners the tests created and stops the
panel.

| File                  | What it tests                                                                                                     |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `creating.spec.ts`    | Creating runners from the form, for a repository and for an owner, and what each registered with; a refused value pointed at in its field; a second runner of a name refused. |
| `lifecycle.spec.ts`   | Stopping, starting (without registering twice) and deleting runners, and their containers following.              |
| `filters.spec.ts`     | Runners racked by owner; searching, and filtering by owner, repository, status and label; no match, and clearing. |
| `statuses.spec.ts`    | A runner failing, restarting, and finishing its one job, as the panel shows them.                                 |
| `persistence.spec.ts` | Runners outliving the panel: restarted, it finds them again.                                                      |
| `logs.spec.ts`        | A runner's logs from its row: what its container wrote, kept current while open, and why a runner failed.         |

**The stand-in runner image** is laid out like GitHub's (a `runner`
user, `/home/runner`, `config.sh` and `run.sh`), so the panel's own
registration script runs unchanged, with no GitHub account, token or
network:

- `config.sh` writes the arguments it was given to `config.args` and
  "registers" by writing `.runner`, unless the token is `expired`, which
  fails the way GitHub refuses one;
- `run.sh` writes its environment to `run.env`, then runs until it is
  stopped, or, for an ephemeral runner, "takes its job" for three
  seconds and exits.

The tests read those files inside the runners' containers, to check
what each runner was really started with.

**Real runners are safe**: the tests only create, and only delete,
runners of owners starting `e2e-` (and `contract-`), so they can run on
a Docker that also holds real runners.

| Variable              | What it does                                                                                       |
| --------------------- | -------------------------------------------------------------------------------------------------- |
| `E2E_BROWSER_CHANNEL` | `chrome` to run the tests in an installed Google Chrome.                                            |
| `E2E_BASE_URL`        | A panel already running, to test instead of starting one; the test that restarts the panel is skipped. |
| `E2E_KEEP`            | `1` to leave the panel running afterwards (as `make test/e2e` does, for the contract test).         |

On failure, Playwright keeps a trace of the failing test in
`e2e/test-results`; `npx playwright show-trace <trace.zip>` replays it.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `main` or `develop`,
every pull request, and on demand:

- **backend**: `cargo fmt --check`, `cargo clippy -D warnings`, and
  `cargo test --workspace -- --include-ignored`, with Cargo's caches
  kept between runs;
- **web**: `npm ci`, the typecheck, the tests and the build;
- **e2e**, once both have passed: the end-to-end tests in Playwright's
  Chromium, then the contract test against the same panel. On failure,
  the Playwright report and traces are uploaded as an artifact, kept for
  a week.

Jobs run on the runner the `CI_RUNS_ON` repository variable names, or on
`ubuntu-latest` without it. The e2e job and the backend's Docker tests
need a Docker engine the job can use, as GitHub's hosted runners have.

`make ci` runs the same workflow locally with act, in
`catthehacker/ubuntu:act-latest` containers that use the host's Docker.

A second workflow, `release.yml`, runs when a release is tagged: it
publishes the image to Docker Hub for amd64 and arm64, then pulls what
it published and checks it serves on both (see
[development.md](development.md#releasing)).
