# The API

Everything the web panel does, it does through a JSON API under `/api`,
which scripts can use too. It has no authentication, like the panel
itself: see [security.md](security.md) before exposing it.

Bodies are JSON (`Content-Type: application/json`). Unknown fields, in
a body or a query, are refused rather than ignored, so a typo fails
loudly.

| Method and path                | What it does                                     | Success          |
| ------------------------------ | ------------------------------------------------ | ---------------- |
| `GET /api/health`              | Whether the panel is up.                         | `200`            |
| `GET /api/images`              | The images runners can be created from.          | `200`            |
| `GET /api/runners`             | The runners, optionally filtered.                | `200`            |
| `POST /api/runners`            | Creates a runner and starts it.                  | `201`, a runner  |
| `GET /api/runners/{id}`        | One runner.                                      | `200`, a runner  |
| `POST /api/runners/{id}/start` | Starts a runner.                                 | `200`, a runner  |
| `POST /api/runners/{id}/stop`  | Stops a runner.                                  | `200`, a runner  |
| `DELETE /api/runners/{id}`     | Deletes a runner.                                | `204`, no body   |

## A runner

```json
{
  "id": "9cf4fc44-2ce1-4c75-a246-5c95f620ded3",
  "forge": "github",
  "url": "https://github.com/lumen-labs/inference",
  "owner": "lumen-labs",
  "repository": "inference",
  "scope": "repository",
  "name": "gpu-1",
  "labels": ["gpu", "cuda"],
  "lifecycle": "persistent",
  "image": "ghcr.io/actions/actions-runner:latest",
  "status": "running",
  "exit_code": null,
  "created_at": 1794219413
}
```

| Field        | Type            | What it is                                                                                          |
| ------------ | --------------- | --------------------------------------------------------------------------------------------------- |
| `id`         | string (UUID)   | The runner's id; its container is `git-runner-swarm-<id>`.                                           |
| `forge`      | string          | `github`.                                                                                           |
| `url`        | string          | What it serves, normalized: no trailing `/` or `.git`.                                              |
| `owner`      | string          | The organization (or user) in the URL.                                                              |
| `repository` | string or null  | The repository, or `null` for a runner of every repository of the owner.                           |
| `scope`      | string          | `repository` or `owner`.                                                                            |
| `name`       | string          | What GitHub lists it as.                                                                            |
| `labels`     | array of string | Its extra labels, beyond the ones GitHub gives every runner.                                        |
| `lifecycle`  | string          | `persistent` (job after job) or `ephemeral` (one job).                                              |
| `image`      | string          | The image its container runs.                                                                       |
| `status`     | string          | Its container's state: `created`, `running`, `restarting`, `paused`, `exited` or `dead`.            |
| `exit_code`  | number or null  | What its container last exited with, while `exited`, `restarting` or `dead`; `null` otherwise.      |
| `created_at` | number          | When it was created, in seconds since the Unix epoch.                                               |

The status is Docker's, not GitHub's: `running` means the container
runs, not that GitHub sees the runner online. An `exited` runner with
an `exit_code` other than `0` failed; the web panel shows it as
*Failed*.

## `GET /api/health`

```sh
curl http://localhost:8080/api/health
```

```json
{"status":"ok"}
```

It answers as soon as the server serves; the server only serves once it
has reached Docker.

## `GET /api/images`

```json
[
  {
    "key": "github-official",
    "forge": "github",
    "image": "ghcr.io/actions/actions-runner:latest",
    "description": "GitHub's official runner image: the Actions runner on Ubuntu, and nothing else"
  }
]
```

`key` is what `POST /api/runners` takes as `image`; `image` is what is
pulled, as `GRS_GITHUB_IMAGE` configures it. Each forge's default image
comes first.

## `GET /api/runners`

Every runner, newest first (and by name among runners created at the
same second). Query parameters narrow the list; each is optional, an
empty one is ignored, and runners must match all that are given.

| Parameter    | Matches runners…                                                   |
| ------------ | ------------------------------------------------------------------ |
| `forge`      | of this forge: `github`                                            |
| `owner`      | of this owner, ignoring case                                       |
| `repository` | of a repository by this name, ignoring case                        |
| `scope`      | of this kind of scope: `owner` or `repository`                     |
| `status`     | in this status: `created`, `running`, `restarting`, `paused`, `exited`, `dead` |
| `label`      | with this extra label, ignoring case                               |
| `q`          | whose name or URL contains this text, ignoring case                |

```sh
curl 'http://localhost:8080/api/runners?owner=lumen-labs&label=gpu'
```

