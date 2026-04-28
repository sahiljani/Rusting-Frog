// Per-crawl JSON Lines debug logger.
//
// Each line is one event ({"t": ..., "kind": "phase"|"sample", ...}).
// The active file rotates at 5MB to <id>.log.1, then .2; older content
// is dropped. If the directory cannot be created or writes start failing
// (disk full, EACCES, etc.) the logger silently disables itself for the
// remainder of the crawl — debug logging must never crash a scrape.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Offset, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::{Value, json};

const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const ROTATED_FILES: usize = 2;
const VICTORIA_TZ: Tz = chrono_tz::America::Vancouver;

/// Default location used when `SF_DEBUG_LOG_DIR` is not set. Matches the
/// shared volume mounted in docker-compose.
pub const DEFAULT_DEBUG_LOG_DIR: &str = "/var/log/sf-debug";

#[derive(Clone)]
pub struct DebugLogger {
    inner: Arc<Mutex<Option<Inner>>>,
    crawl_id: String,
}

struct Inner {
    path: PathBuf,
    file: File,
    bytes_written: u64,
}

impl DebugLogger {
    /// Create a logger writing to `<dir>/<crawl_id>.log`. If the directory
    /// can't be created the logger is returned in a disabled state — call
    /// sites do not need to error-handle.
    pub fn new(dir: &Path, crawl_id: &str) -> Self {
        let inner = match Self::open(dir, crawl_id) {
            Ok(i) => Some(i),
            Err(e) => {
                tracing::warn!(error = %e, dir = %dir.display(), "debug logger disabled");
                None
            }
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            crawl_id: crawl_id.to_string(),
        }
    }

    fn open(dir: &Path, crawl_id: &str) -> std::io::Result<Inner> {
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("{crawl_id}.log"));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let bytes_written = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Inner {
            path,
            file,
            bytes_written,
        })
    }

    pub fn crawl_id(&self) -> &str {
        &self.crawl_id
    }

    /// Emit a `phase` event. `meta` is an arbitrary JSON object; pass
    /// `Value::Null` if there's nothing extra.
    pub fn phase(&self, phase: &str, ms: u64, ok: bool, meta: Value) {
        let line = json!({
            "t": victoria_now(),
            "kind": "phase",
            "crawl_id": &self.crawl_id,
            "phase": phase,
            "ms": ms,
            "ok": ok,
            "meta": meta,
        });
        self.write_line(line);
    }

    /// Emit a `phase` event also tied to a specific URL. Used by the
    /// per-URL phases (fetch / parse / evaluators / db_write).
    pub fn phase_url(&self, phase: &str, url: &str, ms: u64, ok: bool, meta: Value) {
        let line = json!({
            "t": victoria_now(),
            "kind": "phase",
            "crawl_id": &self.crawl_id,
            "phase": phase,
            "url": url,
            "ms": ms,
            "ok": ok,
            "meta": meta,
        });
        self.write_line(line);
    }

    /// Emit a free-form log line. Carried in the same channel so the
    /// debug page shows a unified timeline.
    pub fn log(&self, level: &str, message: &str, meta: Value) {
        let line = json!({
            "t": victoria_now(),
            "kind": "log",
            "crawl_id": &self.crawl_id,
            "level": level,
            "message": message,
            "meta": meta,
        });
        self.write_line(line);
    }

    /// Emit a 1-second resource sample.
    pub fn sample(&self, sample: &SampleEvent) {
        let line = serde_json::to_value(sample).unwrap_or(Value::Null);
        // SampleEvent serializes its own t/kind/crawl_id, but the helper
        // expects a complete JSON object — write_line just appends '\n'.
        self.write_line(line);
    }

    fn write_line(&self, mut value: Value) {
        // Ensure crawl_id is present even on caller-provided values.
        if let Some(obj) = value.as_object_mut()
            && !obj.contains_key("crawl_id")
        {
            obj.insert("crawl_id".to_string(), Value::String(self.crawl_id.clone()));
        }
        let mut buf = match serde_json::to_vec(&value) {
            Ok(b) => b,
            Err(_) => return,
        };
        buf.push(b'\n');

        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return, // poisoned — give up silently
        };
        let inner = match guard.as_mut() {
            Some(i) => i,
            None => return, // disabled
        };

        if inner.bytes_written + buf.len() as u64 > MAX_FILE_BYTES
            && let Err(e) = rotate(inner)
        {
            tracing::warn!(error = %e, "debug log rotation failed; disabling");
            *guard = None;
            return;
        }

        if let Err(e) = inner.file.write_all(&buf) {
            tracing::warn!(error = %e, "debug log write failed; disabling");
            *guard = None;
            return;
        }
        // Each write is a complete event — flush so the API tail reader
        // sees it on the next poll without OS buffering hiding it.
        let _ = inner.file.flush();
        inner.bytes_written += buf.len() as u64;
    }
}

