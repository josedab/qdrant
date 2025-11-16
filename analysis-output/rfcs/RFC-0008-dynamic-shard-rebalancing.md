# RFC-0008: Automatic Shard Rebalancing

**Status:** Draft
**Category:** Long-term
**Estimated Effort:** 35-45 dev-days

## Summary
Automatically rebalance shards based on load and capacity.

## Motivation
- Manual shard management is tedious
- Uneven load distribution
- Can't auto-scale

## Design
```yaml
rebalancing:
  enabled: true
  triggers:
    - cpu_threshold: 80%
    - memory_threshold: 75%
    - query_latency_p99: 100ms
  
  strategy: "least_loaded"
```

### Algorithm
1. Monitor shard metrics
2. Detect imbalance (CPU, memory, QPS)
3. Select shard to move
4. Transfer to least-loaded node
5. Verify and commit

**Challenges:**
- Consensus on rebalancing decisions
- Minimize disruption during transfer
- Cost estimation

**Effort:** 35-45 dev-days