```json
[
  {
    "id": "9cf4fc44-2ce1-4c75-a246-5c95f620ded3",
    "forge": "github",
    "url": "https://github.com/lumen-labs/inference",
    "owner": "lumen-labs",
    "repository": "inference",
    "scope": "repository",
    "name": "gpu-1",
    "labels": ["gpu", "cuda"],
    "lifecycle": "persistent",
    "image": "ghcr.io/actions/actions-runner:latest",
    "status": "running",
    "exit_code": null,
    "created_at": 1794219413
  }
]
```

## `POST /api/runners`

Creates a runner and starts it, pulling its image first if Docker does
not have it (the first pull of GitHub's image takes a while: it is about
2 GB). The answer comes once the container has started; registering
with GitHub happens in the container, after that.

| Field       | Required | What it is                                                                                     |
| ----------- | -------- | ---------------------------------------------------------------------------------------------- |
| `url`       | yes      | A repository's URL, or an organization's for a runner of all its repositories.                 |
| `name`      | yes      | 1–64 letters, digits, `-`, `_` or `.`; unique, ignoring case, among the runners of that URL.   |
| `token`     | yes      | The registration token GitHub shows for that repository or organization.                      |
| `labels`    | no       | Extra labels: each 1–100 letters, digits, `-`, `_` or `.`. Empty ones are dropped.              |
| `lifecycle` | no       | `persistent` (the default) or `ephemeral`.                                                     |
| `image`     | no       | The `key` of an image from `GET /api/images`; the forge's default if left out or empty.        |
| `forge`     | no       | `github` (the default).                                                                        |

The URL may end in `/` or `.git`, so one copied from a repository's page
or a clone command works as it is. Spaces around the name and the labels
are ignored.

```sh
curl -X POST http://localhost:8080/api/runners \
  -H 'Content-Type: application/json' \
  -d '{
        "url": "https://github.com/lumen-labs/inference",
        "name": "gpu-1",
        "token": "<registration token>",
        "labels": ["gpu", "cuda"]
      }'
```

The answer is `201 Created` with the runner, as above. The token is
never part of any answer.

## `GET /api/runners/{id}`

The runner, or `404` if there is none with that id.

## `POST /api/runners/{id}/start` and `/stop`

Start or stop the runner's container, and answer the runner as it is
afterwards. Stopping gives the runner `GRS_STOP_TIMEOUT` seconds (30 by
default) to stop cleanly before it is killed. Starting a running runner,
or stopping a stopped one, is not an error.

## `DELETE /api/runners/{id}`

Removes the runner's container, stopping it first if it runs, and
answers `204 No Content`. GitHub keeps listing the runner, offline,
until it drops it or it is removed under the repository's or
organization's **Settings › Actions › Runners**.

## Errors

Every failure is answered with a status and a body saying why:

```json
{"error": "invalid URL: it names no owner or repository", "field": "URL"}
```

`field` is there when a single value is to blame, named as the domain
names it: `URL`, `owner` or `repository` (a part of the URL), `runner
name`, `registration token`, `label`, `lifecycle`, `image`, `forge`,
`scope` or `status`.

| Status | When                                                                                        |
| ------ | ------------------------------------------------------------------------------------------- |
| `400`  | The body or the query is not what the endpoint takes: not JSON, or a missing or unknown field. |
| `404`  | No runner has that id, or no endpoint has that path.                                        |
| `409`  | Another runner already has that name for that URL.                                          |
| `422`  | A value is invalid, or names an image that does not exist or serves another forge.         |
| `502`  | Docker refused: the image could not be pulled, or the container not created or started.     |
| `503`  | Docker cannot be reached.                                                                   |

Some answers, as the server gives them:

```
400  {"error": "Failed to deserialize the JSON body into the target type: colour: unknown field `colour`, expected one of `forge`, `url`, `name`, `token`, `labels`, `lifecycle`, `image` at line 1 column 63"}
404  {"error": "there is no runner 00000000-0000-4000-8000-000000000000"}
409  {"error": "a runner named GPU-1 already serves https://github.com/lumen-labs/inference", "field": "runner name"}
422  {"error": "invalid URL: it names no owner or repository", "field": "URL"}
422  {"error": "invalid runner name: ` ` is not allowed; only letters, digits, `-`, `_` and `.` are", "field": "runner name"}
422  {"error": "there is no runner image `gitea`", "field": "image"}
422  {"error": "invalid status: `asleep` is not a runner status", "field": "status"}
```

Paths under `/api` that name no endpoint are answered in JSON too
(`{"error": "there is no such endpoint"}`, `404`), never with the web
panel's page.
