//! Retention, thumbnailing, cleanup.

use std::sync::Arc;
use std::time::Duration;

use crate::app::Store;

/// Runs `clip-store`'s retention pruning on a recurring interval, until the
/// task is cancelled. A failed run is logged and does not stop later
/// scheduled runs; a missing retention window makes each run a no-op.
pub async fn run_retention_job(store: Arc<dyn Store>, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let retention_days = match store.get_settings() {
            Ok(settings) => settings.retention_window_days,
            Err(err) => {
                tracing::warn!(error = %err, "failed to read retention settings for scheduled prune");
                continue;
            }
        };
        if let Err(err) = store.prune(retention_days) {
            tracing::warn!(error = %err, "scheduled retention prune failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_retention_job;
    use crate::app::fakes::FakeStore;
    use crate::app::Store;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn prune_is_invoked_again_after_one_interval_elapses() {
        let store = Arc::new(FakeStore::new());
        let interval = Duration::from_secs(60);
        tokio::spawn(run_retention_job(store.clone(), interval));

        tokio::time::advance(interval).await;
        tokio::task::yield_now().await;
        let first = store.prune_call_count();
        assert!(first >= 1);

        tokio::time::advance(interval).await;
        tokio::task::yield_now().await;
        assert!(store.prune_call_count() > first);
    }

    #[tokio::test(start_paused = true)]
    async fn one_failed_prune_run_does_not_stop_later_runs() {
        let store = Arc::new(FakeStore::new());
        store.fail_next_prune();
        let interval = Duration::from_secs(60);
        tokio::spawn(run_retention_job(store.clone(), interval));

        tokio::time::advance(interval).await;
        tokio::task::yield_now().await;
        let after_failure = store.prune_call_count();
        assert!(after_failure >= 1);

        tokio::time::advance(interval).await;
        tokio::task::yield_now().await;
        assert!(store.prune_call_count() > after_failure, "the scheduler should still run after a failed prune");
    }

    #[tokio::test(start_paused = true)]
    async fn a_scheduled_run_with_no_retention_window_deletes_nothing() {
        let store = Arc::new(FakeStore::new());
        let interval = Duration::from_secs(60);
        tokio::spawn(run_retention_job(store.clone(), interval));

        tokio::time::advance(interval).await;
        tokio::task::yield_now().await;

        assert!(store.prune_call_count() >= 1);
        assert!(store.search("", &Default::default()).unwrap().is_empty());
    }
}
