# Configuration

The panel is configured with environment variables, each optional: left
unset (or set empty), each has a default that suits running the panel
with the repository's `compose.yaml`.

| Variable           | Default                                 | What it is                                                       |
| ------------------ | --------------------------------------- | ---------------------------------------------------------------- |
| `GRS_LISTEN`       | `0.0.0.0:8080`                          | The address and port the server listens on.                      |
| `GRS_WEB_DIR`      | `/srv/web` in the image; none otherwise | The built web panel. Without one, the server is the API alone.   |
| `GRS_RUNTIME`      | `docker`                                | `docker`, or `memory` to try the panel with nothing running.     |
| `GRS_GITHUB_IMAGE` | `ghcr.io/actions/actions-runner:latest` | The image GitHub runners run.                                    |
| `GRS_STOP_TIMEOUT` | `30`                                    | Seconds a runner is given to stop cleanly before it is killed.   |
| `GRS_RUNNER_DOCKER_SOCKET` | `/var/run/docker.sock`          | Where Docker's socket is on the host, for runners given Docker.  |
| `GRS_RUNNER_GPUS`  | `/dev/dri`                              | The GPUs runners given the host's are given.                     |
| `DOCKER_HOST`      | the local socket                        | Where Docker is, as for the Docker CLI.                          |
| `RUST_LOG`         | `info`                                  | How much the panel logs.                                         |

A variable set to something the panel cannot use stops it at start,
with a message naming the variable and what it expects:

```
2026-11-09T10:23:23.306469Z ERROR git_runner_swarm: GRS_RUNTIME is `podman`, which is not docker or memory
```

## Setting them

With Compose, under the service's `environment` in `compose.yaml`, or in
a `compose.override.yaml` beside it, which Compose merges in on its own
and which keeps local settings out of the repository's file:

```yaml
# compose.override.yaml
services:
  panel:
    environment:
      GRS_GITHUB_IMAGE: ghcr.io/actions/actions-runner:2.338.2
      GRS_STOP_TIMEOUT: "60"
```

Then `docker compose up -d` again. With `docker run`, pass each with
`-e NAME=value`.

## `GRS_LISTEN`

The address inside the panel's container. Where the panel can be reached
*from* is decided by the published port, in `compose.yaml`:

```yaml
    ports:
      - "127.0.0.1:8080:8080"   # this host only (the default)
      - "127.0.0.1:9000:8080"   # this host only, on port 9000 instead
```

Publishing it on every interface (`"8080:8080"`) hands control of the
host's Docker to anyone who can reach the port: see
[security.md](security.md) first.

## `GRS_WEB_DIR`

Where the built web panel is. The image sets it to `/srv/web`, where it
copies the panel. Unset, the server answers the API alone, which is how
it runs in development, beside Vite's dev server (see
[development.md](development.md)).

## `GRS_RUNTIME`

- `docker`: runners are containers of the Docker engine at
  `DOCKER_HOST`. The panel checks Docker answers before it serves, and
  exits if it does not.
- `memory`: runners are kept in the panel's memory. Nothing runs, nothing
  is pulled, and everything is lost when the panel stops: for trying the
  panel, or the API, with no Docker at all.

## `GRS_GITHUB_IMAGE`

The image GitHub runners are created from. It must be GitHub's official
runner image or one laid out like it: the runner in `/home/runner`, with
its `config.sh` and `run.sh`, and `bash`.

The panel pulls an image only when Docker does not have it, and a
runner keeps the image it was created from. So:

- changing the variable changes the image of runners created afterwards,
  not of existing ones;
- `latest` is pulled once and does not move by itself: `docker pull` it
  again, then recreate runners, to move them to a newer image. (The
  runner program inside updates itself when GitHub releases a new
  version, as any self-hosted runner does.)

**Pinning a version**, so every runner runs the same one:

```yaml
GRS_GITHUB_IMAGE: ghcr.io/actions/actions-runner:2.338.2
```

**A mirror**, for a host that cannot reach `ghcr.io`:

```yaml
GRS_GITHUB_IMAGE: registry.example.com/actions/actions-runner:2.338.2
```

**An image of your own**, for runners that come with the tools your
jobs need rather than installing them in every job. Build it from the
official one:

