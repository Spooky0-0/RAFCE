# Security Policy

This is an academic/portfolio project demonstrating ultra-low latency architectural patterns for High-Frequency Trading (HFT) and Financial Compliance. It is not currently deployed in a production financial environment.

However, security and memory safety are treated with the highest priority, especially given the heavy use of `#[repr(C)]` layouts, `unsafe` memory mapping, and physical atomic barriers.

## Reporting a Vulnerability

If you are a security researcher and discover a memory-safety bug, a false-sharing vulnerability, or an endianness corruption vector within the core engine, please responsibly disclose it.

You can open an issue on this repository or reach out directly to the maintainers. Do not exploit the vulnerability in a way that disrupts the repository or test harnesses.

**Scope of vulnerabilities particularly of interest:**
- Memory leaks in the `mmap` unmapping cycle.
- Out-of-bounds pointer arithmetic in the `IpcRingBuffer`.
- Undefined behavior resulting from relaxed atomic orderings in the disruptor pattern.
