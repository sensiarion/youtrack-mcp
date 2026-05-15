mod config;
mod error;
mod model;
mod report;
mod server;
mod youtrack;

use rmcp::transport::stdio;
use rmcp::ServiceExt;

use crate::config::Config;
use crate::server::Server;
use crate::youtrack::YouTrack;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let yt = YouTrack::new(cfg)?;
    let service = Server::new(yt).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
