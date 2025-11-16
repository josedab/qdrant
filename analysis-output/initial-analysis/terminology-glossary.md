# Qdrant Terminology Glossary

**Analysis Baseline:** Commit `adcda004057df08389106da56f440db185f0c382`

This glossary defines project-specific terms, concepts, and acronyms used throughout the Qdrant codebase and documentation.

---

## Core Concepts

### **Point**
An individual data entity in Qdrant consisting of:
- **ID** (`PointIdType`): Unique identifier (u64 or UUID)
- **Vector** (`Vector`): High-dimensional embedding (dense or sparse)
- **Payload** (`Payload`): Arbitrary JSON metadata attached to the point

**Example:**
```rust
Point {
    id: 123,
    vector: [0.1, 0.2, 0.3, ...],  // 384-dimensional
    payload: { "title": "...", "category": "tech" }
}
```

**Related:** `lib/segment/src/types.rs`

---

### **Collection**
A named set of points with the same vector configuration. Collections are:
- **Sharded** across nodes for horizontal scaling
- **Replicated** for high availability
- **Indexed** for fast search

**Key Properties:**
- Vector dimension
- Distance metric (Cosine, Euclidean, etc.)
- Shard count
- Replication factor

**Related:** `lib/collection/`

---

### **Segment**
An immutable, self-contained unit of vector storage within a shard. Contains:
- **Vector data** (dense, sparse, or quantized)
- **Payload data** (on-disk or in-memory)
- **Indexes** (HNSW for vectors, field indexes for payloads)
- **ID tracker** (maps point IDs to internal IDs)

**Lifecycle:**
1. Mutable segment receives writes
2. When full, optimized into immutable segment
3. Periodically merged/compacted

**Related:** `lib/segment/`

---

### **Shard**
A logical partition of a collection distributed across the cluster.

**Types:**
- **Local Shard**: Stored on the current node
- **Remote Shard**: Proxy to shard on another node
- **Replica**: Copy of a shard for high availability

**Related:** `lib/shard/`, `lib/collection/src/shards/`

---

## Vector & Search Concepts

### **Dense Vector**
Traditional vector representation with a fixed dimension where every component has a value.

**Example:** `[0.1, 0.2, 0.3, ..., 0.5]` (384-dim)

**Storage:** `lib/segment/src/vector_storage/dense/`

---

### **Sparse Vector**
Vector representation where most components are zero, stored as `{index: value}` pairs.

**Example:** `{5: 0.8, 42: 1.2, 103: 0.5}` (only 3 non-zero out of 10,000 dims)

**Use Case:** Keyword search, BM25-like ranking

**Storage:** `lib/segment/src/vector_storage/sparse/`

---

### **HNSW (Hierarchical Navigable Small World)**
A graph-based approximate nearest neighbor (ANN) search algorithm.

**Key Properties:**
- **M**: Number of connections per node in the graph
- **ef_construct**: Size of dynamic candidate list during construction
- **ef** (search-time): Size of candidate list during search
- **Layers**: Multi-layer graph for logarithmic search complexity

**Trade-off:** Sacrifices perfect recall for speed (O(log N) vs O(N))

**Related:** `lib/segment/src/index/hnsw_index/`

---

### **Distance Metrics**
Functions to measure similarity/dissimilarity between vectors.

| Metric | Formula | Use Case | Range |
|--------|---------|----------|-------|
| **Cosine** | 1 - (A·B)/(‖A‖‖B‖) | Semantic similarity | [0, 2] |
| **Euclidean (L2)** | ‖A - B‖₂ | Geometric distance | [0, ∞) |
| **Dot Product** | A · B | Ranking, similarity | (-∞, ∞) |
| **Manhattan (L1)** | ‖A - B‖₁ | Taxi-cab distance | [0, ∞) |

**Related:** `lib/segment/src/spaces/`

---

### **Quantization**
Compression technique that reduces vector precision to save memory.

**Types:**
1. **Scalar Quantization**: Map float32 → int8 (4x compression)
2. **Product Quantization (PQ)**: Subdivide vector, cluster each part (8-32x)
3. **Binary Quantization**: Map to binary (32x compression)

**Trade-off:** Memory savings vs search accuracy

**Related:** `lib/quantization/`

---

## Filtering & Indexing

### **Payload**
JSON metadata attached to each point, supporting:
- **Primitive types**: string, integer, float, boolean, UUID
- **Geo types**: `geo_point`, `geo_polygon`
- **Date/time**: RFC 3339 timestamps
- **Nested objects**: Arbitrary depth
- **Arrays**: Multi-value fields

**Example:**
```json
{
  "title": "Document",
  "price": 19.99,
  "tags": ["rust", "vector"],
  "location": { "lat": 51.5, "lon": -0.1 },
  "published": "2024-01-15T10:30:00Z"
}
```

**Related:** `lib/segment/src/payload_storage/`

