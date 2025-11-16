# RFC-0007: Streaming Backup and Restore

**Status:** Draft
**Category:** Strategic
**Estimated Effort:** 12-18 dev-days

## Summary
Stream snapshots instead of loading entirely in memory.

## Motivation
- Large snapshots (100GB+) OOM during restore
- Slow backup/restore process
- High memory usage

## Proposed Design
```rust
async fn stream_snapshot(path: &Path) -> impl Stream<Item = SnapshotChunk> {
    // Stream 64MB chunks
}
```

### Benefits
- Constant memory usage
- Resume partial backups
- Parallel upload to S3

**Effort:** 12-18 dev-days
