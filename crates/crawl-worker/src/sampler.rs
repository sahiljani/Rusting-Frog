// 1-second resource sampler. Spawned per-crawl, exits when the
// CancellationToken (modelled here as an Arc<AtomicBool>) is set.
//
// Reads worker process RSS, host RAM, host disk for the volume holding
// the debug log, and Postgres database size. Counters (urls_done,
// urls_queued) are read from atomics shared with the pipeline.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use sqlx::PgPool;
use sysinfo::{Disks, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

use crate::debug_log::{DebugLogger, SampleEvent};

/// Counters shared between the pipeline (writer) and sampler (reader).
/// Kept dead-simple — atomics not a Mutex, since the sampler reads at 1Hz
/// and the pipeline updates very frequently. Stale reads are fine.
#[derive(Debug, Default)]
pub struct Counters {
    pub urls_done: AtomicI64,
    pub urls_queued: AtomicI64,
}

/// Handle for stopping the sampler. Drop = stop. Calling stop() also stops.
pub struct SamplerHandle {
    cancel: Arc<AtomicBool>,
}

impl SamplerHandle {
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for SamplerHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Spawn the sampler. Returns a handle whose Drop stops it.
///
/// `disk_path` is used to pick which mount we report disk usage for —
/// caller passes the debug log dir so the user sees free space on the
/// volume that actually matters for log rotation.
pub fn spawn(
    logger: DebugLogger,
    db: PgPool,
    counters: Arc<Counters>,
    disk_path: PathBuf,
) -> SamplerHandle {
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_task = cancel.clone();

    tokio::spawn(async move {
        let pid = Pid::from_u32(std::process::id());
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_memory(sysinfo::MemoryRefreshKind::everything())
                .with_processes(ProcessRefreshKind::new().with_memory()),
        );
        let mut disks = Disks::new_with_refreshed_list();

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        // Skip the immediate tick — first sample lands at t+1s.
        interval.tick().await;

        while !cancel_task.load(Ordering::Relaxed) {
            interval.tick().await;
            if cancel_task.load(Ordering::Relaxed) {
                break;
            }

            sys.refresh_memory();
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                true,
                ProcessRefreshKind::new().with_memory(),
            );
            disks.refresh();

            let worker_rss = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

            let host_mem_total = sys.total_memory();
            let host_mem_used = sys.used_memory();

            let (host_disk_used, host_disk_total) = disk_for_path(&disks, &disk_path);

            // Non-macro form: pg_database_size isn't in the workspace's
            // offline sqlx cache and adding it would require a live DB
            // connection at build time. The query is trivial and run-once
            // per second, so the unchecked form is fine.
            let pg_db_bytes: Option<i64> =
                sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())::BIGINT")
                    .fetch_one(&db)
                    .await
                    .ok();

            let urls_done = counters.urls_done.load(Ordering::Relaxed);
            let urls_queued = counters.urls_queued.load(Ordering::Relaxed);

            let sample = SampleEvent::new(
                logger.crawl_id(),
                worker_rss,
                host_mem_used,
                host_mem_total,
                host_disk_used,
                host_disk_total,
                pg_db_bytes,
                urls_done,
                urls_queued,
            );
            logger.sample(&sample);
        }
    });

    SamplerHandle { cancel }
}

/// Pick the disk whose mount point is the longest prefix of `path`. That
/// way `/var/log/sf-debug` correctly reports the volume that holds it
/// rather than `/`. Falls back to the first disk if no match.
fn disk_for_path(disks: &Disks, path: &std::path::Path) -> (u64, u64) {
    let mut best: Option<(usize, &sysinfo::Disk)> = None;
    for d in disks.list() {
        let mp = d.mount_point();
        if path.starts_with(mp) {
            let len = mp.as_os_str().len();
            if best.map(|(l, _)| len > l).unwrap_or(true) {
                best = Some((len, d));
            }
        }
    }
    let chosen = best.map(|(_, d)| d).or_else(|| disks.list().first());
    match chosen {
        Some(d) => {
            let total = d.total_space();
            let avail = d.available_space();
            (total.saturating_sub(avail), total)
        }
        None => (0, 0),
    }
}
