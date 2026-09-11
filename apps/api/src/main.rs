//! Sitolo API binary: process entry and listener lifecycle.
//!
//! Phase 0–2 audit (F-001/F-020): `main` only orchestrates
//! build → bind → serve → shutdown. Composition lives in [`bootstrap`].
//! The listener binds only after [`bootstrap::StartupContext`] exists, so a
//! socket is never published before startup checks complete.
#![forbid(unsafe_code)]

use std::sync::Arc;

use sitolo_api_bin::bootstrap::{StartupContext, StartupError};
use sitolo_api_bin::serve::serve;
use tokio::sync::oneshot;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(error) => {
            eprintln!("sitolo-api: startup failed: {error}");
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<(), StartupError> {
    let context = StartupContext::build().await?;
    // Structural readiness gate: no socket exists before this point.
    let listener = tokio::net::TcpListener::bind(context.config().bind_address)
        .await
        .map_err(|_| StartupError::ConfigRejected { problems: 0 })?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let serve = tokio::spawn(serve(listener, Arc::clone(context.state()), shutdown_rx));
    wait_for_shutdown_signal().await;
    let _ = shutdown_tx.send(());
    let _ = serve.await;
    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("sitolo-api: failed to install SIGTERM handler: {error}");
                std::process::exit(1);
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}