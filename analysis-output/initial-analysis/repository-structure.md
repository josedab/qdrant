# Qdrant Repository Structure

**Analysis Baseline:** Commit `adcda004057df08389106da56f440db185f0c382`

---

## Directory Tree with Descriptions

```
/home/user/qdrant/
│
├── 📁 src/                                    # Main application binary source
│   ├── main.rs                                # Entry point (CLI parsing, server startup)
│   ├── actix/                                 # REST API implementation
│   │   ├── api/                               # REST endpoint handlers
│   │   │   ├── collections_api.rs             # Collection CRUD operations
│   │   │   ├── points_api.rs                  # Point upsert/delete/retrieve
│   │   │   ├── search_api.rs                  # Vector search endpoints
│   │   │   ├── recommend_api.rs               # Recommendation engine
│   │   │   ├── query_api.rs                   # Advanced query API
│   │   │   ├── cluster_api.rs                 # Cluster management
│   │   │   ├── snapshot_api.rs                # Backup/restore
│   │   │   ├── service_api.rs                 # Health, metrics
│   │   │   └── ...                            # Other endpoints
│   │   ├── auth.rs                            # JWT authentication middleware
│   │   ├── mod.rs                             # Actix-web server setup
│   │   └── web_ui.rs                          # Embedded web UI hosting
│   │
│   ├── tonic/                                 # gRPC API implementation
│   │   ├── api/                               # gRPC service handlers
│   │   │   ├── collections_api.rs             # Collections gRPC service
│   │   │   ├── points_api.rs                  # Points gRPC service
│   │   │   ├── snapshots_api.rs               # Snapshots gRPC service
│   │   │   ├── raft_api.rs                    # Raft consensus service
│   │   │   └── ...
│   │   ├── auth.rs                            # gRPC auth interceptors
│   │   ├── logging.rs                         # gRPC logging middleware
│   │   └── mod.rs                             # Tonic server setup
│   │
│   ├── common/                                # Shared application utilities
│   │   ├── inference/                         # ML model inference service
│   │   │   ├── service.rs                     # Inference server
│   │   │   └── ...
│   │   ├── auth/                              # Authentication helpers
│   │   ├── helpers.rs                         # Runtime creation, TLS config
│   │   ├── metrics.rs                         # Prometheus metrics
│   │   ├── health.rs                          # Health check logic
│   │   └── telemetry.rs                       # Usage telemetry
│   │
│   ├── consensus.rs                           # Raft consensus implementation
│   ├── settings.rs                            # Configuration file parsing
│   ├── snapshots.rs                           # Snapshot recovery logic
│   ├── startup.rs                             # Startup procedures
│   ├── migrations.rs                          # Migration helpers
│   ├── greeting.rs                            # Welcome banner
│   ├── tracing.rs                             # Distributed tracing setup
│   ├── issues_setup.rs                        # Issue tracking initialization
│   │
│   ├── schema_generator.rs                    # OpenAPI schema generator
│   ├── wal_inspector.rs                       # WAL debugging tool
│   ├── wal_pop.rs                             # WAL manipulation tool
│   └── segment_inspector.rs                   # Segment debugging tool
│
├── 📁 lib/                                    # Modular library crates (workspace)
│   │
│   ├── 📁 api/                                # API definitions & type conversions
│   │   └── src/
│   │       ├── rest/                          # REST API schemas
│   │       │   ├── schema.rs                  # Main REST types
│   │       │   ├── models.rs                  # Request/response models
│   │       │   └── ...
│   │       ├── grpc/                          # gRPC protocol definitions
│   │       │   ├── conversions.rs             # Proto ↔ Rust conversions
│   │       │   ├── proto/                     # Protocol Buffer definitions
│   │       │   │   ├── qdrant.proto           # Main service definition
│   │       │   │   ├── collections.proto      # Collection types
│   │       │   │   ├── points_service.proto   # Points CRUD
│   │       │   │   ├── snapshots_service.proto # Snapshots
│   │       │   │   └── ...
│   │       │   └── qdrant.rs                  # Generated proto code
│   │       └── lib.rs
│   │
│   ├── 📁 collection/                         # Collection management layer
│   │   └── src/
│   │       ├── collection/                    # Collection abstraction
│   │       │   ├── mod.rs                     # Main collection logic
│   │       │   ├── search.rs                  # Search operations
│   │       │   ├── update.rs                  # Update operations
│   │       │   └── ...
│   │       ├── shards/                        # Shard management
│   │       │   ├── mod.rs                     # Shard coordinator
│   │       │   ├── local_shard.rs             # Local shard operations
│   │       │   ├── remote_shard.rs            # Remote shard proxy
│   │       │   ├── replica_set.rs             # Replica management
│   │       │   ├── transfer/                  # Shard transfer protocols
│   │       │   └── ...
│   │       ├── operations/                    # Operation types
│   │       │   ├── types.rs                   # Upsert, Delete, etc.
│   │       │   ├── payload_ops.rs             # Payload operations
│   │       │   └── vector_ops.rs              # Vector operations
│   │       ├── config.rs                      # Collection configuration
│   │       ├── wal_delta.rs                   # Write-Ahead Log delta
│   │       ├── hash_ring.rs                   # Consistent hashing
│   │       └── ...
│   │
│   ├── 📁 segment/                            # Core vector segment (largest module)
│   │   └── src/
│   │       ├── segment/                       # Segment implementation
│   │       │   ├── mod.rs                     # Main segment logic
│   │       │   ├── search.rs                  # Search operations
│   │       │   ├── update.rs                  # Update operations
│   │       │   └── ...
│   │       │
│   │       ├── index/                         # Indexing structures
│   │       │   ├── hnsw_index/                # HNSW graph index
│   │       │   │   ├── hnsw.rs                # Core HNSW algorithm
│   │       │   │   ├── graph_layers.rs        # Layer management
│   │       │   │   ├── graph_links.rs         # Link storage
│   │       │   │   ├── entry_points.rs        # Multi-entry optimization
│   │       │   │   ├── point_scorer.rs        # Scoring interface
│   │       │   │   ├── gpu/                   # GPU acceleration
│   │       │   │   └── tests/
│   │       │   │
│   │       │   ├── field_index/               # Payload field indexing
│   │       │   │   ├── mod.rs                 # Field index coordinator
│   │       │   │   ├── numeric_index.rs       # Integer/float indexes
│   │       │   │   ├── keyword_index.rs       # Keyword/UUID indexes
│   │       │   │   ├── full_text_index/       # Full-text search
│   │       │   │   ├── geo_index/             # Geospatial indexing
│   │       │   │   ├── map_index.rs           # Nested object indexing
│   │       │   │   └── ...
│   │       │   │
│   │       │   ├── sparse_index/              # Sparse vector indexing
│   │       │   │   ├── sparse_index_config.rs
│   │       │   │   └── ...
│   │       │   │
│   │       │   └── query_optimization/        # Query planner
│   │       │       ├── optimizer.rs           # Cost-based optimizer
│   │       │       ├── condition_converter.rs
│   │       │       └── ...
│   │       │
│   │       ├── vector_storage/                # Vector storage implementations
│   │       │   ├── dense/                     # Dense vector storage
│   │       │   │   ├── simple_dense_vector_storage.rs  # In-memory
│   │       │   │   ├── mmap_dense_vector_storage.rs    # Memory-mapped
│   │       │   │   ├── appendable_mmap_dense_vector_storage.rs
│   │       │   │   └── ...
│   │       │   │
│   │       │   ├── sparse/                    # Sparse vector storage
│   │       │   │   ├── simple_sparse_vector_storage.rs
│   │       │   │   ├── mmap_sparse_vector_storage.rs
│   │       │   │   └── ...
│   │       │   │
│   │       │   ├── quantized/                 # Quantized vector storage
│   │       │   │   ├── quantized_vectors.rs   # Core quantization
│   │       │   │   ├── quantized_mmap_storage.rs
│   │       │   │   ├── quantized_query_scorer.rs
│   │       │   │   └── ...
│   │       │   │
│   │       │   ├── async_raw_scorer.rs        # io_uring async scoring
│   │       │   ├── query_scorer/              # Scoring implementations
│   │       │   └── vector_storage_base.rs     # Storage trait
│   │       │
│   │       ├── payload_storage/               # Payload (metadata) storage
│   │       │   ├── mod.rs                     # Payload coordinator
│   │       │   ├── on_disk_payload_storage.rs # RocksDB-backed
│   │       │   ├── in_memory_payload_storage.rs
│   │       │   └── ...
│   │       │
│   │       ├── id_tracker/                    # Point ID tracking
│   │       │   ├── mod.rs                     # ID manager
│   │       │   ├── in_memory_id_tracker.rs
│   │       │   └── ...
│   │       │
│   │       ├── quantization/                  # Vector quantization
│   │       │   └── ...                        # (Moved to lib/quantization)
│   │       │
│   │       ├── spaces/                        # Vector space implementations
│   │       │   ├── metric.rs                  # Distance metric trait
│   │       │   ├── simple.rs                  # Cosine, Dot, Euclidean
│   │       │   ├── custom_distance.rs         # User-defined metrics
│   │       │   └── ...
│   │       │
│   │       ├── types.rs                       # Core data types
│   │       ├── entry/                         # Segment entry points
│   │       ├── fixtures/                      # Test fixtures
│   │       └── tests/                         # Unit tests
│   │
│   ├── 📁 shard/                              # Shard operations & management
│   │   └── src/
│   │       ├── search.rs                      # Search on shard
│   │       ├── update.rs                      # Update operations
│   │       ├── wal.rs                         # WAL integration
│   │       └── ...
│   │
│   ├── 📁 storage/                            # Storage layer & persistence
│   │   └── src/
│   │       ├── content_manager/               # Content/collection management
│   │       │   ├── toc/                       # Table of Contents (registry)
│   │       │   │   ├── mod.rs                 # Collection registry
│   │       │   │   └── dispatcher.rs          # Request dispatcher
│   │       │   ├── consensus/                 # Consensus operations
│   │       │   │   ├── operation_sender.rs    # Raft operation sender
│   │       │   │   ├── persistent.rs          # Persistent state
│   │       │   │   └── ...
│   │       │   ├── snapshots/                 # Snapshot management
│   │       │   │   ├── download.rs            # Snapshot download
│   │       │   │   ├── upload.rs              # Snapshot upload
│   │       │   │   └── ...
│   │       │   └── ...
│   │       ├── rbac/                          # Role-Based Access Control
│   │       │   ├── mod.rs                     # RBAC framework
│   │       │   └── ...
│   │       ├── dispatcher.rs                  # Operation dispatcher
│   │       └── types.rs                       # Storage types
│   │
│   ├── 📁 quantization/                       # Vector quantization library
│   │   ├── src/
│   │   │   ├── quantization.rs                # Core quantization logic
│   │   │   ├── product/                       # Product Quantization (PQ)
│   │   │   ├── scalar/                        # Scalar Quantization
│   │   │   ├── binary/                        # Binary Quantization
│   │   │   └── ...
│   │   ├── cpp/                               # C++ optimizations
│   │   └── benches/                           # Performance benchmarks
│   │
│   ├── 📁 sparse/                             # Sparse vector support
│   │   └── src/
│   │       ├── index/                         # Sparse vector indexing
│   │       └── ...
│   │
│   ├── 📁 gridstore/                          # Gridstore (specialized storage)
│   │   └── src/
│   │
│   ├── 📁 posting_list/                       # Posting list data structure
│   │   └── src/                               # Inverted index support
│   │
│   ├── 📁 gpu/                                # GPU acceleration support
│   │   └── src/
│   │       ├── mod.rs                         # GPU interface
│   │       ├── nvidia.rs                      # NVIDIA CUDA support
│   │       └── amd.rs                         # AMD ROCm support
│   │
│   ├── 📁 edge/                               # Edge library (client SDK)
│   │   ├── src/                               # Rust client
│   │   ├── python/                            # Python bindings
│   │   └── examples/
│   │
│   ├── 📁 macros/                             # Procedural macros
│   │   └── src/
│   │
│   └── 📁 common/                             # Shared utilities (multiple crates)
│       ├── common/                            # Common utilities
│       │   ├── bitpacking/                    # Bit-level operations
│       │   ├── mmap_hashmap.rs                # Memory-mapped hash maps
│       │   ├── flags.rs                       # Feature flags
│       │   ├── cpu.rs                         # CPU detection
│       │   ├── budget.rs                      # Resource budgeting
│       │   └── ...
│       ├── cancel/                            # Cancellation tokens
│       ├── memory/                            # Memory management
│       ├── io/                                # I/O utilities
│       ├── issues/                            # Issue tracking
│       └── dataset/                           # Test datasets
│
├── 📁 config/                                 # Configuration templates
│   ├── config.yaml                            # Default configuration
│   └── deb.yaml                               # Debian package config
│
├── 📁 docs/                                   # Documentation
│   ├── QUICK_START.md                         # Getting started guide
│   ├── DEVELOPMENT.md                         # Developer guide
│   ├── CONTRIBUTING.md                        # Contribution guidelines
│   └── redoc/                                 # OpenAPI documentation
│       ├── master/
│       └── ...
│
├── 📁 tests/                                  # Integration & E2E tests
│   ├── basic_api_test.sh                      # REST API integration tests
│   ├── basic_grpc_test.sh                     # gRPC integration tests
│   ├── basic_sparse_test.sh                   # Sparse vector tests
│   ├── basic_multivector_grpc_test.sh         # Multi-vector tests
│   ├── consensus_tests/                       # Consensus/cluster tests
│   │   ├── test_consensus_restart.sh
│   │   └── ...
│   ├── e2e_tests/                             # End-to-end tests
│   └── storage/                               # Storage layer tests
│
├── 📁 tools/                                  # Development tools
│   ├── schema2openapi/                        # OpenAPI schema generator
│   ├── compose/                               # Docker Compose configurations
│   │   ├── docker-compose.yml                 # Basic setup
│   │   ├── docker-compose-cluster.yml         # Cluster setup
│   │   └── ...
│   └── nix/                                   # Nix environment setup
│
├── 📁 openapi/                                # OpenAPI specifications
│   └── openapi-merged.json                    # Merged OpenAPI spec
│
├── 📁 pkg/                                    # Package configurations
│   └── ...                                    # RPM, DEB configs
│
├── 📁 static/                                 # Web UI static assets
│   ├── index.html                             # Dashboard UI
│   └── assets/
│
├── 📄 Cargo.toml                              # Workspace manifest
├── 📄 Cargo.lock                              # Dependency lock file
├── 📄 Dockerfile                              # Multi-stage Docker build
├── 📄 .dockerignore                           # Docker ignore patterns
├── 📄 rustfmt.toml                            # Code formatting config
├── 📄 clippy.toml                             # Linter configuration
├── 📄 README.md                               # Project README
├── 📄 LICENSE                                 # Apache 2.0 License
├── 📄 .gitignore                              # Git ignore patterns
└── 📄 .github/                                # GitHub Actions CI/CD
    └── workflows/
        ├── rust.yml                           # Main CI workflow
        └── ...

```