```Dockerfile
FROM ghcr.io/actions/actions-runner:2.338.2
USER root
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential \
 && rm -rf /var/lib/apt/lists/*
USER runner
```

```sh
docker build -t my-runner:1 .
```

and set `GRS_GITHUB_IMAGE: my-runner:1`. An image Docker already has is
used as it is, never pulled, so an image built on the host needs no
registry.

## `GRS_STOP_TIMEOUT`

How many seconds **Stop** and **Delete** give a runner to stop cleanly
(cancelling the job it is running and telling GitHub it is going
offline) before Docker kills it. A whole number, `0` or more.

## `GRS_RUNNER_DOCKER_SOCKET`

Where Docker's socket is **on the host**, for the runners whose jobs are
given the host's Docker to mount. It is a path on the host, not in the
panel's container: Docker resolves it when it creates a runner's
container. An absolute path; `/var/run/docker.sock` unless set.

Runners given Docker also join the group owning the socket, so their
user may use it. The panel finds that group from the socket it reaches
Docker at itself, which is the host's when it is mounted into the
panel's container, as `compose.yaml` does; it says which group at start,
or warns that it cannot tell (for an engine reached over the network).

For rootless Docker, mount its socket into the panel as below, and set
`GRS_RUNNER_DOCKER_SOCKET: /run/user/1000/docker.sock`.

## `GRS_RUNNER_GPUS`

The GPUs given to runners whose jobs are given the host's GPUs: device
files **on the host**, each an absolute path, and `nvidia`, separated by
commas. `/dev/dri` unless set.

| The host's GPUs              | Set it to           | What the runners' jobs get                                                                    |
| ---------------------------- | ------------------- | --------------------------------------------------------------------------------------------- |
| AMD or Intel                 | `/dev/dri`          | Rendering, video and compute as Mesa drives them: OpenGL, Vulkan, VA-API, OpenCL.             |
| AMD, with ROCm               | `/dev/dri,/dev/kfd` | ROCm's compute as well: HIP, and the frameworks built on it.                                  |
| NVIDIA                       | `nvidia`            | Every NVIDIA GPU, as `docker run --gpus all` gives them, with every capability of the driver. |
| NVIDIA and an integrated one | `/dev/dri,nvidia`   | Both.                                                                                         |

A device path is mapped into the runner's container at the same path,
and a directory, as `/dev/dri`, gives every device in it. Runners also
join the groups owning the devices (`video` and `render`, on most
hosts), so their user may open them. The panel cannot see the host's
devices from its own container, so it asks Docker: before creating a
runner given the GPUs, it runs a short-lived container of the runner's
image, given the same devices, to list their groups, and removes it.

`nvidia` takes the
[NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html)
on the host, set up for Docker (`nvidia-ctk runtime configure
--runtime=docker`, then restart Docker). The toolkit mounts the driver's
libraries and tools, as `nvidia-smi`, into the runner's container, and
the panel asks it for every capability of the driver (graphics and
video as well as compute) with `NVIDIA_DRIVER_CAPABILITIES=all`.

A host without a device the list names makes creating a runner given the
GPUs fail, saying which; runners not given them are not affected. The
panel says the list at start.

## `DOCKER_HOST`

Where the Docker engine is, read as the Docker CLI reads it: unset, the
socket at `/var/run/docker.sock`; otherwise a `unix://` path or a
`tcp://` address.

With Compose, the simplest is to mount the engine's socket where the
panel looks for it. For rootless Docker, for example:

```yaml
    volumes:
      - /run/user/1000/docker.sock:/var/run/docker.sock
```

## `RUST_LOG`

How much the panel logs, to stderr (`docker compose logs panel`), as
[`tracing`'s filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
reads it:

| Value                     | Logs                                                         |
| ------------------------- | ------------------------------------------------------------ |
| `info`                    | Start, stop, and what went wrong.                            |
| `info,tower_http=debug`   | That, and every HTTP request with its status and duration.   |
| `debug`                   | Everything, from every library.                              |

Whatever `RUST_LOG` says, the Docker client (`bollard`) never logs below
`info`: from `debug` on it would log what it sends Docker, and a
runner's container is created with its registration token.
