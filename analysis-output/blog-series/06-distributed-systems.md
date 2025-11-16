# Distributed Qdrant: Raft Consensus, Sharding, and Replication

**Series:** Qdrant Deep Dive (Post 6 of 7)

## Summary

Production distributed deployment:

**Raft Consensus:**
- Leader election
- Log replication
- Strong consistency guarantees

**Sharding Strategy:**
- Hash ring distribution
- Consistent hashing
- Dynamic resharding

**Replication:**
- Configurable replication factor
- Write consistency factor
- Failure recovery

**Durability:**
- Write-Ahead Log (WAL)
- RocksDB persistence
- Snapshot/restore

**Failure Modes:**
- Node failure → replica promotion
- Network partition → Raft handles
- Split-brain prevention

**Code:** [src/consensus.rs](https://github.com/qdrant/qdrant/blob/adcda004057df08389106da56f440db185f0c382/src/consensus.rs), [lib/collection/src/shards/](https://github.com/qdrant/qdrant/tree/adcda004057df08389106da56f440db185f0c382/lib/collection/src/shards)
