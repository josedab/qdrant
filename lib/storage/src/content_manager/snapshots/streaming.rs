//! Streaming snapshot support
//!
//! Enables incremental snapshot uploads/downloads with resumption support

use async_trait::async_trait;
use futures::Stream;
use std::path::Path;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

pub type SnapshotChunk = Vec<u8>;
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<SnapshotChunk, std::io::Error>> + Send>>;

#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    pub total_size: u64,
    pub chunk_size: usize,
    pub checksum: String,
    pub created_at: std::time::SystemTime,
}

#[async_trait]
pub trait StreamingSnapshot {
    /// Create a streaming snapshot
    async fn create_stream(&self, path: &Path) -> Result<ChunkStream, std::io::Error>;

    /// Restore from a streaming snapshot
    async fn restore_from_stream(
        &self,
        stream: ChunkStream,
        path: &Path,
    ) -> Result<(), std::io::Error>;
}

pub struct ResumableUpload {
    snapshot_id: String,
    uploaded_chunks: Vec<usize>,
    total_chunks: usize,
}

impl ResumableUpload {
    pub fn new(snapshot_id: String, total_chunks: usize) -> Self {
        Self {
            snapshot_id,
            uploaded_chunks: Vec::new(),
            total_chunks,
        }
    }

    pub fn mark_chunk_uploaded(&mut self, chunk_index: usize) {
        self.uploaded_chunks.push(chunk_index);
    }

    pub fn get_remaining_chunks(&self) -> Vec<usize> {
        (0..self.total_chunks)
            .filter(|i| !self.uploaded_chunks.contains(i))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.uploaded_chunks.len() == self.total_chunks
    }

    pub fn progress(&self) -> f32 {
        self.uploaded_chunks.len() as f32 / self.total_chunks as f32
    }
}

pub struct StreamingSnapshotManager {
    chunk_size: usize,
}

impl StreamingSnapshotManager {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Create snapshot stream from file
    pub async fn create_snapshot_stream(
        &self,
        file_path: &Path,
    ) -> Result<ChunkStream, std::io::Error> {
        use futures::stream;
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(file_path).await?;
        let chunk_size = self.chunk_size;

        let stream = stream::unfold(
            (file, Vec::with_capacity(chunk_size)),
            move |(mut file, mut buffer)| async move {
                buffer.clear();
                buffer.resize(chunk_size, 0);

                match file.read(&mut buffer).await {
                    Ok(0) => None, // EOF
                    Ok(n) => {
                        buffer.truncate(n);
                        Some((Ok(buffer.clone()), (file, buffer)))
                    }
                    Err(e) => Some((Err(e), (file, buffer))),
                }
            },
        );

        Ok(Box::pin(stream))
    }

    /// Restore snapshot from stream
    pub async fn restore_from_stream(
        &self,
        mut stream: ChunkStream,
        output_path: &Path,
    ) -> Result<(), std::io::Error> {
        use futures::StreamExt;
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;

        let mut file = File::create(output_path).await?;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }

    /// Calculate metadata for snapshot
    pub async fn calculate_metadata(
        &self,
        file_path: &Path,
    ) -> Result<SnapshotMetadata, std::io::Error> {
        use tokio::fs;

        let metadata = fs::metadata(file_path).await?;
        let total_size = metadata.len();
        let chunk_count = (total_size + self.chunk_size as u64 - 1) / self.chunk_size as u64;

        // Simple checksum (in practice, use SHA-256)
        let checksum = format!("{:x}", total_size);

        Ok(SnapshotMetadata {
            total_size,
            chunk_size: self.chunk_size,
            checksum,
            created_at: metadata.modified()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_streaming_snapshot() {
        let manager = StreamingSnapshotManager::new(1024);

        // Create test file
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = tokio::fs::File::create(temp_file.path()).await.unwrap();
        file.write_all(b"test data for snapshot").await.unwrap();
        file.flush().await.unwrap();

        // Create stream
        let stream = manager.create_snapshot_stream(temp_file.path()).await.unwrap();

        // Restore to new file
        let output_file = NamedTempFile::new().unwrap();
        manager.restore_from_stream(stream, output_file.path()).await.unwrap();

        // Verify
        let content = tokio::fs::read(output_file.path()).await.unwrap();
        assert_eq!(content, b"test data for snapshot");
    }

    #[test]
    fn test_resumable_upload() {
        let mut upload = ResumableUpload::new("test".to_string(), 10);

        upload.mark_chunk_uploaded(0);
        upload.mark_chunk_uploaded(2);

        assert_eq!(upload.progress(), 0.2);
        assert!(!upload.is_complete());

        let remaining = upload.get_remaining_chunks();
        assert_eq!(remaining.len(), 8);
        assert!(!remaining.contains(&0));
        assert!(!remaining.contains(&2));
    }
}
