use std::collections::VecDeque;
use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Ring buffer capacity: 64KB.
pub const RING_CAPACITY: usize = 65_536;

/// Session output persistence with ring buffer + append-only file.
///
/// The ring buffer holds the most recent bytes for fast tail reads.
/// The append-only file retains all output for full history.
pub struct SessionOutput {
    /// Append-only log file.
    file: File,
    /// In-memory ring buffer for fast tail access.
    ring: VecDeque<u8>,
    /// Total bytes written since session start.
    total_bytes: u64,
}

impl SessionOutput {
    /// Create a new SessionOutput with file at the given path.
    ///
    /// Creates parent directories if they do not exist.
    /// Opens the file in append+create mode.
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self {
            file,
            ring: VecDeque::new(),
            total_bytes: 0,
        })
    }

    /// Append data to both the file and the ring buffer.
    ///
    /// Evicts oldest bytes from the ring buffer if capacity is exceeded.
    pub async fn append(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.file.write_all(data).await?;
        self.file.flush().await?;

        for &byte in data {
            if self.ring.len() >= RING_CAPACITY {
                self.ring.pop_front();
            }
            self.ring.push_back(byte);
        }
        self.total_bytes += data.len() as u64;
        Ok(())
    }

    /// Return the current ring buffer contents as a contiguous Vec.
    pub fn tail(&self) -> Vec<u8> {
        self.ring.iter().copied().collect()
    }

    /// Return the total number of bytes written since session start.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Read bytes from an append-only log file starting at the given offset.
    /// Returns up to `limit` bytes. Returns empty vec if offset >= file_size.
    pub async fn read_range(
        path: &std::path::Path,
        offset: u64,
        limit: usize,
    ) -> anyhow::Result<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let mut file = tokio::fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();
        if offset >= file_size {
            return Ok(vec![]);
        }
        let available = (file_size - offset) as usize;
        let read_size = available.min(limit);
        let mut buf = vec![0u8; read_size];
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.read_exact(&mut buf).await?;
        Ok(buf)
    }
}
