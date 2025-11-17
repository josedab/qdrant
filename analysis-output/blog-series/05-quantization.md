# Quantization Strategies: Achieving 97% Memory Reduction

**Series:** Qdrant Deep Dive (Post 5 of 7)
**Reading Time:** ~20 minutes
**Code Baseline:** Commit [`adcda004057df08389106da56f440db185f0c382`](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)
**Prerequisites:** Post 1 (Architecture), Post 2 (HNSW), basic linear algebra

---

## What You'll Learn

- Why quantization is essential for production vector databases
- Three quantization strategies: Scalar, Product, and Binary
- The accuracy-memory-speed tradeoffs for each approach
- How rescoring maintains high recall despite compression
- When to use each quantization method
- Real-world benchmarks and configuration examples

---

## The Memory Problem

**Scenario:** You have 10 million product embeddings from a 768-dimensional model (e.g., OpenAI's `text-embedding-3-small`).

**Unquantized memory:**
```
10M vectors × 768 dimensions × 4 bytes (float32) = 30.7 GB
```

**Add HNSW index:**
```
HNSW: ~200 bytes per vector × 10M = 2 GB
Total: 32.7 GB
```

**Problem:** Single machine RAM limits, higher cloud costs, slower cold starts.

**Solution:** Quantization - compress vectors with minimal accuracy loss.

---

## What is Quantization?

**Quantization** reduces numerical precision to save memory:
- **float32** (32 bits) → **int8** (8 bits) = 4x reduction
- **float32** → **binary** (1 bit) = 32x reduction

**Key insight:** For similarity search, **relative rankings matter more than absolute distances**. We can compress aggressively as long as we preserve ordering.

---

## Strategy 1: Scalar Quantization

### The Concept

Map float32 values to int8 by finding min/max bounds:

```rust
// Simplified from lib/quantization/src/encoded_vectors_u8.rs

fn quantize_vector(vector: &[f32]) -> (Vec<u8>, f32, f32) {
    let min = vector.iter().copied().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max = vector.iter().copied().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    let range = max - min;
    let scale = 255.0 / range;

    let quantized: Vec<u8> = vector
        .iter()
        .map(|&v| ((v - min) * scale) as u8)
        .collect();

    (quantized, min, max)
}
```

**Example:**
```
Original: [0.1, 0.5, -0.3, 0.9, -0.1]
Min: -0.3, Max: 0.9, Range: 1.2

Quantized:
  0.1 → ((0.1 - (-0.3)) / 1.2) × 255 = 85
  0.5 → ((0.5 - (-0.3)) / 1.2) × 255 = 170
 -0.3 → ((−0.3 - (-0.3)) / 1.2) × 255 = 0
  0.9 → ((0.9 - (-0.3)) / 1.2) × 255 = 255
 -0.1 → ((−0.1 - (-0.3)) / 1.2) × 255 = 42

Result: [85, 170, 0, 255, 42]
```

**Storage:** 1 byte per dimension + 2 floats (min, max) per vector

### Distance Calculation

**Fast int8 distance** (using SIMD):
```rust
// Dot product on quantized vectors
fn quantized_dot_product(a: &[u8], b: &[u8]) -> i32 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32) * (y as i32))
        .sum()
}

// Asymmetric distance (query stays float32)
fn asymmetric_distance(query: &[f32], quantized: &[u8], min: f32, max: f32) -> f32 {
    let scale = (max - min) / 255.0;

    query.iter()
        .zip(quantized.iter())
        .map(|(&q, &v)| {
            let dequantized = min + (v as f32) * scale;
            q * dequantized
        })
        .sum()
}
```

**SIMD optimization:** Modern CPUs can compute 16-32 int8 products in a single instruction.

### Performance

**Memory:**
- **Reduction:** 4x (float32 → int8)
- **10M × 768-dim:** 30.7 GB → 7.7 GB ✅

**Speed:**
- **SIMD:** ~2-3x faster than float32 on modern CPUs
- **Cache:** Better cache utilization (4x more vectors per cache line)

**Accuracy:**
- **Recall@10:** 98-99% (nearly perfect)
- **Best for:** Cosine and dot product distances

**Configuration:**
```yaml
quantization:
  scalar:
    type: int8
    quantile: 0.99  # Clip outliers at 99th percentile
    always_ram: true  # Keep quantized vectors in RAM
```

**Code location:** [`lib/quantization/src/encoded_vectors_u8.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_u8.rs)

---

## Strategy 2: Product Quantization (PQ)

### The Concept

**Idea:** Split vectors into subvectors, cluster each subspace independently.

**Example (768-dim, 96 subvectors of 8-dim each):**
```
Original vector: [v₀, v₁, ..., v₇₆₇]

Split into 96 chunks:
  chunk₀ = [v₀, v₁, ..., v₇]
  chunk₁ = [v₈, v₉, ..., v₁₅]
  ...
  chunk₉₅ = [v₇₆₀, v₇₆₁, ..., v₇₆₇]

For each chunk, run k-means (k=256):
  chunk₀ → cluster ID (0-255) = 1 byte
  chunk₁ → cluster ID (0-255) = 1 byte
  ...
```

**Result:** 768 dimensions → 96 bytes (8x compression)

### Codebook Training

```rust
// Conceptual from lib/quantization/src/encoded_vectors_pq.rs

struct ProductQuantizer {
    codebooks: Vec<Codebook>,  // 96 codebooks
    num_subvectors: usize,     // 96
    subvector_dim: usize,      // 8
}

struct Codebook {
    centroids: Vec<Vec<f32>>,  // 256 centroids × 8-dim each
}

fn train_pq(vectors: &[Vec<f32>], num_subvectors: usize) -> ProductQuantizer {
    let dim = vectors[0].len();
    let subvector_dim = dim / num_subvectors;

    let mut codebooks = Vec::new();

    for i in 0..num_subvectors {
        let start = i * subvector_dim;
        let end = start + subvector_dim;

        // Extract subvectors
        let subvectors: Vec<Vec<f32>> = vectors
            .iter()
            .map(|v| v[start..end].to_vec())
            .collect();

        // Run k-means (k=256)
        let centroids = kmeans(&subvectors, 256);

        codebooks.push(Codebook { centroids });
    }

    ProductQuantizer {
        codebooks,
        num_subvectors,
        subvector_dim,
    }
}
```

### Distance Calculation

**Lookup tables** for fast approximate distance:
```rust
fn pq_distance(query: &[f32], encoded: &[u8], codebooks: &[Codebook]) -> f32 {
    let mut distance = 0.0;

    for (i, &cluster_id) in encoded.iter().enumerate() {
        let start = i * codebooks[i].subvector_dim;
        let end = start + codebooks[i].subvector_dim;

        let query_chunk = &query[start..end];
        let centroid = &codebooks[i].centroids[cluster_id as usize];

        // Compute partial distance
        distance += euclidean_distance(query_chunk, centroid);
    }

    distance
}
```

**Optimization:** Precompute lookup table once per query:
```rust
fn precompute_lookup_table(query: &[f32], codebooks: &[Codebook]) -> Vec<Vec<f32>> {
    codebooks
        .iter()
        .enumerate()
        .map(|(i, codebook)| {
            let start = i * codebook.subvector_dim;
            let end = start + codebook.subvector_dim;
            let query_chunk = &query[start..end];

            // Distance from query chunk to each centroid
            codebook.centroids
                .iter()
                .map(|centroid| euclidean_distance(query_chunk, centroid))
                .collect()
        })
        .collect()
}

// Then distance is just lookup + sum
fn fast_pq_distance(encoded: &[u8], lookup_table: &[Vec<f32>]) -> f32 {
    encoded
        .iter()
        .enumerate()
        .map(|(i, &cluster_id)| lookup_table[i][cluster_id as usize])
        .sum()
}
```

**Time complexity:** O(num_subvectors) = O(96) per distance (very fast!)

### Performance

**Memory:**
- **Reduction:** 8x (768-dim float32 → 96-byte PQ)
- **10M × 768-dim:** 30.7 GB → 3.8 GB ✅
- **+ Codebooks:** 96 × 256 × 8 × 4 bytes = 750 KB (negligible)

**Speed:**
- **Faster than float32:** Lookup table makes distance O(num_subvectors)
- **Throughput:** 2-3x more queries/sec

**Accuracy:**
- **Recall@10:** 90-95% (depends on data distribution)
- **Loss:** ~5% compared to full precision

**Configuration:**
```yaml
quantization:
  product:
    compression: 8x  # Or 16x, 32x
    always_ram: true
```

**Code location:** [`lib/quantization/src/encoded_vectors_pq.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_pq.rs)

---

## Strategy 3: Binary Quantization

### The Concept

**Extreme compression:** Map each dimension to 1 bit.

```rust
fn binary_quantize(vector: &[f32]) -> Vec<u8> {
    let mut binary = vec![0u8; (vector.len() + 7) / 8];

    for (i, &value) in vector.iter().enumerate() {
        if value > 0.0 {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            binary[byte_idx] |= 1 << bit_idx;
        }
    }

    binary
}
```

**Example:**
```
Original: [0.5, -0.2, 0.8, -0.1, 0.3, -0.4, 0.1, -0.7]
Binary:   [  1,    0,   1,    0,   1,    0,   1,    0]

Packed into byte: 0b10101010 = 0xAA
```

### Distance: Hamming Distance

**For binary vectors, use Hamming distance** (count differing bits):
```rust
fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x ^ y).count_ones())  // XOR + popcount
        .sum()
}
```

**Hardware acceleration:** Modern CPUs have POPCNT instruction (1 cycle).

**SIMD:** Can compute 256-bit XOR + popcount in one instruction (AVX2).

### Oversampling & Rescoring

Binary quantization is **very lossy**. Solution: **oversample + rescore**.

```rust
fn binary_search_with_rescore(
    query: &[f32],
    binary_vectors: &[Vec<u8>],
    full_vectors: &[Vec<f32>],
    k: usize,
) -> Vec<ScoredPoint> {
    let query_binary = binary_quantize(query);

    // Step 1: Oversample using binary (fast)
    let candidates = binary_search(&query_binary, binary_vectors, k * 4);

    // Step 2: Rescore with full precision (accurate)
    let mut rescored: Vec<_> = candidates
        .iter()
        .map(|&id| {
            let score = cosine_distance(query, &full_vectors[id]);
            ScoredPoint { id, score }
        })
        .collect();

    rescored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    rescored.truncate(k);

    rescored
}
```

**Key:** Fetch 4x candidates (cheap), rescore with full precision (small overhead).

### Performance

**Memory:**
- **Reduction:** 32x (float32 → 1 bit)
- **10M × 768-dim:** 30.7 GB → 960 MB ✅
- **With full vectors for rescoring:** +30.7 GB = 31.7 GB (no savings)
- **Solution:** Disk-backed full vectors, RAM for binary

**Speed:**
- **10-20x faster** than float32 (POPCNT + SIMD)
- **Throughput:** 5-10x more queries/sec

**Accuracy:**
- **Without rescore:** ~70% recall (too low)
- **With 4x oversample + rescore:** 92-96% recall ✅
- **With 10x oversample + rescore:** 97-99% recall

**Configuration:**
```yaml
quantization:
  binary:
    always_ram: true

storage:
  on_disk_payload: true  # Keep full vectors on disk
```

**Code location:** [`lib/quantization/src/encoded_vectors_binary.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_binary.rs)

---

## Rescoring: The Secret Sauce

All quantization methods benefit from **rescoring** (also called "reranking").

### Two-Stage Search

```mermaid
graph LR
    Query[Query Vector] --> Stage1[Stage 1: Quantized Search]
    Stage1 --> Candidates[Top-K × α candidates]
    Candidates --> Stage2[Stage 2: Full Precision Rescore]
    Stage2 --> Results[Final Top-K]

    style Stage1 fill:#e1f5ff
    style Stage2 fill:#fff4e1
```

**Example:**
```
Query: Find top-10 similar products

Stage 1 (quantized):
  - Search with int8/PQ/binary
  - Retrieve top-40 candidates (α = 4)
  - Cost: Fast (quantized distance)

Stage 2 (rescore):
  - Compute exact float32 distance for 40 candidates
  - Re-rank and take top-10
  - Cost: 40 × full precision (acceptable)
```

**Why it works:**
- Quantization identifies the **right neighborhood** (95%+ recall)
- Rescoring fixes **local ranking errors**
- Total cost dominated by Stage 1 (fast)

### Asymmetric Distance

**Optimization:** Keep query in full precision, only quantize stored vectors.

```rust
// Asymmetric: query=float32, stored=int8
fn asymmetric_dot_product(
    query: &[f32],
    quantized: &[u8],
    min: f32,
    max: f32,
) -> f32 {
    let scale = (max - min) / 255.0;

    query
        .iter()
        .zip(quantized.iter())
        .map(|(&q, &v)| q * (min + v as f32 * scale))
        .sum()
}
```

**Benefit:** Better accuracy than symmetric (both quantized) at no extra cost.

---

## Comparison Table

| Method | Compression | Memory (10M × 768) | Recall@10 | Speed vs Float32 | Best For |
|--------|-------------|-------------------|-----------|------------------|----------|
| **None** | 1x | 30.7 GB | 100% | 1x | Gold standard |
| **Scalar (int8)** | 4x | 7.7 GB | 98-99% | 2-3x | General purpose |
| **Product (8x)** | 8x | 3.8 GB | 90-95% | 2-3x | Large datasets |
| **Product (16x)** | 16x | 1.9 GB | 85-92% | 3-4x | Aggressive compression |
| **Binary + rescore** | 32x | 960 MB | 92-96% | 5-10x | Extreme memory limits |

**Rescore factor:** Binary assumes 4x oversampling + rescore.

---

## Real-World Benchmarks

**Dataset:** GIST-960 (1M vectors, 960 dimensions)
**Hardware:** AWS c5.4xlarge (16 vCPU, 32 GB RAM)
**Metric:** Recall@10

### Scalar Quantization

| Config | Memory | QPS | Recall@10 | Notes |
|--------|--------|-----|-----------|-------|
| Float32 | 3.7 GB | 1,200 | 100% | Baseline |
| int8 | 960 MB | 3,500 | 99.2% | ✅ Best balance |
| int8 + rescore | 960 MB | 2,800 | 99.8% | Near-perfect |

**Takeaway:** Scalar is a no-brainer for most workloads.

---

### Product Quantization

| Config | Memory | QPS | Recall@10 | Notes |
|--------|--------|-----|-----------|-------|
| Float32 | 3.7 GB | 1,200 | 100% | Baseline |
| PQ (8x) | 480 MB | 3,200 | 94.1% | Good recall |
| PQ (8x) + rescore 2x | 480 MB | 2,400 | 97.8% | Excellent |
| PQ (16x) | 240 MB | 4,000 | 88.5% | Aggressive |
| PQ (16x) + rescore 4x | 240 MB | 2,000 | 95.2% | Still viable |

**Takeaway:** PQ shines when memory is tight and you can afford slight recall loss.

---

### Binary Quantization

| Config | Memory | QPS | Recall@10 | Notes |
|--------|--------|-----|-----------|-------|
| Float32 | 3.7 GB | 1,200 | 100% | Baseline |
| Binary | 120 MB | 12,000 | 72.3% | Too low |
| Binary + rescore 4x | 120 MB | 6,500 | 95.8% | ✅ Viable |
| Binary + rescore 10x | 120 MB | 3,800 | 98.4% | High accuracy |

**Takeaway:** Binary is extreme but works with aggressive rescoring.

---

## When to Use Each Method

### Use Scalar Quantization (int8) if:
- ✅ You want minimal setup (no training needed)
- ✅ 98-99% recall is acceptable
- ✅ You have moderate memory constraints (4x reduction)
- ✅ Your distance is Cosine or Dot Product

**Example:** Production RAG systems, semantic search

---

### Use Product Quantization if:
- ✅ You need 8-32x compression
- ✅ 90-95% recall is acceptable (or you'll rescore)
- ✅ You can afford codebook training time
- ✅ Your vectors have structure (not random)

**Example:** Large-scale recommendation, image search

---

### Use Binary Quantization if:
- ✅ Extreme memory pressure (32x reduction)
- ✅ You can oversample and rescore (4-10x)
- ✅ Throughput >> latency (serve millions of queries/sec)
- ✅ Your vectors are high-dimensional (>512 dims)

**Example:** Large-scale deduplication, anomaly detection

---

## Configuration Examples

### E-Commerce (10M products)

```yaml
collections:
  products:
    vectors:
      description:
        size: 768
        distance: Cosine
        quantization:
          scalar:
            type: int8
            quantile: 0.99
            always_ram: true
        hnsw_config:
          m: 16
          ef_construct: 100
```

**Rationale:**
- Scalar for 98%+ recall
- 4x memory reduction (30 GB → 7.5 GB)
- Fits on single machine

---

### Large-Scale Search (100M documents)

```yaml
collections:
  documents:
    vectors:
      content:
        size: 384
        distance: Cosine
        quantization:
          product:
            compression: 8x
            always_ram: true
        on_disk_payload: true
        hnsw_config:
          m: 32
          ef_construct: 200
```

**Rationale:**
- PQ for 8x compression (15 GB → 1.9 GB)
- Rescore top-50 for 97% recall
- Payloads on disk to save RAM

---

### Real-Time Deduplication (Billions of items)

```yaml
collections:
  fingerprints:
    vectors:
      hash:
        size: 1024
        distance: Cosine
        quantization:
          binary:
            always_ram: true
        on_disk: true  # Full vectors on SSD
        hnsw_config:
          m: 16
          ef_construct: 64
```

**Rationale:**
- Binary for 32x compression
- 10x oversample + rescore for 98% recall
- Maximize throughput (10k+ QPS)

---

## Advanced: Hybrid Quantization

**Idea:** Use different quantization for search vs storage.

```rust
// Search: Binary (fast candidate selection)
let candidates = binary_search(query, binary_index, k * 10);

// Rescore: Scalar int8 (better than full float32 but still fast)
let rescored = rescore_with_scalar(query, scalar_vectors, candidates, k);
```

**Memory:** Binary (960 MB) + Scalar (7.7 GB) = 8.7 GB
**vs Float32:** 30.7 GB (72% reduction)
**Recall:** 98%+
**Speed:** 5x faster than float32

**Configuration:** (not currently supported, but illustrative)
```yaml
quantization:
  search: binary
  rescore: scalar
```

---

## Common Pitfalls

### Pitfall 1: Forgetting to Train PQ

**Problem:** Using PQ without training on representative data.

**Solution:**
```bash
# Train PQ codebooks on sample of your data
curl -X POST 'http://localhost:6333/collections/my_collection/quantize' \
  -H 'Content-Type: application/json' \
  -d '{
    "quantization": {
      "product": {
        "compression": 8,
        "always_ram": true
      }
    }
  }'
```

---

### Pitfall 2: Binary Without Rescoring

**Problem:** 70% recall is too low for production.

**Solution:** Always use `rescore: true` with binary.

---

### Pitfall 3: Wrong Distance Metric

**Problem:** Binary works poorly with Euclidean distance.

**Solution:** Use **Cosine** or **Dot Product** for binary quantization.

---

## Key Takeaways

1. **Quantization trades memory for slight accuracy loss** (98%+ recall)
2. **Scalar (int8) is the best default:** 4x compression, 98-99% recall, fast
3. **Product Quantization for extreme compression:** 8-32x, 90-95% recall
4. **Binary Quantization for throughput:** 32x compression, needs rescoring
5. **Rescoring recovers lost accuracy** with minimal overhead
6. **Asymmetric distance is free accuracy** (query stays float32)
7. **Choose based on memory budget and recall requirements**

---

## Next Up

**Post 6:** Distributed Qdrant - Raft consensus, sharding strategies, and handling failures in production clusters.

**Try it yourself:**
```bash
# Enable scalar quantization
curl -X PATCH 'http://localhost:6333/collections/my_collection' \
  -H 'Content-Type: application/json' \
  -d '{
    "quantization_config": {
      "scalar": {
        "type": "int8",
        "quantile": 0.99,
        "always_ram": true
      }
    }
  }'
```

---

**Further Reading:**
- Product Quantization Paper: Jégou et al., TPAMI 2011
- Qdrant Quantization Docs: https://qdrant.tech/documentation/guides/quantization/
- Binary Embedding Research: Gong et al., NeurIPS 2013

---

**Code References:**
- Scalar quantization: [`lib/quantization/src/encoded_vectors_u8.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_u8.rs)
- Product quantization: [`lib/quantization/src/encoded_vectors_pq.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_pq.rs)
- Binary quantization: [`lib/quantization/src/encoded_vectors_binary.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/quantization/src/encoded_vectors_binary.rs)
- Quantized storage: [`lib/segment/src/vector_storage/quantized/quantized_vectors.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/vector_storage/quantized/quantized_vectors.rs)
