//! Streaming utilities for handling large files with constant memory usage.
//!
//! This module provides chunked reading and writing capabilities to prevent
//! loading entire files into memory, particularly useful for large snapshots.

use std::io::{self, Read, Write};
use std::path::Path;

/// Default chunk size: 64MB as specified in RFC-0007
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024 * 1024;

/// A chunked file reader that reads files in fixed-size chunks to maintain
/// constant memory usage regardless of file size.
pub struct ChunkedFileReader {
    chunk_size: usize,
}

impl ChunkedFileReader {
    /// Create a new chunked file reader with the default chunk size (64MB)
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create a new chunked file reader with a custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Stream a file in chunks, calling the provided function for each chunk.
    /// Returns the total number of bytes read.
    pub fn stream_file<F>(&self, path: &Path, mut chunk_handler: F) -> io::Result<u64>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            chunk_handler(&buffer[..bytes_read])?;
            total_bytes += bytes_read as u64;
        }

        Ok(total_bytes)
    }

    /// Stream from a reader in chunks
    pub fn stream_reader<R, F>(&self, reader: &mut R, mut chunk_handler: F) -> io::Result<u64>
    where
        R: Read,
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            chunk_handler(&buffer[..bytes_read])?;
            total_bytes += bytes_read as u64;
        }

        Ok(total_bytes)
    }
}

impl Default for ChunkedFileReader {
    fn default() -> Self {
        Self::new()
    }
}

/// A chunked file writer that writes data in chunks
pub struct ChunkedFileWriter {
    chunk_size: usize,
}

impl ChunkedFileWriter {
    /// Create a new chunked file writer with the default chunk size (64MB)
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create a new chunked file writer with a custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Write data to a writer in chunks to maintain constant memory usage
    pub fn write_in_chunks<W: Write>(&self, mut writer: W, data: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let end = std::cmp::min(offset + self.chunk_size, data.len());
            writer.write_all(&data[offset..end])?;
            offset = end;
        }
        writer.flush()?;
        Ok(())
    }

    /// Copy from reader to writer in chunks
    pub fn copy_in_chunks<R: Read, W: Write>(
        &self,
        reader: &mut R,
        writer: &mut W,
    ) -> io::Result<u64> {
        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buffer[..bytes_read])?;
            total_bytes += bytes_read as u64;
        }

        writer.flush()?;
        Ok(total_bytes)
    }
}

impl Default for ChunkedFileWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_chunked_reader_small_file() {
        let data = b"Hello, World!";
        let mut reader = Cursor::new(data);
        let chunked_reader = ChunkedFileReader::with_chunk_size(5);

        let mut chunks = Vec::new();
        let total = chunked_reader
            .stream_reader(&mut reader, |chunk| {
                chunks.push(chunk.to_vec());
                Ok(())
            })
            .unwrap();

        assert_eq!(total, 13);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], b"Hello");
        assert_eq!(chunks[1], b", Wor");
        assert_eq!(chunks[2], b"ld!");
    }

    #[test]
    fn test_chunked_writer() {
        let data = b"Hello, World! This is a test of chunked writing.";
        let mut output = Vec::new();
        let writer = ChunkedFileWriter::with_chunk_size(10);

        writer.write_in_chunks(&mut output, data).unwrap();
        assert_eq!(&output[..], data);
    }

    #[test]
    fn test_copy_in_chunks() {
        let data = b"Test data for copy in chunks functionality";
        let mut reader = Cursor::new(data);
        let mut writer = Vec::new();
        let chunked_writer = ChunkedFileWriter::with_chunk_size(10);

        let total = chunked_writer
            .copy_in_chunks(&mut reader, &mut writer)
            .unwrap();

        assert_eq!(total, data.len() as u64);
        assert_eq!(&writer[..], data);
    }
}
