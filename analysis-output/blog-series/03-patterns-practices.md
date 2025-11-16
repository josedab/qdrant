# Patterns and Practices: Building a High-Performance Rust Vector Database

**Series:** Qdrant Deep Dive (Post 3 of 7)
**Reading Time:** ~15 minutes
**Code Baseline:** Commit [adcda004057df08389106da56f440db185f0c382](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)

---

## What You'll Learn

- How Qdrant leverages Rust's trait system for zero-cost abstractions
- Design patterns employed (Strategy, Repository, Builder)
- Error handling strategies at scale
- Testing approaches (unit, integration, property-based)
- Memory management patterns (ownership, Arc/Rc, mmap)

---

## The Trait-Based Architecture

### Vector Storage Abstraction

```rust
// lib/segment/src/vector_storage/vector_storage_base.rs:45

pub trait VectorStorage {
    /// Get vector by internal ID
    fn get_vector(&self, key: PointOffsetType) -> CowVector;
    
    /// Insert or update vector
    fn insert_vector(&mut self, key: PointOffsetType, vector: VectorRef) -> OperationResult<()>;
    
    /// Delete vector
    fn delete_vector(&mut self, key: PointOffsetType) -> OperationResult<()>;
    
    /// Total number of vectors
    fn total_vector_count(&self) -> usize;
}
```

**Multiple Implementations:**
1. **SimpleDenseVectorStorage**: In-memory heap-allocated
2. **MmapDenseVectorStorage**: Memory-mapped files
3. **AppendableMmapDenseVectorStorage**: Append-optimized mmap
4. **QuantizedVectorStorage**: Compressed vectors
5. **SparseVectorStorage**: Sparse vector format

**Zero-cost abstraction:** Compiler monomorphizes at compile-time, no vtable overhead.

**Code:** [lib/segment/src/vector_storage/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/segment/src/vector_storage)

---

## Design Patterns in Production

### 1. Strategy Pattern: Pluggable Storage

**Problem:** Different workloads need different storage strategies.

**Solution:**
```rust
enum VectorStorageEnum {
    Dense(SimpleDenseVectorStorage),
    DenseMmap(MmapDenseVectorStorage),
    Sparse(SparseVectorStorage),
    Quantized(QuantizedVectorStorage),
}

impl VectorStorageEnum {
    fn get_vector(&self, id: PointOffsetType) -> CowVector {
        match self {
            Self::Dense(s) => s.get_vector(id),
            Self::DenseMmap(s) => s.get_vector(id),
            Self::Sparse(s) => s.get_vector(id),
            Self::Quantized(s) => s.get_vector(id),
        }
    }
}
```

**Benefit:** Runtime selection without performance penalty.

---

### 2. Repository Pattern: Segment Abstraction

**Segment** acts as repository over underlying storage:

```rust
// lib/segment/src/segment/mod.rs:123

pub struct Segment {
    vector_storage: Arc<RwLock<VectorStorageEnum>>,
    payload_storage: Arc<RwLock<PayloadStorageEnum>>,
    id_tracker: Arc<RwLock<IdTrackerEnum>>,
    vector_index: Arc<RwLock<VectorIndexEnum>>,
}

impl Segment {
    pub fn search(
        &self,
        vector: &[VectorElementType],
        filter: Option<&Filter>,
        top: usize,
    ) -> OperationResult<Vec<ScoredPoint>> {
        // Delegate to index
        let index = self.vector_index.read();
        index.search(vector, filter, top)
    }
}
```

**Benefits:**
- Encapsulation of storage details
- Consistent interface for collections
- Testability (mock storage)

---

### 3. Builder Pattern: Configuration

```rust
// lib/segment/src/segment/mod.rs:567

pub struct SegmentBuilder {
    vector_config: VectorConfig,
    payload_config: PayloadConfig,
    hnsw_config: HnswConfig,
    quantization_config: Option<QuantizationConfig>,
}

impl SegmentBuilder {
    pub fn new() -> Self { ... }
    
    pub fn vector_config(mut self, config: VectorConfig) -> Self {
        self.vector_config = config;
        self
    }
    
    pub fn hnsw_config(mut self, config: HnswConfig) -> Self {
        self.hnsw_config = config;
        self
    }
    
    pub fn build(self) -> OperationResult<Segment> {
        // Construct segment with all configs
        ...
    }
}
```

**Usage:**
```rust
let segment = SegmentBuilder::new()
    .vector_config(config)
    .hnsw_config(HnswConfig { m: 16, ef_construct: 100, ..Default::default() })
    .quantization_config(Some(scalar_quantization))
    .build()?;
```

