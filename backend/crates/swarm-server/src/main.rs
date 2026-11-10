//! The panel: the application put together with its adapters, serving
//! its API and its web panel over HTTP.
//!
//! This is the one place that knows which adapters there are. Another
//! runner image (Gitea's, Forgejo's) is added to [`images`]; another
//! engine, to [`runtime`].

mod config;

use std::error::Error;
use std::io::IsTerminal;
use std::process::ExitCode;
use std::sync::Arc;

use swarm_application::ports::{RunnerImage, RunnerRuntime};
use swarm_application::{RunnerImages, RunnerService};
use swarm_docker::{DockerRuntime, EngineSocket, local_socket, socket_group};
use swarm_github::GitHubOfficialImage;
use swarm_memory::InMemoryRuntime;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::{Config, RuntimeKind};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(log_filter())
        // Colours for a person at a terminal, plain text for `docker logs`.
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(std::io::stderr)
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// What the panel logs: what `RUST_LOG` asks for, `info` when it asks for
/// nothing, but never the Docker client below `info`, whatever it asks:
/// from `debug` on, the client logs the bodies it sends Docker and gets
/// back, and a runner's container holds its registration token.
fn log_filter() -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(
            "bollard=info"
                .parse()
                .expect("a valid directive for the Docker client"),
        )
}

async fn run() -> Result<(), Box<dyn Error>> {
    let config = Config::from_lookup(|name| std::env::var(name).ok())?;

    let service = Arc::new(RunnerService::new(runtime(&config).await?, images(&config)));
    let router =
        swarm_http::router(service, config.web_dir.as_deref()).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(address = %listener.local_addr()?, "serving the panel");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown())
        .await?;
    tracing::info!("stopped");
    Ok(())
}

/// Where runners run, as the configuration says. Docker is checked to
/// answer before the panel serves anything, so a panel that cannot reach
/// it says so once, at start, rather than on every request.
async fn runtime(config: &Config) -> Result<Arc<dyn RunnerRuntime>, Box<dyn Error>> {
    match config.runtime {
        RuntimeKind::Docker => {
            let docker = DockerRuntime::connect()?
                .with_stop_timeout(config.stop_timeout)
                .with_engine_socket(engine_socket(config));
            docker.ping().await.map_err(|error| {
                format!("{error}; is the Docker socket mounted into the panel's container?")
            })?;
            tracing::info!("runners run on Docker");
            Ok(Arc::new(docker))
        }
        RuntimeKind::Memory => {
            tracing::warn!(
                "runners are kept in memory: none of them runs, and all are lost when the panel stops"
            );
            Ok(Arc::new(InMemoryRuntime::new()))
        }
    }
}

/// The engine's socket as runners given Docker get it: the path on the
/// host the configuration says, and the group owning the socket this
/// panel reaches the engine at, which is the same socket when the panel
/// is given the host's, as `compose.yaml` gives it.
fn engine_socket(config: &Config) -> EngineSocket {
    let group = local_socket(std::env::var("DOCKER_HOST").ok().as_deref())
        .and_then(|path| socket_group(&path));
    if let Some(group) = group {
        tracing::info!(
            socket = config.runner_docker_socket,
            group,
            "runners given Docker mount its socket and join its group"
        );
    } else {
        tracing::warn!(
            socket = config.runner_docker_socket,
            "the group of Docker's socket is unknown: runners given Docker may be refused its use"
        );
    }
    EngineSocket::new(config.runner_docker_socket.clone(), group)
}

/// The images runners can be created from, each forge's default first.
fn images(config: &Config) -> RunnerImages {
    RunnerImages::new([
        Arc::new(GitHubOfficialImage::new(config.github_image.clone())) as Arc<dyn RunnerImage>,
    ])
}

/// Resolves when the panel is asked to stop: Ctrl-C, or the SIGTERM
/// `docker stop` sends. Requests under way are finished first; runners
/// are left as they are, running on without the panel.
async fn shutdown() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!("cannot listen for Ctrl-C: {error}");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!("cannot listen for SIGTERM: {error}");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
    tracing::info!("stopping");
}