---

### **Field Index**
Specialized indexes on payload fields for fast filtering.

**Types:**
| Index Type | Field Type | Use Case |
|------------|------------|----------|
| **Keyword** | String | Exact match, category filtering |
| **Integer** | i64 | Range queries, numerical filtering |
| **Float** | f64 | Range queries |
| **Geo** | `geo_point` | Radius/polygon geo queries |
| **Text** | String | Full-text search with tokenization |
| **DateTime** | Timestamp | Time-range queries |
| **UUID** | UUID | Unique identifier filtering |
| **Bool** | Boolean | Binary filtering |

**Related:** `lib/segment/src/index/field_index/`

---

### **Filter**
Conditions to restrict search results.

**Operators:**
- **must**: AND logic (all conditions must match)
- **should**: OR logic (at least one must match)
- **must_not**: NOT logic (none must match)

**Example:**
```json
{
  "must": [
    { "key": "price", "range": { "lt": 100 } },
    { "key": "category", "match": { "value": "electronics" } }
  ]
}
```

**Related:** `lib/segment/src/types.rs` (Filter types)

---

## Distributed System Concepts

### **Raft Consensus**
Algorithm for maintaining consistent state across distributed nodes.

**Key Roles:**
- **Leader**: Handles all writes, replicates to followers
- **Follower**: Passively receives log entries from leader
- **Candidate**: Temporary role during leader election

**Guarantees:** Strong consistency, linearizability

**Related:** `src/consensus.rs`, `lib/storage/src/content_manager/consensus/`

---

### **Write-Ahead Log (WAL)**
Durable log of all operations before they're applied to segments.

**Purpose:**
- **Durability**: Survive crashes
- **Replication**: Sync shards across nodes
- **Recovery**: Replay operations after restart

**Format:** Append-only, sequential writes

**Related:** `lib/collection/src/wal_delta.rs`

---

### **Replication Factor**
Number of copies of each shard maintained across the cluster.

**Example:** `replication_factor: 3` → Each shard exists on 3 nodes

**Trade-off:** Availability/durability vs storage cost

---

### **Write Consistency Factor**
Minimum number of replicas that must acknowledge a write.

**Example:** `write_consistency_factor: 2` → Wait for 2 replicas

**Trade-off:** Durability guarantees vs latency

---

## Storage & Persistence

### **RocksDB**
Embedded key-value store used for:
- Segment metadata
- Payload data (when `on_disk_payload: true`)
- WAL entries
- Consensus state

**Features:** LSM-tree, compression, compaction

**Related:** rocksdb dependency

---

### **Memory-Mapped (mmap) Files**
Technique to map files directly into process address space.

**Benefits:**
- Zero-copy reads
- OS-managed caching
- Efficient for large datasets

**Trade-off:** Less predictable memory usage

**Related:** `lib/segment/src/vector_storage/*/mmap_*`

---

### **Snapshot**
Point-in-time backup of a collection or entire database.

**Types:**
- **Collection snapshot**: Single collection
- **Full snapshot**: Entire Qdrant instance

**Format:** Compressed tarball with all segments + metadata

**Related:** `lib/storage/src/content_manager/snapshots/`

---

## Performance & Optimization

### **Query Optimizer**
Component that selects the best execution plan for searches with filters.

**Strategies:**
- **Full scan**: No index, iterate all points
- **Filtered HNSW**: Apply filter during graph traversal
- **Pre-filter**: Use field index first, then vector search
- **Post-filter**: Vector search first, filter results

**Decision based on:** Filter selectivity, cardinality estimation

**Related:** `lib/segment/src/index/query_optimization/`

---

### **Cardinality Estimation**
Estimate of number of points matching a filter condition.

**Used for:** Query optimization, index selection

**Methods:**
- Exact (for indexed fields)
- Estimated (for non-indexed fields)

---

### **Optimizer (Segment Optimizer)**
Background process that optimizes segment structure.

**Tasks:**
- Merge small segments
- Rebuild HNSW indexes
- Vacuum deleted points
- Build quantized vectors

**Triggers:**
- Deleted vector threshold exceeded
- Segment count too high
- Manual trigger

**Related:** `config.yaml` (`optimizers` section)

---

## API & Integration

### **REST API**
HTTP-based JSON API (port 6333).

**Framework:** Actix-web

**Endpoints:** Collections, Points, Search, Recommend, Query, Cluster, Snapshots

**Related:** `src/actix/api/`

---

### **gRPC API**
Binary protocol buffer API (port 6334).

**Framework:** Tonic

**Advantages:** Lower latency, better throughput, streaming

**Related:** `src/tonic/api/`, `lib/api/src/grpc/proto/`

---

### **OpenAPI**
Machine-readable API specification (Swagger/OpenAPI 3.0).

**Purpose:** Client generation, documentation

