# Security

What the panel can do, what it protects, and what it leaves to you.

## The panel is root on the host

The panel manages runners through the host's Docker socket, and whoever
can use that socket can start a container that mounts the host's whole
filesystem: Docker's socket is root on the host. The panel's container
runs as root for the same reason (it would gain nothing by not doing
so).

The panel has **no login**. Whoever can reach it can create, stop and
delete runners, and so, through Docker, could do anything on the host
that the panel's code lets them, and anything a bug in it might.

So:

- **Keep it on localhost**, as `compose.yaml` does (`127.0.0.1:8080`),
  and reach it from elsewhere through an SSH tunnel:

  ```sh
  ssh -N -L 8080:127.0.0.1:8080 you@runner-host
  ```

  then open <http://localhost:8080> on your own machine.

- **Or put it behind a proxy that authenticates**, and serves it over
  HTTPS. With Caddy, for example:

  ```
  runners.example.com {
      basic_auth {
          # caddy hash-password --plaintext '…'
          admin $2a$14$...
      }
      reverse_proxy 127.0.0.1:8080
  }
  ```

  The proxy must cover every path, `/api` included.

- **Never publish its port on a public interface** without one of the
  two.

## Registration tokens

A runner's registration token is given to the panel once, when the
runner is created.

- The panel does not store it: it passes it to the runner's container,
  and never answers it back through the API, nor writes it to its logs
  at any `RUST_LOG` level.
- The token is in the container's environment, so anyone who can run
  `docker inspect` on the host can read it, until it expires an hour
  after GitHub issued it. Within that hour it could register other
  runners for the same repository or organization.
- The runner's script unsets it before the runner starts, so the jobs a
  runner runs (which inherit its environment) do not see it.

Once registered, a runner authenticates with credentials GitHub gave it,
kept in its container's filesystem (`.runner`, `.credentials`). Deleting
the runner deletes them.

The panel is never given a GitHub token of any kind, and so cannot act
on GitHub on its own: not create tokens, not remove registrations.

## What runners can do

Jobs run inside their runner's container, as the `runner` user, who can
use `sudo` without a password: a job is root **in its container**.
The panel gives runners nothing of the host's: no Docker socket, no
mounts, no extra privileges, and Docker's default network, from which
they can reach the internet and whatever the host can reach.

A persistent runner keeps its container from job to job: what one job
leaves (files, installed tools, a process left running) is there for the
next, whichever workflow or repository it comes from. An ephemeral
runner takes one job only, so no job runs where another ran.

## Public repositories

A self-hosted runner of a public repository runs code from pull
requests, and anyone can open one from a fork. GitHub advises using
self-hosted runners only with private repositories. If you use one
with a public repository anyway:

- under the repository's **Settings › Actions › General**, require
  approval for workflows from all outside contributors;
- prefer ephemeral runners;
- keep workflows triggered by `pull_request` off self-hosted runners, or
  run them only for branches of the repository itself.

## The panel's image

The panel's image is distroless: the server's binary, the web panel's
files and the C runtime, with no shell or package manager for an
intruder to use. It listens on one port, and talks only to Docker.
