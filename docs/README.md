# git-runner-swarm documentation

The [README](../README.md) says what the panel is and how to start it.
These pages go further.

## Using it

- **[Usage](usage.md)**: running the panel; creating runners, field by
  field; using them in workflows; reading statuses; filtering;
  stopping, starting and deleting; troubleshooting.
- **[Configuration](configuration.md)**: every setting, and recipes:
  pinning the runner image, a mirror, an image with more tools.
- **[Security](security.md)**: what the panel can do, how to keep it
  private, where tokens go, and what runners mean for public
  repositories.
- **[API](api.md)**: the JSON API the panel is built on, for scripts.

## Working on it

- **[Architecture](architecture.md)**: how the panel is put together:
  hexagonal crates, value objects, Docker as the only store, the web
  panel, and the decisions behind them.
- **[Development](development.md)**: setting up, running it while
  working on it, conventions, git flow, commits and releases.
- **[Testing](testing.md)**: the levels of tests, how to run each, the
  end-to-end tests' stand-in runner, and CI.
- **[Extending](extending.md)**: adding a runner image, a forge (Gitea,
  Forgejo) or an engine.
