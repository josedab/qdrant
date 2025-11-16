# HNSW in Production: How Qdrant Implements Fast Vector Search

**Series:** Qdrant Deep Dive (Post 2 of 7)
**Reading Time:** ~20 minutes
**Code Baseline:** Commit [`adcda004057df08389106da56f440db185f0c382`](https://github.com/qdrant/qdrant/commit/adcda004057df08389106da56f440db185f0c382)
**Prerequisites:** Post 1 (Architecture), basic graph theory

---

## What You'll Learn

- What HNSW is and why it's superior to alternatives
- How the graph is constructed (layer selection, link strategy)
- The search algorithm (greedy routing with backtracking)
- Qdrant-specific optimizations
- Trade-offs: accuracy vs speed, memory vs performance
- How to tune HNSW parameters for your workload

---

## The ANN Problem: Why Not Brute Force?

**Scenario:** You have 10 million 384-dimensional vectors. A user queries with a new vector. Find the 10 most similar.

**Brute-force approach:**
```rust
// O(N) linear scan
for each vector in database {
    score = cosine_distance(query, vector);
    keep top-K scores;
}
```

**Cost:** 10 million distance calculations
- Each calculation: ~384 floating-point operations
- Total: ~3.8 billion operations per query
- **Latency: 500-1000ms** (unacceptable for production)

**Solution:** Trade perfect accuracy for speed using **Approximate Nearest Neighbor (ANN)** search.

### ANN Algorithms Compared

| Algorithm | Structure | Search Time | Build Time | Recall | Use Case |
|-----------|-----------|-------------|------------|--------|----------|
| **Brute Force** | Array | O(N) | O(1) | 100% | N < 10k |
| **KD-Tree** | Binary tree | O(log N) | O(N log N) | 100% | Low dim (<20) |
| **LSH** | Hash tables | O(1) | O(N) | ~80% | Extremely large N |
| **HNSW** | Layered graph | O(log N) | O(N log N) | 95-99% | **Production default** |

**Why HNSW wins:**
- ✅ **Fast search**: O(log N) with high constants
- ✅ **High recall**: 95-99% with tuning
- ✅ **Incremental updates**: Add points without rebuilding
- ✅ **Memory efficient**: ~16 bytes per link
- ❌ **Build cost**: Slower than LSH (but builds once)

**Citation:** ["Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs"](https://arxiv.org/abs/1603.09320) by Malkov & Yashunin, 2018.

---

## HNSW: The Core Idea

HNSW combines two concepts:

### 1. **Skip Lists for Graphs**

Think of a skip list, but for graphs:

```
Layer 2:  1 ===================> 100
Layer 1:  1 ======> 25 =======> 100
Layer 0:  1 -> 10 -> 25 -> 50 -> 100
```

**Search:** Start at top layer, move right until you overshoot, then drop down.

**HNSW applies this to graphs:**
- **Layer 0** (bottom): Dense graph, all points, many connections
- **Layers 1, 2, ...** (higher): Sparse graphs, fewer points, "highways"
- **Search:** Start at the top, navigate to approximate area, drop down

### 2. **Small World Networks**

A "small world" network has:
- **High clustering**: Friends of your friends are likely your friends
- **Short path lengths**: Six degrees of separation

**HNSW maintains small-world properties** using a greedy heuristic during construction.

---

## Graph Construction: Building the Index

### Layer Assignment: Exponential Decay

When inserting a point, how many layers should it appear in?

```rust
// From lib/segment/src/index/hnsw_index/hnsw.rs:245
// Simplified for clarity

fn select_layer(ml: f64) -> usize {
    let uniform: f64 = rng.gen(); // Random [0, 1)
    let layer = (-uniform.ln() * ml).floor() as usize;
    layer
}

// ml (max layer) = 1 / ln(M)
// M = number of connections per layer
```

**Result:** Exponentially fewer points at higher layers.

**Example** (M=16, ml ≈ 0.36):
- ~100% of points in Layer 0
- ~36% in Layer 1
- ~13% in Layer 2
- ~5% in Layer 3

**Why exponential?** Guarantees O(log N) search complexity.

---

### Link Selection: Heuristic Pruning

When inserting a point, which neighbors should we link to?

**Naive:** Connect to M nearest neighbors
**Problem:** Creates long-range connections that skip intermediate regions (hurts small-world property)

**HNSW's heuristic** (simplified):
```rust
// From lib/segment/src/index/hnsw_index/hnsw.rs:652

fn select_neighbors_heuristic(
    candidates: Vec<(PointId, f32)>,  // (id, distance)
    m: usize,                         // Max connections
) -> Vec<PointId> {
    let mut result = Vec::new();

    // Sort by distance (closest first)
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for (candidate_id, candidate_dist) in candidates {
        if result.len() >= m {
            break;
        }

        // Add candidate if it's not "behind" an already-selected neighbor
        let mut good = true;
        for &selected_id in &result {
            let selected_to_candidate = distance(selected_id, candidate_id);
            if selected_to_candidate < candidate_dist {
                good = false;  // Candidate is "behind" selected
                break;
            }
        }

        if good {
            result.push(candidate_id);
        }
    }

    result
}
```

**Intuition:** Avoid adding a point if it's "behind" a closer neighbor. This maintains diversity of connections.

**Result:** Graph with short diameter and good clustering.

**Code location:** [`lib/segment/src/index/hnsw_index/hnsw.rs:652`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/hnsw.rs#L652)

---

### Bi-directional Links

HNSW maintains **undirected** edges by updating both nodes:

```rust
// When linking A → B, also update B → A
graph.add_link(point_a, point_b);
graph.add_link(point_b, point_a);

// But enforce max degree M
if graph.degree(point_b) > M {
    graph.prune_links(point_b, M);
}
```

**Why bidirectional?** Ensures reachability from any starting point.

---

### Construction Complexity

**Time:** O(N log N)
- Each insert: O(log N) to find entry point + O(M × ef_construct) for neighbor search
- N inserts: O(N log N)

**Space:** O(N × M)
- Each point stores M links per layer
- Average ~2-3 layers per point
- Total: ~2M links per point × 8 bytes = ~256 bytes per point (M=16)

**Parallelization:** Qdrant builds HNSW using Rayon for parallel construction.

**Code location:** [`lib/segment/src/index/hnsw_index/graph_layers_builder.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/graph_layers_builder.rs)

---

## Search Algorithm: Navigating the Graph

### Greedy Search (Simplified)

```rust
// Conceptual pseudocode from lib/segment/src/index/hnsw_index/hnsw.rs:412

fn search_layer(
    query: &[f32],
    entry_points: Vec<PointId>,
    num_closest: usize,
    layer: usize,
) -> Vec<PointId> {
    let mut candidates = BinaryHeap::new();  // Min-heap by distance
    let mut visited = HashSet::new();
    let mut result = BinaryHeap::new();      // Max-heap (top-K)

    // Initialize with entry points
    for ep in entry_points {
        let dist = distance(query, ep);
        candidates.push((dist, ep));
        result.push((dist, ep));
        visited.insert(ep);
    }

    while let Some((dist, current)) = candidates.pop() {
        // If current is farther than worst result, stop
        if dist > result.peek().unwrap().0 {
            break;
        }

        // Explore neighbors
        for &neighbor in graph.neighbors(current, layer) {
            if visited.contains(&neighbor) {
                continue;
            }
            visited.insert(neighbor);

            let neighbor_dist = distance(query, neighbor);

            // Add to candidates if promising
            if neighbor_dist < result.peek().unwrap().0 || result.len() < num_closest {
                candidates.push((neighbor_dist, neighbor));
                result.push((neighbor_dist, neighbor));

                // Keep only top-K
                if result.len() > num_closest {
                    result.pop();
                }
            }
        }
    }

    result.into_sorted_vec()
}
```

**Key insight:** Greedy best-first search with a dynamic candidate list.

**Parameters:**
- **ef** (exploration factor): Size of candidate list
  - Higher ef → more exploration → better recall
  - Lower ef → faster search → lower recall
- **num_closest**: K (number of results desired)

**Typical:** ef = 32 for ~95% recall, ef = 128 for ~99% recall

---

### Multi-Layer Search

```rust
fn search(query: &[f32], k: usize, ef: usize) -> Vec<PointId> {
    let entry_point = get_entry_point();  // Top-most layer
    let mut current_layer = max_layer;
    let mut entry_points = vec![entry_point];

    // Traverse layers top-down
    while current_layer > 0 {
        entry_points = search_layer(query, entry_points, 1, current_layer);
        current_layer -= 1;
    }

    // Final search on layer 0 (dense layer)
    search_layer(query, entry_points, k, 0)
}
```

**Why multi-layer?**
- **Upper layers**: Quickly navigate to the right region (coarse navigation)
- **Layer 0**: Fine-grained search for exact nearest neighbors

**Time complexity:** O(log N) expected

**Code location:** [`lib/segment/src/index/hnsw_index/hnsw.rs:412`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/hnsw.rs#L412)

---

## Qdrant-Specific Optimizations

### 1. Multi-Entry Points

**Standard HNSW:** Single entry point at the top layer
**Qdrant enhancement:** Multiple entry points

```rust
// lib/segment/src/index/hnsw_index/entry_points.rs

struct EntryPoints {
    entry_points: Vec<PointId>,  // Multiple starting points
}
```

**Why?** Graph might have disconnected components or suboptimal structure. Multiple entry points improve worst-case recall.

**Code:** [`lib/segment/src/index/hnsw_index/entry_points.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/entry_points.rs)

---

### 2. Search Context & Filtering

**Challenge:** How to search with filters? (e.g., "Find nearest where price < 50")

**Options:**
1. **Post-filter:** Search first, filter results (low recall if filter is selective)
2. **Pre-filter:** Use field index first, then search (expensive if many matches)
3. **Filtered HNSW:** Apply filter during graph traversal (Qdrant's approach)

**Filtered search:**
```rust
fn search_filtered(
    query: &[f32],
    filter: &Filter,
    ef: usize,
) -> Vec<PointId> {
    // ... (same as before, but skip filtered-out points)

    for &neighbor in graph.neighbors(current, layer) {
        if !filter.check(neighbor) {
            continue;  // Skip points that don't match filter
        }

        // ... (rest of search logic)
    }
}
```

**Trade-off:** Filtering reduces effective graph connectivity, so you may need higher `ef`.

**Code:** [`lib/segment/src/index/hnsw_index/search_context.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/search_context.rs)

---

### 3. GPU Acceleration (Optional)

**When enabled** (`--features gpu`):
- Distance calculations on GPU (CUDA/ROCm)
- Batch computation for throughput
- Significantly faster on large batches

**Code:** [`lib/segment/src/index/hnsw_index/gpu/`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/gpu/)

**Trade-off:** GPU helps with throughput (many queries) but adds latency overhead for single queries.

---

### 4. Build Cache

During construction, Qdrant caches intermediate structures:

```rust
// lib/segment/src/index/hnsw_index/build_cache.rs

struct BuildCache {
    // Cache computed distances to avoid recomputation
    distance_cache: HashMap<(PointId, PointId), f32>,
}
```

**Benefit:** Speeds up construction by ~20-30%

**Code:** [`lib/segment/src/index/hnsw_index/build_cache.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/build_cache.rs)

---

## Parameter Tuning Guide

### Build-Time Parameters

| Parameter | Default | Effect | Tune for... |
|-----------|---------|--------|-------------|
| **M** | 16 | Connections per node | Higher M → better recall, more memory |
| **ef_construct** | 100 | Build candidate list size | Higher → better graph quality, slower build |
| **max_indexing_threads** | 0 (auto) | Parallel build threads | More threads → faster build |

**Example (high recall):**
```yaml
hnsw_index:
  m: 32
  ef_construct: 200
```

**Example (fast build):**
```yaml
hnsw_index:
  m: 12
  ef_construct: 64
```

---

### Search-Time Parameters

| Parameter | Default | Effect | Tune for... |
|-----------|---------|--------|-------------|
| **ef** | 32 | Search candidate list size | Higher → better recall, slower search |
| **limit** | - | Number of results (K) | More results → longer search |

**API Example:**
```json
{
  "vector": [0.1, 0.2, ...],
  "limit": 10,
  "params": {
    "hnsw_ef": 128
  }
}
```

**Recall vs ef:**
| ef | Recall | Search Time |
|----|--------|-------------|
| 16 | ~90% | 1x |
| 32 | ~95% | 1.5x |
| 64 | ~97% | 2x |
| 128 | ~99% | 3x |

**Note:** Results vary by dataset. Always benchmark with your data.

---

## Accuracy vs Speed Trade-offs

### HNSW vs Exact Search

```
Exact (brute force):
- Recall: 100%
- Latency: 500ms (10M vectors)

HNSW (M=16, ef=32):
- Recall: 95%
- Latency: 5ms (100x faster)

HNSW (M=32, ef=128):
- Recall: 99%
- Latency: 15ms (33x faster)
```

**When to use exact:**
- N < 10,000 vectors
- 100% recall required
- Offline batch processing

**When to use HNSW:**
- N > 10,000 vectors
- Real-time queries (< 100ms)
- Acceptable to miss 1-5% of perfect results

### Qdrant's Automatic Fallback

```yaml
storage:
  hnsw_index:
    full_scan_threshold_kb: 10000
```

**Behavior:** If estimated query matches < 10,000 KB of vectors, use brute force.

**Why?** For small result sets, HNSW overhead isn't worth it.

**Code:** [`lib/segment/src/index/query_optimization/optimizer.rs:187`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/query_optimization/optimizer.rs#L187)

---

## Memory Considerations

### Memory per Point

```
HNSW memory:
- Layer 0: M₀ links × 8 bytes = 16 × 8 = 128 bytes
- Layer 1: M links × 8 bytes = 16 × 8 = 128 bytes
- Average layers per point: 1.5
- Total: ~200 bytes per point
```

**For 10M points:**
- HNSW index: ~2 GB
- Vector data (384-dim, float32): 384 × 4 × 10M = 15 GB
- **Total: ~17 GB** (without quantization)

**With quantization (scalar):**
- Vector data: 384 × 1 × 10M = 3.8 GB
- **Total: ~6 GB** (71% reduction)

See Post 5 for quantization deep-dive.

---

## Graph Healer: Maintaining Quality

Over time, deletions and updates can degrade graph quality.

**Solution:** Background "graph healer"

```rust
// lib/segment/src/index/hnsw_index/graph_layers_healer.rs

fn heal_graph(graph: &mut Graph) {
    for point in graph.points() {
        if graph.is_deleted(point) {
            // Remove from neighbors' lists
            graph.remove_point(point);
        }

        // Check connectivity
        if graph.degree(point) < M / 2 {
            // Re-link to restore connectivity
            graph.rebuild_links(point);
        }
    }
}
```

**Triggered by:**
- Segment optimization
- Manual rebuild

**Code:** [`lib/segment/src/index/hnsw_index/graph_layers_healer.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index/graph_layers_healer.rs)

---

## Comparing HNSW Implementations

| Implementation | Language | Features | Production-Ready |
|----------------|----------|----------|------------------|
| **Qdrant** | Rust | Multi-entry, filtered search, GPU | ✅ |
| **FAISS (Meta)** | C++ | Extensive optimizations, GPU | ✅ |
| **hnswlib** | C++ | Lightweight, Python bindings | ✅ |
| **Annoy (Spotify)** | C++ | Trees, not HNSW | ✅ |
| **ScaNN (Google)** | C++ | Quantization + pruning | ✅ |

**Qdrant advantages:**
- Memory safety (Rust)
- Integrated filtering (payload indexes)
- Distributed deployment
- Full database features (persistence, replication)

**FAISS advantages:**
- More indexing algorithms
- Extremely optimized SIMD
- Larger research backing

**Use Qdrant when:** You need a complete vector database
**Use FAISS when:** You only need ANN (no persistence, no filtering)

---

## Debugging & Monitoring

### Inspecting the Graph

Qdrant provides debugging tools:

```bash
# Inspect segment HNSW index (requires service_debug feature)
./target/debug/segment_inspector \
  --segment-path ./storage/collections/my_collection/0/segments/... \
  --hnsw-stats
```

**Output:**
- Layer distribution
- Average degree per layer
- Disconnected components
- Entry point quality

---

### Metrics

Prometheus metrics for HNSW:

```
# Number of HNSW index builds
qdrant_hnsw_builds_total

# Search duration quantiles
qdrant_hnsw_search_duration_seconds{quantile="0.95"}

# Graph statistics
qdrant_hnsw_graph_size_bytes
qdrant_hnsw_average_degree
```

**Code:** [`src/common/metrics.rs`](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/src/common/metrics.rs)

---

## Common Pitfalls & Solutions

### Pitfall 1: Low Recall with Filters

**Problem:** Filtered search only returns 2 results when you asked for 10.

**Cause:** Filter eliminates most graph, breaking connectivity.

**Solution:**
1. Increase `ef` significantly (2-4x)
2. Use more selective pre-filters
3. Consider separate indexes per category

---

### Pitfall 2: Slow Build Times

**Problem:** Building HNSW on 10M points takes hours.

**Solutions:**
- Increase `max_indexing_threads` (use more CPUs)
- Lower `ef_construct` (trades quality for speed)
- Build in batches, merge segments later

---

### Pitfall 3: Memory Explosion

**Problem:** HNSW index uses more RAM than expected.

**Cause:** High M or many layers.

**Solutions:**
- Lower M (12-16 is usually sufficient)
- Enable quantization (reduces vector memory)
- Use `on_disk: true` for HNSW (experimental)

---

## Key Takeaways

1. **HNSW is a layered graph** enabling O(log N) search
2. **Construction uses heuristic pruning** to maintain small-world property
3. **Search is greedy best-first** from top layer to bottom
4. **Tune M, ef_construct, ef** for your recall/latency requirements
5. **Qdrant adds multi-entry, filtering, GPU** optimizations
6. **Trade-off: ~95-99% recall for 100x speedup**

---

## Next Up

**Post 3:** Patterns and Practices - How Qdrant uses Rust traits, zero-cost abstractions, and modular architecture.

**Try it:**
```bash
# Create collection with custom HNSW params
curl -X PUT 'http://localhost:6333/collections/tuned' \
  -H 'Content-Type: application/json' \
  -d '{
    "vectors": {"size": 384, "distance": "Cosine"},
    "hnsw_config": {"m": 32, "ef_construct": 200}
  }'
```

---

**Further Reading:**
- HNSW Paper: https://arxiv.org/abs/1603.09320
- Qdrant HNSW Code: [`lib/segment/src/index/hnsw_index/`](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/segment/src/index/hnsw_index)
- Benchmark Comparisons: https://qdrant.tech/benchmarks/

---

**Questions?** Ask in the comments or join the [Qdrant Discord](https://qdrant.to/discord).