---

## Workspace Members

The project uses Cargo workspace with 19 independent crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **qdrant** (binary) | `/` | Main application entry point |
| **api** | `lib/api` | API definitions and conversions |
| **collection** | `lib/collection` | Collection and shard management |
| **segment** | `lib/segment` | Core vector segment (largest module) |
| **shard** | `lib/shard` | Shard-level operations |
| **storage** | `lib/storage` | Persistence and content management |
| **quantization** | `lib/quantization` | Vector quantization algorithms |
| **sparse** | `lib/sparse` | Sparse vector support |
| **gridstore** | `lib/gridstore` | Specialized storage layer |
| **posting_list** | `lib/posting_list` | Inverted index data structure |
| **gpu** | `lib/gpu` | GPU acceleration |
| **edge** | `lib/edge` | Client SDK |
| **edge/python** | `lib/edge/python` | Python bindings |
| **macros** | `lib/macros` | Procedural macros |
| **common** | `lib/common/common` | Common utilities |
| **cancel** | `lib/common/cancel` | Cancellation tokens |
| **memory** | `lib/common/memory` | Memory utilities |
| **io** | `lib/common/io` | I/O utilities |
| **issues** | `lib/common/issues` | Issue tracking |
| **dataset** | `lib/common/dataset` | Test datasets |

