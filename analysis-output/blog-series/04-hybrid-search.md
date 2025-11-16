# Hybrid Search & Advanced Filtering: Beyond Simple Vector Similarity

**Series:** Qdrant Deep Dive (Post 4 of 7)

## Summary

Explores how Qdrant combines:
- **Dense vectors** (semantic similarity)
- **Sparse vectors** (keyword matching, BM25-like)
- **Payload filtering** (structured queries)
- **Query optimizer** (cost-based execution planning)

**Key Concepts:**
- Sparse vector creation and indexing
- Fusion algorithms (RRF - Reciprocal Rank Fusion)
- Filter strategies: pre-filter, post-filter, filtered-HNSW
- Field index types: keyword, numeric, geo, full-text

**Code:** [lib/sparse/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/sparse), [lib/segment/src/index/field_index/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/field_index)