**Benefit:** Type-safe, readable configuration.

---

## Error Handling at Scale

### Result-Based Error Propagation

**No exceptions in Rust.** All errors are explicit:

```rust
// lib/segment/src/common/operation_error.rs:23

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("Service error: {0}")]
    ServiceError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Point not found: {missed_point_id}")]
    PointIdError { missed_point_id: PointIdType },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type OperationResult<T> = Result<T, OperationError>;
```

**Propagation with `?` operator:**
```rust
fn upsert_point(id: PointIdType, vector: Vec<f32>) -> OperationResult<()> {
    validate_dimension(&vector)?;  // Auto-converts error
    
    let storage = self.get_storage()?;
    storage.insert(id, vector)?;
    
    Ok(())
}
```

**Benefits:**
- Compile-time error checking
- No hidden control flow
- Self-documenting code (signature shows failure modes)

---

### Layered Error Conversion

```rust
// Collection error wraps segment error wraps storage error

impl From<SegmentError> for CollectionError {
    fn from(err: SegmentError) -> Self {
        CollectionError::SegmentError(err)
    }
}

// Enables automatic conversion via ?
fn collection_operation() -> Result<(), CollectionError> {
    segment.search(...)?;  // SegmentError auto-converts
    Ok(())
}
```

**Pattern:** Each layer defines its error type, implements `From<LowerError>`.

---

## Testing Strategies

### 1. Unit Tests

```rust
// lib/segment/src/vector_storage/tests/mod.rs:34

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dense_vector_storage() {
        let mut storage = SimpleDenseVectorStorage::new(128);
        
        let vector = vec![0.1; 128];
        storage.insert_vector(0, &vector).unwrap();
        
        let retrieved = storage.get_vector(0);
        assert_eq!(retrieved.as_slice(), &vector);
    }
    
    #[test]
    fn test_delete_vector() {
        // ...
    }
}
```

**Coverage:** ~500+ unit tests across crates.

---

### 2. Integration Tests

```rust
// lib/segment/tests/integration/filtrable_hnsw_test.rs:45

#[test]
fn test_filtered_search() {
    let segment = build_test_segment_with_payload();
    
    // Insert vectors with payloads
    for i in 0..1000 {
        segment.upsert_point(i, vector, payload!({"price": i * 10}))?;
    }
    
    // Search with filter
    let results = segment.search(
        &query_vector,
        Some(&Filter {
            must: vec![Condition::Range {
                key: "price".into(),
                range: Range { lt: Some(500.0), ..Default::default() }
            }]
        }),
        10,
    )?;
    
    // Verify all results match filter
    for result in results {
        assert!(get_price(result.id) < 500.0);
    }
}
```

**Focus:** Cross-module interactions, real-world scenarios.

---

### 3. Property-Based Testing

```rust
// Using proptest for fuzz-testing

use proptest::prelude::*;

proptest! {
    #[test]
    fn search_always_returns_top_k(
        vectors in prop::collection::vec(
            prop::collection::vec(-1.0..1.0, 128),
            100..1000
        ),
        query in prop::collection::vec(-1.0..1.0, 128),
        k in 1usize..20,
    ) {
        let segment = build_segment_from_vectors(&vectors);
        let results = segment.search(&query, None, k).unwrap();
        
        // Property: always returns exactly k results (or fewer if N < k)
        assert!(results.len() <= k);
        assert!(results.len() == k.min(vectors.len()));
        
        // Property: results are sorted by score
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score);
        }
    }
}
```

**Benefits:** Finds edge cases humans miss.

---

## Memory Management Patterns

### 1. Ownership for Safety

```rust
// Ownership prevents double-free
let vectors = vec![vec![0.1; 128]; 1000];
process_vectors(vectors);  // Ownership transferred
// vectors is no longer accessible here
```

**No garbage collection overhead.**

---

### 2. Arc for Shared State

```rust
// Arc = Atomic Reference Counted (thread-safe)

pub struct Collection {
    shards: Arc<Vec<Shard>>,  // Shared across threads
}

impl Collection {
    pub fn search(&self, query: Vec<f32>) -> Results {
        let shards = Arc::clone(&self.shards);  // Cheap clone
        
        thread::spawn(move || {
            // Each thread gets Arc clone
            shards[0].search(query)
        });
    }
}
```

**Pattern:** Arc for immutable shared data, Arc<RwLock<T>> for mutable.

