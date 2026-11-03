//! Production composition for the durable SNS FIFO outbox relay.
#![forbid(unsafe_code)]

use std::{error::Error, sync::Arc, time::Duration};

use sitolo_config::{EnvLoader, Environment, validate};
use sitolo_events::OutboxClaimPolicy;
use sitolo_integrations::secrets::AwsSecretsManagerProvider;
use sitolo_persistence::{PgWorkerPool, outbox_repository::PostgresOutboxRepository};
use sitolo_security::{EnvSecretProvider, SecretProvider};
use sitolo_worker_bin::relay::{RelayPolicy, RelayWorker};
use sitolo_worker_bin::sns::{SnsFifoPublisher, SnsRelayConfig};
use tokio::sync::watch;

// FW-TOK-01: the worker entry point is the single sanctioned runtime
// construction site; library crates receive the runtime instead of building one.
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-01: the binary entry point is the only place allowed to construct the multi-thread runtime."
)]
#[tokio::main(flavor = "multi_thread")]
#[expect(
    clippy::print_stderr,
    reason = "the worker has no logging subscriber until configuration validates, and the fixed message deliberately omits the error chain, which can embed credentials"
)]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_failure) => {
            // The error chain may quote DSNs, secret references, or provider
            // diagnostics, so only the fixed class is reported.
            eprintln!("sitolo-worker: startup or relay failed; process stopped");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let raw = EnvLoader::load().map_err(|_unknown| "invalid worker configuration key")?;
    let config = validate(raw)?;
    let (Some(region), Some(topic_arn)) = (
        config.relay_aws_region.as_deref(),
        config.relay_sns_topic_arn.as_deref(),
    ) else {
        return Err("SNS FIFO relay destination is not configured".into());
    };
    let database = config
        .worker_database_runtime()
        .ok_or("separate app_worker secret reference is required")?;
    let worker_secret_ref = database.password_ref.clone();
    let credential = match config.environment {
        Environment::Production => {
            let secret_region = config
                .secrets_aws_region
                .as_deref()
                .ok_or("managed secret region is required")?;
            let provider = AwsSecretsManagerProvider::from_ecs_task_role(
                secret_region,
                worker_secret_ref.clone(),
                Duration::from_secs(5),
            )
            .await?;
            provider.get(&worker_secret_ref).await?
        }
        Environment::Development | Environment::Staging => {
            let provider = EnvSecretProvider::new(false);
            provider.get(&worker_secret_ref).await?
        }
    };
    let database_pool = PgWorkerPool::connect(
        &database,
        credential,
        !matches!(config.environment, Environment::Development),
    )
    .await?;

    let publisher_config = SnsRelayConfig::new(region, topic_arn, Duration::from_secs(10))?;
    let publisher = Arc::new(SnsFifoPublisher::from_ecs_task_role(publisher_config).await?);
    let reader = Arc::new(PostgresOutboxRepository::new(database_pool.pool()));
    let claim = OutboxClaimPolicy::new(
        usize::try_from(config.relay_batch_size)?,
        Duration::from_secs(30),
        config.relay_max_attempts,
    )?;
    let policy = RelayPolicy::new(
        claim,
        Duration::from_secs(1),
        Duration::from_millis(100),
        Duration::from_millis(config.relay_backoff_max_ms),
    )?;
    let worker = RelayWorker::new(reader, publisher, policy);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let worker_run = worker.run_until_shutdown(shutdown_rx);
    tokio::pin!(worker_run);
    tokio::select! {
        result = &mut worker_run => result?,
        signal = shutdown_signal() => {
            signal?;
            shutdown_tx.send(true)?;
            worker_run.await?;
        }
    }
    database_pool.close().await;
    Ok(())
}

async fn shutdown_signal() -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}
