use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::Serialize;

/// Snapshot of process resource usage at a point in time.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    /// CPU usage as a percentage (0.0 - 100.0+), computed as delta between readings.
    pub cpu_percent: f32,
    /// Resident set size in bytes.
    pub rss_bytes: u64,
    /// Seconds since the session was created.
    pub uptime_secs: f64,
}

/// Tracks previous CPU reading for delta calculation.
///
/// Reads /proc/{pid}/stat and /proc/{pid}/statm via the procfs crate.
/// Returns `None` when the process no longer exists or /proc is inaccessible.
pub struct MetricsCollector {
    pid: i32,
    prev_cpu_ticks: u64,
    prev_wall_time: Instant,
    session_start: Instant,
    warned: AtomicBool,
}

impl MetricsCollector {
    /// Create a new MetricsCollector for the given PID.
    pub fn new(pid: u32, session_start: Instant) -> Self {
        Self {
            pid: pid as i32,
            prev_cpu_ticks: 0,
            prev_wall_time: Instant::now(),
            session_start,
            warned: AtomicBool::new(false),
        }
    }

    /// Collect current resource metrics for the tracked process.
    ///
    /// Returns `None` if the process no longer exists or /proc is inaccessible.
    /// Logs a warning once (not on every failure) to avoid log spam.
    pub fn collect(&mut self) -> Option<MetricsSnapshot> {
        let proc = match procfs::process::Process::new(self.pid) {
            Ok(p) => p,
            Err(_) => {
                if !self.warned.load(Ordering::Relaxed) {
                    tracing::warn!(pid = self.pid, "Cannot read /proc for process");
                    self.warned.store(true, Ordering::Relaxed);
                }
                return None;
            }
        };

        let stat = proc.stat().ok()?;
        let statm = proc.statm().ok()?;

        // CPU ticks: user + system time
        let current_cpu_ticks = stat.utime + stat.stime;
        let delta_ticks = current_cpu_ticks.saturating_sub(self.prev_cpu_ticks);

        // Wall time elapsed in ticks
        let elapsed_secs = self.prev_wall_time.elapsed().as_secs_f64();
        let ticks_per_sec = procfs::ticks_per_second() as f64;
        let elapsed_ticks = elapsed_secs * ticks_per_sec;

        // CPU% as delta
        let cpu_percent = if elapsed_ticks > 0.0 {
            (delta_ticks as f32 / elapsed_ticks as f32) * 100.0
        } else {
            0.0
        };

        // Update previous state for next delta
        self.prev_cpu_ticks = current_cpu_ticks;
        self.prev_wall_time = Instant::now();

        // RSS in bytes: resident pages * page size
        let page_size = procfs::page_size();
        let rss_bytes = statm.resident * page_size;

        // Uptime since session start
        let uptime_secs = self.session_start.elapsed().as_secs_f64();

        Some(MetricsSnapshot {
            cpu_percent,
            rss_bytes,
            uptime_secs,
        })
    }
}