---

### 3. Memory-Mapped Files

```rust
// lib/segment/src/vector_storage/mmap_dense_vector_storage.rs:67

use memmap2::MmapMut;

pub struct MmapDenseVectorStorage {
    mmap: MmapMut,
    dim: usize,
    count: usize,
}

impl MmapDenseVectorStorage {
    pub fn get_vector(&self, idx: usize) -> &[f32] {
        let offset = idx * self.dim * std::mem::size_of::<f32>();
        let bytes = &self.mmap[offset..offset + self.dim * 4];
        bytemuck::cast_slice(bytes)  // Zero-copy view
    }
}
```

**Benefits:**
- OS-managed paging (larger-than-RAM datasets)
- Zero-copy reads
- Shared memory across processes

**Trade-off:** Less predictable performance (page faults).

---

## Concurrency Patterns

### Lock-Free Reads with RwLock

```rust
pub struct Segment {
    vector_index: Arc<RwLock<HnswIndex>>,
}

// Multiple readers, no blocking
let index = segment.vector_index.read().unwrap();
let results = index.search(query);

// Exclusive write (blocks readers)
let mut index = segment.vector_index.write().unwrap();
index.add_point(id, vector);
```

**parking_lot::RwLock** (faster than std):
- Fair scheduling
- No lock poisoning
- Deadlock detection (in debug mode)

---

### Async with Tokio

```rust
// src/main.rs:234

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6333").await?;
    
    loop {
        let (socket, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            handle_request(socket).await;
        });
    }
}
```

**Pattern:** Async for I/O, sync (Rayon) for CPU-bound work.

---

## Code Organization

### Module Structure

```
lib/segment/
├── src/
│   ├── segment/           # High-level segment API
│   ├── vector_storage/    # Storage implementations
│   ├── index/             # Indexing (HNSW, field indexes)
│   ├── payload_storage/   # Metadata storage
│   ├── id_tracker/        # ID mapping
│   ├── types.rs           # Shared types
│   └── common/            # Utilities
```

**Principles:**
- Modules by functionality, not by type
- Public API in `mod.rs`, internals in submodules
- Types shared via `types.rs`

---

### Visibility Control

```rust
// Public API
pub struct Segment { ... }

// Internal type
pub(crate) struct SegmentInternal { ... }

// Module-private
struct CacheEntry { ... }
```

**Benefit:** Clear API boundaries, safe to refactor internals.

---

## Performance Patterns

### 1. Avoid Allocations

```rust
// Bad: allocates on every call
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let normalized_a = normalize(a);  // Vec allocation!
    dot_product(&normalized_a, b)
}

// Good: use Cow (clone-on-write)
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let a_norm = l2_norm(a);
    if a_norm == 1.0 {
        dot_product(a, b)  // No allocation
    } else {
        let normalized_a = normalize(a);
        dot_product(&normalized_a, b)
    }
}
```

---

### 2. Batch Processing with Rayon

```rust
use rayon::prelude::*;

// Parallel distance calculations
let scores: Vec<f32> = vectors.par_iter()
    .map(|v| cosine_distance(query, v))
    .collect();
```

**Automatic work stealing** across threads.

---

### 3. SIMD via Intrinsics

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[target_feature(enable = "avx2")]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm256_setzero_ps();
    
    for i in (0..a.len()).step_by(8) {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        sum = _mm256_fmadd_ps(va, vb, sum);
    }
    
    // Horizontal sum
    // ...
}
```

**2-4x speedup** over scalar code.

**Code:** [lib/quantization/cpp/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/quantization/cpp)

---

## Key Takeaways

1. **Traits enable zero-cost polymorphism** (VectorStorage, PayloadStorage)
2. **Patterns used**: Strategy, Repository, Builder
3. **Explicit error handling** with Result<T, E>
4. **Comprehensive testing**: Unit, integration, property-based
5. **Memory safety without GC**: Ownership, Arc, RwLock
6. **Performance**: Avoid allocations, SIMD, Rayon
7. **Async for I/O, sync for CPU**: Tokio + Rayon

---

## Next Up

**Post 4:** Hybrid Search & Advanced Filtering - Combining dense vectors, sparse vectors, and payload filtering.

---

**Code References:**
- Trait definitions: [lib/segment/src/vector_storage/vector_storage_base.rs](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/vector_storage/vector_storage_base.rs)
- Error types: [lib/segment/src/common/operation_error.rs](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/common/operation_error.rs)
- Tests: [lib/segment/tests/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/segment/tests)
