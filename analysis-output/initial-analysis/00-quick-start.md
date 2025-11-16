# Qdrant Codebase Analysis: Quick Start Guide

**Analysis Date:** 2025-11-16
**Commit SHA:** `adcda004057df08389106da56f440db185f0c382`
**Qdrant Version:** 1.15.5
**Analyst:** Claude (Sonnet 4.5)

---

## 📖 Read This First

This document provides a high-level overview of the Qdrant codebase analysis. For detailed information, consult the specific analysis documents in this directory.

## 🎯 What is Qdrant?

**Qdrant** is a high-performance, production-ready **vector similarity search engine and vector database** written in Rust. It enables efficient storage, search, and management of high-dimensional vectors (embeddings) with rich metadata attached to each vector.

### Key Value Proposition
- **Vector Search Engine**: Find K-nearest neighbors in high-dimensional space
- **Semantic Search**: Beyond keyword matching using neural network embeddings
- **Hybrid Search**: Combine dense and sparse vectors for better relevance
- **Distributed by Design**: Horizontal scaling via sharding and replication
- **Production-Grade**: Enterprise features like WAL, snapshots, consensus, and RBAC

---

## 📊 Codebase at a Glance

| Metric | Value |
|--------|-------|
| **Primary Language** | Rust (100%) |
| **Rust Version** | 1.89+ (Edition 2024) |
| **Total LOC** | ~279,000 lines |
| **Source Files** | 969 Rust files |
| **Workspace Crates** | 19 modular libraries |
| **Architecture** | Layered + Modular Workspace |
| **License** | Apache 2.0 |

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────┐
│     API Layer (REST + gRPC)                 │
│  - Actix-web (REST)                         │
│  - Tonic (gRPC)                             │
├─────────────────────────────────────────────┤
│     Storage Layer                           │
│  - Collection Management                    │
│  - Shard Distribution                       │
│  - Consensus (Raft)                         │
├─────────────────────────────────────────────┤
│     Segment Layer                           │
│  - Vector Storage (dense/sparse/quantized)  │
│  - Payload Storage                          │
│  - ID Tracking                              │
├─────────────────────────────────────────────┤
│     Index Layer                             │
│  - HNSW Graph (ANN search)                  │
│  - Field Indexes (filtering)                │
│  - Query Optimization                       │
├─────────────────────────────────────────────┤
│     Persistence (RocksDB + WAL)             │
└─────────────────────────────────────────────┘
```

**Pattern:** Layered architecture with clear separation of concerns, combined with modular workspace architecture for independent compilation and testing.

---

## 🔑 Core Abstractions

| Abstraction | Purpose | Location |
|-------------|---------|----------|
| **Point** | Individual vector + payload + ID | `lib/segment/src/types.rs` |
| **Segment** | Immutable collection of vectors with indexes | `lib/segment` |
| **Collection** | Logical grouping of points (sharded) | `lib/collection` |
| **Shard** | Distributed partition of a collection | `lib/shard` |
| **Index** | Search acceleration structure (HNSW, field indexes) | `lib/segment/src/index` |

---

## 🛠️ Technology Stack

### Core Technologies
- **Web Framework:** Actix-web 4.11.0 (REST API)
- **RPC Framework:** Tonic 0.11.0 (gRPC)
- **Async Runtime:** Tokio 1.47.1
- **Database:** RocksDB 0.23.0 (key-value store)
- **Consensus:** Raft 0.7.0 (distributed coordination)
- **Memory Allocator:** Jemalloc 0.6 (performance-optimized)

### Key Libraries
- **Serialization:** serde, serde_json, serde_cbor, bincode
- **Parallelism:** rayon 1.11.0, tokio
- **Observability:** prometheus 0.14.0, tracing 0.1
- **Security:** jsonwebtoken 10.0, rustls 0.23.31
- **Math/Geo:** geo 0.31.0, geohash 0.13.1, half 2.7.0

---

## 🎯 Key Features

### 1. **Vector Search**
- **HNSW** (Hierarchical Navigable Small World) for approximate nearest neighbor search
- **Multiple distance metrics**: Cosine, Euclidean, Dot Product, Manhattan
- **Exact search fallback** for small result sets

### 2. **Hybrid Search**
- **Dense vectors** (traditional embeddings)
- **Sparse vectors** (keyword-based, BM25-like)
- **Combined search** for best of both worlds

### 3. **Advanced Filtering**
- **Payload indexing** on any JSON field
- **Full-text search** capabilities
- **Geo-spatial queries** with radius/polygon filters
- **Date/time range filtering**

### 4. **Optimization Features**
- **Vector Quantization**: Scalar, Product, Binary (up to 97% RAM reduction)
- **On-disk storage** for vectors and payloads
- **SIMD acceleration** (x86-64, ARM NEON)
- **Async I/O** with io_uring on Linux

### 5. **Distributed Deployment**
- **Sharding** for horizontal scaling
- **Replication** for high availability
- **Raft consensus** for cluster coordination
- **Zero-downtime** rolling updates

### 6. **Reliability**
- **Write-Ahead Log (WAL)** for durability
- **Snapshots** for backup/restore
- **Health monitoring** (Green/Yellow/Red status)
- **Graceful degradation**

---

## 🧪 Testing Strategy

### Test Coverage
- **Unit tests**: In each library crate (`lib/*/tests/`)
- **Integration tests**: End-to-end workflows (`tests/`)
- **Consensus tests**: Distributed system testing
- **Benchmarks**: Performance micro-benchmarks (`lib/*/benches/`)

### Test Types
1. **Basic API tests** (REST/gRPC functionality)
2. **Consensus tests** (Raft cluster coordination)
3. **Segment tests** (Vector storage and indexing)
4. **Collection tests** (Shard management)
5. **Quantization tests** (Compression accuracy)

---

## 📈 Performance Characteristics

### Optimizations Employed
1. **Memory-mapped files** for large datasets
2. **Jemalloc** custom allocator with tuned page sizes
3. **Rayon** for data parallelism
4. **io_uring** for Linux async I/O
5. **SIMD** hardware acceleration
6. **Bitpacking** for compression
7. **Query planning** to optimize execution

### Scalability Approach
- **Vertical**: Quantization, on-disk storage, SIMD
- **Horizontal**: Sharding, replication, consensus

---

## 🔐 Security Features

- **JWT Authentication** (jsonwebtoken 10.0)
- **TLS/SSL Support** (rustls 0.23.31)
- **RBAC Framework** (Role-Based Access Control)
- **API Key validation**
- **Read-only API keys** for restricted access
- **Payload validation** and sanitization

---

## 🚀 Deployment

### Build Profiles
- **release**: Production (full LTO, optimized)
- **dev**: Development (fast compilation)
- **ci**: CI/CD (release + debug assertions)
- **bench**: Benchmarking
- **perf**: Performance testing

### Docker Support
- **Multi-stage builds** for optimization
- **GPU support**: NVIDIA (CUDA), AMD (ROCm)
- **Multi-architecture**: x86_64, aarch64

### Ports
- **6333**: HTTP REST API
- **6334**: gRPC API
- **6335**: P2P cluster communication

---

## 📚 Documentation Roadmap

### Initial Analysis
1. ✅ **00-quick-start.md** (this file)
2. **repository-structure.md** - Detailed directory tree
3. **dependency-graph.md** - Visual dependency mapping
4. **metrics-summary.md** - Quantitative code metrics
5. **terminology-glossary.md** - Project-specific terms

### Blog Series (5-7 posts)
1. Architecture and core concepts
2. Deep dive: HNSW index implementation
3. Patterns and practices
4. Extending and integrating
5. Performance analysis
6. Distributed systems deep-dive
7. Vector quantization strategies

### RFCs (5-10 proposals)
Focus areas: Performance, API design, developer experience, testing, architecture

---

## 🎓 Key Learnings

### Strengths
1. **Excellent modularity**: 19 workspace crates with clear boundaries
2. **Production-ready**: WAL, snapshots, consensus, monitoring
3. **Performance-focused**: Multiple optimization strategies
4. **Well-tested**: Comprehensive unit, integration, and e2e tests
5. **Modern Rust**: Edition 2024, leverages latest features

### Trade-offs
1. **Complexity vs Flexibility**: Layered architecture adds indirection but enables modularity
2. **Memory vs Speed**: Quantization trades accuracy for RAM savings
3. **Approximate vs Exact**: HNSW trades perfect recall for speed
4. **Distributed Complexity**: Raft consensus adds operational overhead

### Design Philosophy
- **Safety First**: Rust's memory safety guarantees
- **Zero-Cost Abstractions**: Performance without runtime overhead
- **Explicit over Implicit**: Clear error handling, no magic
- **Composability**: Small, focused modules that work together

---

## 📖 Next Steps

1. **For Understanding Architecture**: Read `repository-structure.md` and `dependency-graph.md`
2. **For Code Metrics**: Check `metrics-summary.md`
3. **For Learning Concepts**: Start with Blog Series #1
4. **For Contributing**: Review RFCs for improvement opportunities
5. **For Deep Dives**: Explore individual module documentation in `lib/*/README.md`

---

## 🔗 Quick Links

- **Repository**: https://github.com/qdrant/qdrant
- **Documentation**: https://qdrant.tech/documentation/
- **API Docs**: https://api.qdrant.tech/
- **Benchmarks**: https://qdrant.tech/benchmarks/
- **Cloud**: https://cloud.qdrant.io/

---

## Analysis Baseline

All analysis based on:
- **Commit**: [`adcda004057df08389106da56f440db185f0c382`](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)
- **Branch**: `claude/codebase-analysis-blog-rfcs-013Xgm985AdvzYpEk1JAEgCW`
- **Date**: 2025-11-16

---

**Note**: This is a comprehensive analysis of an open-source project. All observations are based on code inspection and public documentation.
