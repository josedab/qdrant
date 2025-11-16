# RFC-0005: Multi-Vector Per Point Support

**Status:** Draft
**Category:** Long-term
**Estimated Effort:** 30-40 dev-days

## Summary
Allow multiple named vectors per point for multi-modal search.

## Motivation
**Use cases:**
- Text + image embeddings
- Multi-lingual search
- Hierarchical embeddings

## Current Limitation
```json
{"id": 1, "vector": [...], "payload": {}}
```

## Proposed
```json
{
  "id": 1,
  "vectors": {
    "text": [...],
    "image": [...],
    "audio": [...]
  },
  "payload": {}
}
```

### Search
```json
{
  "vector": {
    "text": [...],
    "image": [...]
  },
  "fusion": "weighted_sum",
  "weights": {"text": 0.7, "image": 0.3}
}
```

## Implementation
- Extend `VectorStruct` to support named vectors
- Multiple HNSW indexes per segment
- Score fusion logic

**Breaking change:** API extension (backwards compatible)

**Effort:** 30-40 dev-days