fn rotate(inner: &mut Inner) -> std::io::Result<()> {
    let base = &inner.path;
    // Drop the oldest, slide down the chain, swap in a fresh active file.
    for i in (1..=ROTATED_FILES).rev() {
        let dst = with_suffix(base, i);
        if i == ROTATED_FILES && dst.exists() {
            fs::remove_file(&dst)?;
        }
        let src = if i == 1 {
            base.clone()
        } else {
            with_suffix(base, i - 1)
        };
        if src.exists() {
            // remove dst first if it still exists from the previous iteration
            if dst.exists() {
                fs::remove_file(&dst)?;
            }
            fs::rename(&src, &dst)?;
        }
    }
    let new_file = OpenOptions::new().create(true).append(true).open(base)?;
    inner.file = new_file;
    inner.bytes_written = 0;
    Ok(())
}

fn with_suffix(base: &Path, n: usize) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(format!(".{n}"));
    PathBuf::from(s)
}

/// Current time formatted as ISO-8601 with the Victoria BC offset
/// (PDT/PST). DST is handled by chrono-tz; we then convert to a
/// FixedOffset so the formatter prints `-07:00`/`-08:00` rather than the
/// IANA name.
fn victoria_now() -> String {
    let utc: DateTime<Utc> = Utc::now();
    let pt = utc.with_timezone(&VICTORIA_TZ);
    let offset = pt.offset().fix();
    pt.with_timezone(&offset)
        .format("%Y-%m-%dT%H:%M:%S%.3f%:z")
        .to_string()
}

/// Resource sample emitted every second by the sampler task. Numbers in
/// bytes; counters from the pipeline are passed in.
#[derive(Debug, Serialize)]
pub struct SampleEvent {
    pub t: String,
    pub kind: &'static str,
    pub crawl_id: String,
    pub worker_rss_bytes: u64,
    pub host_mem_used_bytes: u64,
    pub host_mem_total_bytes: u64,
    pub host_disk_used_bytes: u64,
    pub host_disk_total_bytes: u64,
    pub pg_db_bytes: Option<i64>,
    pub urls_done: i64,
    pub urls_queued: i64,
}

impl SampleEvent {
    pub fn new(
        crawl_id: &str,
        worker_rss_bytes: u64,
        host_mem_used_bytes: u64,
        host_mem_total_bytes: u64,
        host_disk_used_bytes: u64,
        host_disk_total_bytes: u64,
        pg_db_bytes: Option<i64>,
        urls_done: i64,
        urls_queued: i64,
    ) -> Self {
        Self {
            t: victoria_now(),
            kind: "sample",
            crawl_id: crawl_id.to_string(),
            worker_rss_bytes,
            host_mem_used_bytes,
            host_mem_total_bytes,
            host_disk_used_bytes,
            host_disk_total_bytes,
            pg_db_bytes,
            urls_done,
            urls_queued,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_jsonl_and_rotates_at_size_cap() {
        let dir = tempdir().unwrap();
        let logger = DebugLogger::new(dir.path(), "test-crawl");

        // Each line is a JSON object terminated by '\n'.
        logger.phase("fetch", 100, true, json!({"status": 200}));
        let active = dir.path().join("test-crawl.log");
        let body = fs::read_to_string(&active).unwrap();
        assert!(body.ends_with('\n'));
        let val: Value = serde_json::from_str(body.trim()).unwrap();
        assert_eq!(val["kind"], "phase");
        assert_eq!(val["phase"], "fetch");
        assert_eq!(val["ms"], 100);
        assert_eq!(val["crawl_id"], "test-crawl");

        // Force rotation by writing >5MB of payload.
        let blob = "x".repeat(10 * 1024);
        for _ in 0..600 {
            logger.log("info", &blob, json!({}));
        }
        assert!(dir.path().join("test-crawl.log.1").exists());
    }

    #[test]
    fn disabled_logger_is_silent_on_unwritable_dir() {
        // A path with a NUL byte is invalid on every OS — open() will fail
        // and the logger should fall back to disabled mode without panicking.
        let bad = Path::new("\0/definitely/not/a/dir");
        let logger = DebugLogger::new(bad, "x");
        logger.phase("noop", 1, true, json!({})); // must not panic
    }
}
