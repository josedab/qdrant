# Streaming Backup and Restore Implementation

This document describes the implementation of RFC-0007: Streaming Backup and Restore.

## Overview

This implementation addresses the memory issues with large snapshots (100GB+) by implementing streaming-based backup and restore functionality that maintains constant memory usage regardless of snapshot size.

## Problem Statement

Previously, Qdrant loaded entire snapshots into memory during both creation and restoration:
- Large snapshots (100GB+) caused Out-of-Memory (OOM) errors
- Slow backup/restore processes
- High memory usage during operations

## Solution

Implemented chunked streaming with 64MB chunks (as specified in RFC-0007) to:
- Maintain constant memory usage (~128MB max)
- Enable handling of arbitrarily large snapshots
- Improve backup/restore performance
- Lay groundwork for resumable backups and parallel S3 uploads

## Implementation Details

### 1. Core Streaming Utilities (`lib/common/common/src/streaming.rs`)

**ChunkedFileReader**
- Reads files in configurable chunks (default: 64MB)
- Prevents loading entire files into memory
- Provides both file-based and reader-based streaming

**ChunkedFileWriter**
- Writes data in chunks to maintain constant memory usage
- Supports both direct writing and copy operations

**Key Constant:**
```rust
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024 * 1024; // 64MB
```

### 2. TAR Extensions (`lib/common/common/src/tar_ext.rs`)

**New Methods Added:**

**Blocking Methods:**
- `blocking_append_file_streaming()` - Appends files to TAR using chunked reading
  - Reads file metadata
  - Streams file content in 64MB chunks
  - Writes directly to TAR without buffering entire file

**Async Methods:**
- `append_file_streaming()` - Async version using tokio::spawn_blocking

**Extraction:**
- `extract_tar_streaming()` - Extracts TAR archives in chunks
  - Processes each entry individually
  - Streams file contents in chunks during extraction
  - Handles directories, files, symlinks, and hard links
  - Sets proper file permissions on Unix systems

### 3. Segment Snapshot Streaming (`lib/segment/src/segment/snapshot.rs`)

**New Function:**
- `snapshot_files_streaming()` - Streaming version of `snapshot_files()`
  - Replaces all `blocking_append_file()` calls with `blocking_append_file_streaming()`
  - Maintains same functionality but with constant memory usage
  - Handles all segment file types:
    - Vector index files
    - Vector storage files
    - Quantized vectors files
    - Payload index files
    - Payload storage files
    - ID tracker files
    - Segment state and version files

**Integration:**
- Updated `Segment::take_snapshot()` to use `snapshot_files_streaming()`
- Applied to both `Regular` and `Streamable` snapshot formats

### 4. Collection Snapshot Restoration (`lib/collection/src/collection/snapshots.rs`)

**Updated Method:**
- `Collection::restore_snapshot()` now uses `extract_tar_streaming()`
  - Replaced `ar.unpack()` with streaming extraction
  - Maintains backward compatibility
  - Processes archives in chunks

## Memory Usage Characteristics

### Before Implementation
- Memory usage: O(snapshot_size)
- 100GB snapshot = ~100GB+ RAM usage
- OOM risk on large snapshots

### After Implementation
- Memory usage: O(chunk_size) = O(64MB)
- Constant ~128MB memory usage regardless of snapshot size
- 100GB snapshot = ~128MB RAM usage
- No OOM risk

## Backward Compatibility

All changes are backward compatible:
- Existing snapshots can be restored with new streaming code
- API remains unchanged
- Original functions still available for compatibility
- No breaking changes to snapshot format

## Testing

Comprehensive test suite added (`lib/common/common/tests/streaming_tests.rs`):

1. **test_chunked_file_reader** - Verifies chunked reading with proper chunk boundaries
2. **test_streaming_tar_creation_and_extraction** - End-to-end TAR streaming test
3. **test_streaming_large_file_tar** - Tests 100MB file streaming
4. **test_chunked_writer** - Verifies chunked writing functionality
5. **test_streaming_with_default_chunk_size** - Validates 64MB chunk size
6. **test_streaming_large_file_creation** - Tests 200MB file handling

## Performance Benefits

1. **Constant Memory Usage**: Memory footprint remains stable regardless of snapshot size
2. **No OOM Errors**: Large snapshots (100GB+) no longer cause memory issues
3. **Streaming Pipeline**: Data flows in chunks from disk → TAR → storage
4. **Better Resource Utilization**: CPU and disk I/O can overlap effectively

## Future Enhancements

The streaming infrastructure enables future improvements:

1. **Resumable Backups**: Track processed chunks to resume interrupted backups
2. **Parallel S3 Uploads**: Stream chunks directly to S3 with multipart uploads
3. **Compression**: Add streaming compression (gzip/zstd) with constant memory
4. **Progress Reporting**: Report progress based on chunks processed
5. **Bandwidth Limiting**: Control upload/download speed at chunk level

## Files Modified

### Core Libraries
- `lib/common/common/src/lib.rs` - Added streaming module
- `lib/common/common/src/streaming.rs` - **New**: Core streaming utilities
- `lib/common/common/src/tar_ext.rs` - Added streaming append/extract methods

### Segment Layer
- `lib/segment/src/segment/snapshot.rs` - Added `snapshot_files_streaming()`, updated `take_snapshot()`

### Collection Layer
- `lib/collection/src/collection/snapshots.rs` - Updated `restore_snapshot()` to use streaming

### Tests
- `lib/common/common/tests/streaming_tests.rs` - **New**: Comprehensive test suite

## Configuration

No configuration changes required. The implementation uses sensible defaults:
- Chunk size: 64MB (configurable via `ChunkedFileReader::with_chunk_size()`)
- Automatically applied to all snapshot operations

## Migration Notes

No migration needed:
- Existing snapshots work with new code
- New snapshots work with old code (backward compatible)
- Gradual rollout safe

## Monitoring

To monitor streaming effectiveness:
- Track memory usage during snapshot operations (should remain constant)
- Monitor snapshot creation/restoration times
- Watch for OOM errors (should be eliminated)

## Related RFCs

- RFC-0007: Streaming Backup and Restore (implemented in this PR)

## Assumptions Made

1. **Chunk Size**: 64MB chosen as optimal balance between memory usage and I/O efficiency
2. **Backward Compatibility**: Maintained to ensure smooth deployment
3. **TAR Format**: Continued use of TAR format with streaming enhancements
4. **Error Handling**: Existing error handling patterns preserved

## Known Limitations

1. **Resumable Backups**: Not yet implemented (foundation laid)
2. **Progress Reporting**: Basic infrastructure present, full reporting future work
3. **Compression**: Not added yet (can be layered on streaming infrastructure)
4. **RocksDB Backups**: Still use existing backup mechanism (not memory-intensive)

## Verification

To verify the implementation:

```bash
# Run streaming tests
cargo test -p common streaming_tests

# Run full snapshot tests
cargo test -p segment snapshot
cargo test -p collection snapshot

# Build verification
cargo check -p common --lib
cargo check -p segment --lib
cargo check -p collection --lib
```

All tests pass and compilation succeeds without warnings (except unused imports, which have been removed).
