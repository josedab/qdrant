# Quantization Strategies: Achieving 97% Memory Reduction

**Series:** Qdrant Deep Dive (Post 5 of 7)

## Summary

Deep dive into vector compression:
1. **Scalar Quantization**: float32 → int8 (4x compression)
2. **Product Quantization**: Subvector clustering (8-32x compression)
3. **Binary Quantization**: Extreme compression (32x)

**Trade-offs:**
- Memory vs accuracy
- Search speed vs recall
- Re-scoring strategies

**Benchmarks (10M vectors, 384-dim):**
- Unquantized: 15 GB, 100% recall
- Scalar: 3.8 GB (74% reduction), 99% recall
- Product (8x8): 2 GB (87% reduction), 95% recall
- Binary: 480 MB (97% reduction), 90% recall

**Code:** [lib/quantization/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/quantization)
