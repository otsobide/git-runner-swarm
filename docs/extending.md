# Extending

The panel is built so that what varies is an adapter: another runner
image, another forge or another engine is added beside what is there,
without changing the use cases. How the pieces fit is in
[architecture.md](architecture.md).

## Another runner image

An image runners can be created from is an adapter of the
`RunnerImage` port: it says which image it runs, which forge it serves,
and how to start a container of that image so the runner in it
registers. Nothing else in the backend knows anything about images.

1. **A crate** beside `swarm-github`, or a module in it for another
   GitHub image, depending only on `swarm-application` and
   `swarm-domain`. A sketch:

   ```rust
   use swarm_application::ports::{EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage};
   use swarm_domain::{Forge, ImageReference, RegistrationToken, RunnerDefinition, RunnerLifecycle};

   /// A runner image of our own, with the tools our jobs need.
   #[derive(Debug, Clone)]
   pub struct ToolboxImage {
       image: ImageReference,
   }

   impl RunnerImage for ToolboxImage {
       fn key(&self) -> &str {
           "toolbox" // what the API and the form choose it by
       }

       fn description(&self) -> &str {
           "Our runner image: GitHub's, with compilers and Docker's CLI"
       }

       fn forge(&self) -> Forge {
           Forge::GitHub
       }

       fn image(&self) -> &ImageReference {
           &self.image
       }

       fn launch_spec(&self, runner: &RunnerDefinition, token: &RegistrationToken) -> LaunchSpec {
           LaunchSpec {
               image: self.image.clone(),
               command: vec!["/usr/local/bin/start-runner".to_owned()],
               environment: vec![
                   EnvironmentVariable::plain("RUNNER_URL", runner.scope().to_string()),
                   EnvironmentVariable::plain("RUNNER_NAME", runner.name().as_str()),
                   EnvironmentVariable::plain("RUNNER_LABELS", runner.labels().to_comma_separated()),
                   // A secret: its value never shows in Debug output or logs.
                   EnvironmentVariable::secret("RUNNER_TOKEN", token.expose()),
               ],
               restart: match runner.lifecycle() {
                   RunnerLifecycle::Persistent => RestartPolicy::UnlessStopped,
                   RunnerLifecycle::Ephemeral => RestartPolicy::Never,
               },
           }
       }
   }
   ```

   The command must register the runner only on the container's first
   start (the token expires within the hour), keep the token from the
   jobs, and leave the runner as the process `docker stop` signals.
   The `SCRIPT` in `swarm-github/src/official.rs` does all three for
   GitHub's image; an image laid out like GitHub's is best adapted in
   that crate, where it can reuse the script.

2. **The workspace**: the crate is a member already (`crates/*`); add it
   to `[workspace.dependencies]` in `backend/Cargo.toml`, and as a
   dependency of `swarm-server`.

3. **The composition root**: add it to `images()` in
   `swarm-server/src/main.rs`. The first image of each forge is that
   forge's default:

   ```rust
   fn images(config: &Config) -> RunnerImages {
       RunnerImages::new([
           Arc::new(GitHubOfficialImage::new(config.github_image.clone())) as Arc<dyn RunnerImage>,
           Arc::new(ToolboxImage::new(config.toolbox_image.clone())),
       ])
   }
   ```

   If its image should be configurable, read a `GRS_*` variable for it in
   `config.rs`, as `GRS_GITHUB_IMAGE` is read.

4. **Tests**: unit tests of `launch_spec` (the command, each variable,
   the token as a secret, the restart policy per lifecycle), and, if it
   runs a script, a test running the script with stand-ins for the
   runner's own programs, as `swarm-github/tests/registration_script.rs`
   does.

The web panel needs no change: with more than one image, the form offers
a choice between them, by their descriptions.

## Another forge: Gitea, Forgejo

A forge is a variant of `Forge`, plus at least one image that serves it.

1. **The domain**: add the variant to `Forge` (`swarm-domain/src/forge.rs`),
   its name to `as_str` (`gitea`, `forgejo`), and the variant to
   `Forge::ALL`. The compiler then points at every `match` that must
   handle it.

2. **An image adapter** for the forge's runner, as above, with its own
   registration. Gitea's and Forgejo's runner registers with
   `register --no-interactive --instance <URL> --token <token> --name
   <name> --labels <labels>` and then runs as a daemon; its
   registration is kept in a `.runner` file, so the same first-start
   rule applies.

3. **Scopes**: `RunnerScope` holds an owner, or an owner and a
   repository, at a forge's address. That fits runners of a repository,
   a user or an organization. Gitea also has runners for a whole
   instance, which would be a third kind of scope.

4. **The web panel**: the form should send the `forge` of the image it
   chooses (the API answers each image's `forge`, and refuses an image
   of another forge than the one asked for), and its hints should name
   where each forge shows a registration token. Racks already name the
   forge's host when it is not `github.com`.

5. **The README and these docs**: say the forge is supported, and how to
   get its tokens.

## Another engine

Where runners run is an adapter of the `RunnerRuntime` port:
`swarm-docker` for a Docker engine, `swarm-memory` for none.

An engine's adapter must:

- **keep each runner's definition with the runner itself**, as
  `swarm-docker` keeps it in container labels, so the engine stays the
  only store and runners outlive the panel;
- **list only the panel's runners**, never other workloads;
- **map the engine's states to `RunnerStatus`**, keeping the exit code
  of a runner that exited or is being restarted;
- **apply the launch spec** as it is given (command, environment,
  restart policy) and never show a secret variable's value;
- **treat starting a running runner, and stopping a stopped one, as
  done**, and an unknown runner as `RuntimeError::NotFound`.

Then add a value of `GRS_RUNTIME` for it in `swarm-server/src/config.rs`,
and a branch in `runtime()` in `main.rs`. Its tests should mirror
`swarm-docker/tests/docker_runtime.rs`: each port method against a real
engine, ignored unless asked for.
