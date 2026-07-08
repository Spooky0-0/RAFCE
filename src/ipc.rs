use libc;
use std::ptr;
use std::io;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use crate::types::AuditRecord;

// -----------------------------------------------------------------------------
// Step 2: The Atomic Pointer Structures (Mechanical Sympathy)
// -----------------------------------------------------------------------------

/// Forces a 64-byte alignment (typical L1 cache line size on x86_64/ARM).
/// This entirely eliminates false sharing between producer and consumer cores.
#[repr(C, align(64))]
pub struct CacheAlignedIndex {
    pub sequence: AtomicUsize,
}

#[repr(C, align(64))]
pub struct CacheAlignedHeartbeat {
    pub timestamp_ns: AtomicU64,
}

#[repr(C)]
pub struct RingBufferHeader {
    pub producer_idx: CacheAlignedIndex, // Offset 0
    pub consumer_idx: CacheAlignedIndex, // Offset 64
    pub rafce_heartbeat: CacheAlignedHeartbeat, // Offset 128 (Isolated cache line)
    // Followed immediately by the contiguous array of AuditRecords in memory...
}

/// A wrapper for the memory-mapped SPSC lock-free ring buffer.
pub struct IpcRingBuffer {
    ptr: *mut u8,
    size: usize,
    capacity: usize, // Total number of AuditRecord slots
}

impl IpcRingBuffer {
    // -----------------------------------------------------------------------------
    // Step 1: The mmap Bootstrap Code (Targeting 2MB Huge Pages)
    // -----------------------------------------------------------------------------
    
    /// Initializes the shared memory segment mapping 2MB Huge Pages.
    /// `capacity` is the exact number of AuditRecord slots.
    pub fn new(capacity: usize) -> io::Result<Self> {
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let payload_size = capacity * std::mem::size_of::<AuditRecord>();
        let total_size = header_size + payload_size;
        
        // Align total size up to the nearest 2MB boundary (Huge Page size)
        let huge_page_size = 2 * 1024 * 1024;
        let mapped_size = (total_size + huge_page_size - 1) & !(huge_page_size - 1);

        // Required flags for a shared, anonymous HugeTLB mapping
        // We use MAP_ANONYMOUS | MAP_SHARED because we intend to fork or pass the fd via Unix domain socket in a real system,
        // or alternatively map a specific tmpfs path if using shm_open. 
        // For this bare-metal zero-copy implementation, we assume fork-based IPC or passing the memory reference directly.
        let flags = libc::MAP_SHARED | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB;
        
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                mapped_size,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1, // fd is -1 for MAP_ANONYMOUS
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        // Initialize the atomic headers to 0
        unsafe {
            let header = &mut *(ptr as *mut RingBufferHeader);
            header.producer_idx.sequence.store(0, Ordering::Relaxed);
            header.consumer_idx.sequence.store(0, Ordering::Relaxed);
            header.rafce_heartbeat.timestamp_ns.store(0, Ordering::Relaxed);
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            size: mapped_size,
            capacity,
        })
    }

    fn header(&self) -> &RingBufferHeader {
        unsafe { &*(self.ptr as *const RingBufferHeader) }
    }

    fn payload_ptr(&self) -> *mut AuditRecord {
        unsafe { self.ptr.add(std::mem::size_of::<RingBufferHeader>()) as *mut AuditRecord }
    }

    // -----------------------------------------------------------------------------
    // Heartbeat Monitoring
    // -----------------------------------------------------------------------------

    /// Called continuously during the consumer's spin loop
    pub fn emit_heartbeat(&self, current_time_ns: u64) {
        self.header().rafce_heartbeat.timestamp_ns.store(current_time_ns, Ordering::Relaxed);
    }

    /// The DEC hot-path checks the heartbeat periodically.
    pub fn check_auditor_health(&self, current_time_ns: u64) -> bool {
        const MAX_HEARTBEAT_STALL_NS: u64 = 500_000_000; // 500 milliseconds
        let last_pulse = self.header().rafce_heartbeat.timestamp_ns.load(Ordering::Relaxed);
        
        // If last_pulse is 0, RAFCE might not have started yet, but let's assume strict monitoring once active.
        if last_pulse > 0 && current_time_ns.saturating_sub(last_pulse) > MAX_HEARTBEAT_STALL_NS {
            // RAFCE is clinically dead. 
            // Trigger the "Auditor Offline" protocol.
            return false;
        }
        true
    }

    // -----------------------------------------------------------------------------
    // Step 3: Hardware Memory Barriers (Strict Release/Acquire semantics)
    // -----------------------------------------------------------------------------

    /// Producer (DEC): Pushes a record to the ring buffer via Ordering::Release.
    /// Returns false if the buffer is 90% full (Circuit Breaker threshold).
    pub fn push(&self, record: &AuditRecord) -> Result<(), &'static str> {
        let head = self.header().producer_idx.sequence.load(Ordering::Relaxed);
        let tail = self.header().consumer_idx.sequence.load(Ordering::Acquire); // See where the consumer is

        // Circuit breaker logic: 90% full
        let used_slots = head.wrapping_sub(tail);
        if used_slots >= (self.capacity * 90) / 100 {
            return Err("Circuit Breaker Triggered: Buffer 90% Full");
        }

        let slot_idx = head % self.capacity;
        
        // Write the payload directly into physical memory
        unsafe {
            let slot_ptr = self.payload_ptr().add(slot_idx);
            ptr::write_volatile(slot_ptr, *record);
        }

        // Publish the sequence update using Ordering::Release.
        // This memory barrier guarantees the CPU flushes the payload to RAM
        // *before* the consumer can observe the updated sequence number.
        self.header().producer_idx.sequence.store(head.wrapping_add(1), Ordering::Release);
        
        Ok(())
    }

    /// Consumer (RAFCE): Polls and reads a record via Ordering::Acquire.
    pub fn pop(&self) -> Option<AuditRecord> {
        let tail = self.header().consumer_idx.sequence.load(Ordering::Relaxed);
        
        // Read the producer's sequence using Ordering::Acquire.
        // This ensures the CPU does not read the payload data from cache out-of-order 
        // before confirming the producer has actually updated the sequence.
        let head = self.header().producer_idx.sequence.load(Ordering::Acquire);

        if tail == head {
            return None; // Buffer is empty, consumer should spin-wait
        }

        let slot_idx = tail % self.capacity;
        
        // Read the payload directly from physical memory
        let record = unsafe {
            let slot_ptr = self.payload_ptr().add(slot_idx);
            ptr::read_volatile(slot_ptr)
        };

        // Advance the tail pointer so the producer can reclaim the slot
        self.header().consumer_idx.sequence.store(tail.wrapping_add(1), Ordering::Release);
        
        Some(record)
    }
}

impl Drop for IpcRingBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}
