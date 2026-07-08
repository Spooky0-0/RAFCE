# Regulatory Audit & Financial Compliance Engine (RAFCE)

RAFCE is a highly-optimized, zero-allocation regulatory compliance layer designed to run alongside the Deterministic Exchange Core (DEC) and the Distributed Clearing & Settlement Engine (DCSE). 

It acts as the definitive verification layer, ensuring that every order execution matches a settled transaction, providing microsecond-accurate trade reconstruction for regulatory bodies (like FINRA CAT), and catching wash-trading violations without pausing the high-frequency matching engine.

## Core Architecture

RAFCE is built entirely in Rust (`1.94+`) with an aggressive zero-heap allocation profile. It interacts with the DEC via a shared-memory lock-free ring buffer (LMAX Disruptor pattern) to achieve sub-microsecond Inter-Process Communication (IPC).

### Key Components

- **Volumetric Reconciler (`src/reconciliation.rs`)**: Verifies the core conservation law ($\sum \text{Executed\_Volume}_{DEC} \equiv \sum \text{Settled\_Amount}_{DCSE}$). Operates in strictly $\mathcal{O}(N)$ linear time utilizing 100% cache-hit sequential scans over memory-mapped arrays.
- **Temporal Time-Travel (`src/time_travel.rs`)**: Reconstructs exact microsecond state leveraging $\mathcal{O}(\log S)$ binary partitioning over monotonic historical snapshots.
- **Regulator Exporter (`src/cat_exporter.rs`)**: A zero-copy serialization engine formatting records into FINRA CAT CSV specifications. Uses `itoa` for direct byte-slicing into a rolling OS-level `memmap2` buffer, completely eliminating string allocation overhead.
- **Zero-Copy IPC (`src/ipc.rs`)**: Maps a 1GB ring buffer backed by 2MB Huge Pages (`MAP_HUGETLB`) to eliminate Translation Lookaside Buffer (TLB) misses. Uses cache-aligned atomics (`#[repr(align(64))]`) to prevent false sharing between the producer (DEC) and consumer (RAFCE).

## Build & Testing

The system is tested extensively using property-based chaos testing (`proptest`) to inject synthetic anomalies like wash-trades and volume leaks.

```bash
# Run unit tests and chaos compliance tests
cargo test

# Run the Criterion micro-benchmark suite
cargo bench
```

## Security & Concurrency Safety

- **Data Races**: Bypassed entirely through `Ordering::Release` and `Ordering::Acquire` memory barriers wrapping volatile physical memory pointer accesses.
- **Slow Consumer Breaches**: A strict 90% capacity Circuit Breaker halts execution gateway ingestion if the audit layer is stalling, mathematically guaranteeing that federal compliance logs are never overwritten or dropped during extreme volatility spikes.

## License
Proprietary.
