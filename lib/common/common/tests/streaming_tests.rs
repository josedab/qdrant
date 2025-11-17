//! Integration tests for streaming backup/restore functionality

use std::io::Write;
use std::path::Path;

use common::streaming::{ChunkedFileReader, ChunkedFileWriter, DEFAULT_CHUNK_SIZE};
use common::tar_ext::{extract_tar_streaming, BuilderExt};
use tempfile::TempDir;

#[test]
fn test_streaming_large_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("large_test_file.bin");

    // Create a 200MB file (larger than chunk size)
    let file_size = 200 * 1024 * 1024; // 200MB
    let mut file = std::fs::File::create(&test_file).unwrap();

    // Write data in chunks to avoid using too much memory in test
    let chunk = vec![0xAB; 1024 * 1024]; // 1MB chunks
    for _ in 0..200 {
        file.write_all(&chunk).unwrap();
    }
    drop(file);

    // Verify file was created
    let metadata = std::fs::metadata(&test_file).unwrap();
    assert_eq!(metadata.len(), file_size);
}

#[test]
fn test_chunked_file_reader() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create a test file larger than chunk size
    let test_data = vec![b'X'; 10 * 1024 * 1024]; // 10MB
    std::fs::write(&test_file, &test_data).unwrap();

    // Read file in chunks
    let reader = ChunkedFileReader::with_chunk_size(1024 * 1024); // 1MB chunks
    let mut total_size = 0u64;
    let mut chunk_count = 0;

    reader
        .stream_file(&test_file, |chunk| {
            total_size += chunk.len() as u64;
            chunk_count += 1;
            Ok(())
        })
        .unwrap();

    assert_eq!(total_size, test_data.len() as u64);
    assert_eq!(chunk_count, 10); // Should be 10 chunks of 1MB each
}

#[test]
fn test_streaming_tar_creation_and_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create some test files
    let src_dir = temp_dir.path().join("source");
    std::fs::create_dir_all(&src_dir).unwrap();

    let test_file1 = src_dir.join("file1.txt");
    let test_file2 = src_dir.join("file2.txt");

    std::fs::write(&test_file1, b"Hello, World!").unwrap();
    std::fs::write(&test_file2, b"Streaming snapshots!").unwrap();

    // Create TAR archive using streaming
    let archive_file = std::fs::File::create(&archive_path).unwrap();
    let tar = BuilderExt::new_seekable_owned(archive_file);

    tar.blocking_append_file_streaming(&test_file1, Path::new("file1.txt"))
        .unwrap();
    tar.blocking_append_file_streaming(&test_file2, Path::new("file2.txt"))
        .unwrap();

    tar.blocking_finish().unwrap();

    // Extract TAR archive using streaming
    std::fs::create_dir_all(&extract_dir).unwrap();
    let archive_file = std::fs::File::open(&archive_path).unwrap();
    let mut archive = tar::Archive::new(archive_file);

    extract_tar_streaming(&mut archive, &extract_dir).unwrap();

    // Verify extracted files
    let extracted_file1 = extract_dir.join("file1.txt");
    let extracted_file2 = extract_dir.join("file2.txt");

    assert!(extracted_file1.exists());
    assert!(extracted_file2.exists());

    let content1 = std::fs::read_to_string(&extracted_file1).unwrap();
    let content2 = std::fs::read_to_string(&extracted_file2).unwrap();

    assert_eq!(content1, "Hello, World!");
    assert_eq!(content2, "Streaming snapshots!");
}

#[test]
fn test_streaming_large_file_tar() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("large.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a large test file (100MB)
    let src_dir = temp_dir.path().join("source");
    std::fs::create_dir_all(&src_dir).unwrap();

    let large_file = src_dir.join("large_file.bin");
    let file_size = 100 * 1024 * 1024; // 100MB

    // Create file with pattern that can be verified
    let mut file = std::fs::File::create(&large_file).unwrap();
    let pattern = vec![0xDE, 0xAD, 0xBE, 0xEF];
    for _ in 0..(file_size / 4) {
        file.write_all(&pattern).unwrap();
    }
    drop(file);

    // Create TAR using streaming
    let archive_file = std::fs::File::create(&archive_path).unwrap();
    let tar = BuilderExt::new_seekable_owned(archive_file);

    tar.blocking_append_file_streaming(&large_file, Path::new("large_file.bin"))
        .unwrap();
    tar.blocking_finish().unwrap();

    // Extract using streaming
    std::fs::create_dir_all(&extract_dir).unwrap();
    let archive_file = std::fs::File::open(&archive_path).unwrap();
    let mut archive = tar::Archive::new(archive_file);

    extract_tar_streaming(&mut archive, &extract_dir).unwrap();

    // Verify extracted file
    let extracted_file = extract_dir.join("large_file.bin");
    assert!(extracted_file.exists());

    let metadata = std::fs::metadata(&extracted_file).unwrap();
    assert_eq!(metadata.len(), file_size);

    // Verify content pattern (check first few bytes)
    let content = std::fs::read(&extracted_file).unwrap();
    assert_eq!(&content[0..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_chunked_writer() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.bin");

    let writer = ChunkedFileWriter::new();
    let data = vec![0x42; 150 * 1024 * 1024]; // 150MB

    let mut file = std::fs::File::create(&output_file).unwrap();
    writer.write_in_chunks(&mut file, &data).unwrap();
    drop(file);

    let metadata = std::fs::metadata(&output_file).unwrap();
    assert_eq!(metadata.len(), data.len() as u64);
}

#[test]
fn test_streaming_with_default_chunk_size() {
    // Verify default chunk size is 64MB as per RFC
    assert_eq!(DEFAULT_CHUNK_SIZE, 64 * 1024 * 1024);

    let reader = ChunkedFileReader::new();
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.bin");

    // Create file exactly 128MB (2 chunks)
    let data = vec![0xFF; 128 * 1024 * 1024];
    std::fs::write(&test_file, &data).unwrap();

    let mut chunk_count = 0;
    reader
        .stream_file(&test_file, |_chunk| {
            chunk_count += 1;
            Ok(())
        })
        .unwrap();

    // Should be exactly 2 chunks
    assert_eq!(chunk_count, 2);
}