---

## Key Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace manifest, dependencies, features, profiles |
| `config/config.yaml` | Default server configuration |
| `Dockerfile` | Multi-stage Docker build with GPU support |
| `rustfmt.toml` | Code formatting rules (edition 2024) |
| `clippy.toml` | Linter configuration |
| `.github/workflows/rust.yml` | CI/CD pipeline |

---

## Build Artifacts (Excluded from Analysis)

```
target/                    # Cargo build output
  ├── debug/               # Development builds
  ├── release/             # Production builds
  └── ...
```

---

## Navigation Guide

### To Understand API Surface
- **REST API**: `src/actix/api/` and `lib/api/src/rest/`
- **gRPC API**: `src/tonic/api/` and `lib/api/src/grpc/proto/`

### To Understand Core Logic
- **Search**: `lib/segment/src/index/hnsw_index/`
- **Filtering**: `lib/segment/src/index/field_index/`
- **Vector Storage**: `lib/segment/src/vector_storage/`
- **Quantization**: `lib/quantization/`

### To Understand Distributed Systems
- **Consensus**: `src/consensus.rs` and `lib/storage/src/content_manager/consensus/`
- **Sharding**: `lib/collection/src/shards/`
- **Replication**: `lib/collection/src/shards/replica_set.rs`

### To Understand Persistence
- **WAL**: `lib/collection/src/wal_delta.rs`
- **Storage**: `lib/storage/src/content_manager/`
- **Snapshots**: `lib/storage/src/content_manager/snapshots/`

---

## Entry Points

| Entry Point | File | Purpose |
|-------------|------|---------|
| **Main Application** | `src/main.rs` | Server startup |
| **REST Server** | `src/actix/mod.rs` | Actix-web setup |
| **gRPC Server** | `src/tonic/mod.rs` | Tonic setup |
| **Collection Management** | `lib/storage/src/content_manager/toc/mod.rs` | Table of contents |
| **Segment Operations** | `lib/segment/src/segment/mod.rs` | Segment abstraction |
| **HNSW Index** | `lib/segment/src/index/hnsw_index/hnsw.rs` | Graph search |

---

## Notes

- **Modularity**: Each `lib/` crate can be compiled and tested independently
- **Feature Flags**: Optional features (GPU, tracing, rocksdb) are feature-gated
- **Platform Support**: Cross-platform (Linux, macOS, Windows), multi-arch (x86_64, aarch64)
- **Test Organization**: Unit tests in `src/`, integration tests in `tests/`

---

**Analysis Baseline:** Commit [`adcda004057df08389106da56f440db185f0c382`](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)
