# RFC-0002: Query Explain API

**Status:** Draft
**Estimated Effort:** 15-20 dev-days
**Category:** Strategic

## Summary
Add `/explain` endpoint to show how queries are executed, helping users optimize performance.

## Motivation
Users can't see:
- Which index was used (HNSW vs full-scan)
- Filter cardinality estimates
- Why a query is slow

## Detailed Design

### API Endpoint
```http
POST /collections/{name}/points/search/explain
{
  "vector": [...],
  "filter": {...},
  "limit": 10
}
```

### Response
```json
{
  "execution_plan": {
    "strategy": "filtered_hnsw",
    "estimated_points_to_scan": 1523,
    "index_used": "hnsw_m16_ef100",
    "filter_selectivity": 0.15,
    "steps": [
      {"stage": "filter_pre_check", "cardinality": 150000},
      {"stage": "hnsw_search", "candidates_evaluated": 1523},
      {"stage": "post_filter", "results": 10}
    ]
  },
  "estimated_latency_ms": 5.2,
  "actual_latency_ms": 4.8
}
```

## Implementation
1. Add `Explainer` trait to query optimizer
2. Capture execution stats during search
3. Return structured JSON

**Code location:** `lib/segment/src/index/query_optimization/explainer.rs` (new file)

## Success Metrics
- 80% of slow query issues resolved using explain
- Users can self-diagnose performance issues

**Effort:** 15-20 dev-days
