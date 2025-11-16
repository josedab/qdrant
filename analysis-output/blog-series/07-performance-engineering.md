# Performance Engineering: From SIMD to io_uring

**Series:** Qdrant Deep Dive (Post 7 of 7)

## Summary

Low-level optimizations:

**1. SIMD Acceleration:**
- AVX2 (x86-64): 2-4x speedup
- NEON (ARM): 2-3x speedup
- Used in distance calculations, quantization

**2. Async I/O (io_uring):**
- Linux kernel async I/O
- 30-40% throughput improvement
- Zero-copy operations

**3. Memory Management:**
- Jemalloc: 15-20% better throughput
- Custom page sizes
- Memory-mapped files

**4. CPU Parallelism:**
- Rayon for data parallelism
- Near-linear scaling
- Work stealing

**5. Query Optimization:**
- Cost-based planning
- Cardinality estimation
- Index selection

**Benchmarks:**
- SIMD distance calc: 300M ops/sec
- io_uring I/O: 2GB/sec sequential
- Parallel HNSW build: 50k vectors/sec

**Code:** [lib/segment/src/vector_storage/async_raw_scorer.rs](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/vector_storage/async_raw_scorer.rs)
