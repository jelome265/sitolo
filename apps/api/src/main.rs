//! Sitolo API binary: process entry and listener lifecycle.
//!
//! Phase 0–2 audit (F-001/F-020): `main` only orchestrates
//! build → bind → serve → shutdown. Composition lives in [`bootstrap`].
//! The listener binds only after [`bootstrap::StartupContext`] exists, so a
//! socket is never published before startup checks complete.
#![forbid(unsafe_code)]

use std::sync::Arc;

use sitolo_api_bin::bootstrap::{StartupContext, StartupError};
use sitolo_api_bin::serve::{ServeError, serve};
use tokio::sync::oneshot;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), StartupError> {
    run().await
}

async fn run() -> Result<(), StartupError> {
    let context = StartupContext::build().await?;
    // Structural readiness gate: no socket exists before this point.
    let listener = tokio::net::TcpListener::bind(context.config().bind_address)
        .await
        .map_err(|source| {
            // The OS error can quote the bound address; only the class crosses
            // the startup boundary.
            drop(source);
            StartupError::ServeFailed {
                class: "listener-bind",
            }
        })?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let serve_task = tokio::spawn(serve(listener, Arc::clone(context.state()), shutdown_rx));
    wait_for_shutdown_signal().await?;
    let _sent = shutdown_tx.send(());
    match serve_task.await {
        Ok(Ok(_shutdown_order)) => Ok(()),
        Ok(Err(error)) => Err(StartupError::ServeFailed {
            class: serve_error_class(&error),
        }),
        Err(_) => Err(StartupError::ServeFailed { class: "task-join" }),
    }
}

fn serve_error_class(error: &ServeError) -> &'static str {
    match error {
        ServeError::InvalidRouteInventory => "route-inventory",
        ServeError::Listener(_) => "listener",
        ServeError::DrainDeadlineExceeded => "drain-deadline",
    }
}

async fn wait_for_shutdown_signal() -> Result<(), StartupError> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(signal_error)?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                result.map_err(signal_error)
            },
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.map_err(signal_error)
    }
}

/// Maps a signal-handler failure to its fail-closed startup class. The
/// underlying OS error carries no operator-usable detail beyond the class.
fn signal_error(source: std::io::Error) -> StartupError {
    drop(source);
    StartupError::SignalHandlerUnavailable
}