**Location:** `openapi/openapi-merged.json`

---

## Security & Access Control

### **JWT (JSON Web Token)**
Token-based authentication mechanism.

**Usage:** Set `api_key` or use JWT RBAC

**Header:** `api-key: <token>`

**Related:** `src/actix/auth.rs`, `src/tonic/auth.rs`

---

### **RBAC (Role-Based Access Control)**
Fine-grained access control framework.

**Features:**
- Per-collection permissions
- Role definitions
- JWT-based enforcement

**Status:** Framework implemented, extensible

**Related:** `lib/storage/src/rbac/`

---

### **TLS/SSL**
Transport Layer Security for encrypted communication.

**Implementation:** Rustls (memory-safe Rust TLS)

**Endpoints:** REST, gRPC, P2P cluster communication

**Configuration:** `tls` section in config.yaml

---

## Operations & Monitoring

### **Health Status**
Collection/cluster health indicator.

**States:**
- **Green**: All replicas operational
- **Yellow**: Degraded (some replicas down)
- **Red**: Critical (no operational replicas)

**Related:** `/health` endpoint

---

### **Telemetry**
Anonymous usage statistics sent to Qdrant team (opt-out available).

**Data:** Version, OS, collection count (no user data)

**Configuration:** `telemetry_disabled: true` to opt-out

---

### **Prometheus Metrics**
Time-series metrics exposed at `/metrics` endpoint.

**Metrics:**
- Request counts
- Latencies (P50, P95, P99)
- Collection statistics
- System resources

**Related:** `src/common/metrics.rs`

---

## Data Types & Formats

### **PointIdType**
Unique identifier for points.

**Types:**
- `PointIdNum` (u64): Numeric ID
- `PointIdUuid` (UUID): UUID string

**Related:** `lib/segment/src/types.rs`

---

### **SeqNumberType**
Sequential number for tracking modifications (u64).

**Purpose:** Versioning, ordering, conflict resolution

---

### **ScoreType**
Floating-point score for search results (f32).

**Interpretation:** Lower is better for distances, higher for similarities

---

## Acronyms & Abbreviations

| Acronym | Full Form | Meaning |
|---------|-----------|---------|
| **ANN** | Approximate Nearest Neighbor | Fast similarity search |
| **HNSW** | Hierarchical Navigable Small World | Graph-based ANN algorithm |
| **WAL** | Write-Ahead Log | Durability mechanism |
| **LSM** | Log-Structured Merge-tree | RocksDB storage model |
| **PQ** | Product Quantization | Vector compression technique |
| **SIMD** | Single Instruction Multiple Data | CPU parallelism |
| **LTO** | Link-Time Optimization | Compiler optimization |
| **RBAC** | Role-Based Access Control | Security framework |
| **TOC** | Table of Contents | Collection registry |
| **FP16** | 16-bit Floating Point | Half-precision floats |
| **FP32** | 32-bit Floating Point | Single-precision floats |
| **KNN** | K-Nearest Neighbors | Search for K similar items |
| **BM25** | Best Matching 25 | Ranking function (sparse) |

---

## Common Variable Names & Patterns

| Name | Meaning |
|------|---------|
| `toc` | Table of Contents (collection registry) |
| `shard_key` | Identifier for a shard within a collection |
| `replica_set` | Set of shard replicas |
| `local_shard` | Shard stored locally |
| `remote_shard` | Shard on another node |
| `payload_index` | Index on payload field |
| `vector_storage` | Storage for vector data |
| `id_tracker` | Mapping point IDs to internal IDs |
| `scorer` | Component that scores vector similarity |
| `query_context` | Search context with filters |

---

## File Extensions & Formats

| Extension | Format | Purpose |
|-----------|--------|---------|
| `.rs` | Rust source | Source code |
| `.proto` | Protocol Buffer | gRPC API definitions |
| `.yaml` | YAML | Configuration files |
| `.json` | JSON | OpenAPI specs, payloads |
| `.sh` | Shell script | Integration tests |
| `.toml` | TOML | Cargo manifest, rustfmt config |

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `QDRANT_BOOTSTRAP` | Bootstrap peer URI | None |
| `QDRANT_URI` | This peer's URI | None |
| `QDRANT_LOG_LEVEL` | Log level | INFO |
| `RUST_LOG` | Rust logging config | None |

---

## References & Further Reading

- **HNSW Paper**: "Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs" (Malkov & Yashunin, 2018)
- **Raft Paper**: "In Search of an Understandable Consensus Algorithm" (Ongaro & Ousterhout, 2014)
- **Product Quantization**: "Product Quantization for Nearest Neighbor Search" (Jégou et al., 2011)
- **RocksDB**: https://rocksdb.org/
- **Qdrant Docs**: https://qdrant.tech/documentation/

---

**Analysis Baseline:** Commit [`adcda004057df08389106da56f440db185f0c382`](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)
