use std::time::Duration;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cups_http_gateway::entrypoints::http::{build_router, AppState};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "cups-http-gateway", version, about = "HTTP gateway for CUPS")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long, default_value_t = 6631)]
    port: u16,

    #[arg(long, default_value = "localhost")]
    cups_host: String,

    #[arg(long, default_value_t = 631)]
    cups_port: u16,

    #[arg(long, default_value = "info")]
    log_level: String,

    /// How long to cache printer capabilities before re-querying CUPS (seconds).
    /// Set to 0 to disable caching.
    #[arg(long, default_value_t = 3600)]
    caps_cache_ttl: u64,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::new(&args.log_level))
        .init();

    let caps_ttl = Duration::from_secs(args.caps_cache_ttl);
    let state = AppState::new(&args.cups_host, args.cups_port, caps_ttl);
    let router = build_router(state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));

    info!(
        "cups-http-gateway v{} listening on http://{}",
        env!("CARGO_PKG_VERSION"),
        addr
    );
    info!(
        "Forwarding to CUPS at {}:{} (caps cache TTL: {}s)",
        args.cups_host, args.cups_port, args.caps_cache_ttl
    );

    axum::serve(listener, router).await.expect("server error");
}
