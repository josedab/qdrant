# RFC-0003: Async Segment Optimization

**Status:** Draft
**Estimated Effort:** 20-25 dev-days
**Category:** Strategic

## Summary
Make segment optimization non-blocking to avoid search latency spikes.

## Motivation
- Segment optimization can take 10-60 seconds
- Blocks search during optimization
- P99 latency spikes

## Current Behavior
```
Search → Segment (write lock) → Block until optimization done
```

## Proposed Design
```
Search → Segment (read lock) → Use old segment
Background → Optimize → Atomic swap
```

### Implementation
1. Copy-on-write segments
2. Background optimization task
3. Atomic pointer swap when done

**Trade-off:** Temporary increased memory (old + new segment)

## Success Metrics
- P99 latency < 50ms during optimization (currently: 500ms+)
- No search blocking

**Effort:** 20-25 dev-days
