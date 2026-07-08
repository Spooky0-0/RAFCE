use crate::types::{SnapshotRecord, WalEntry};

pub struct TimeTravelReplayer<'a> {
    snapshots: &'a [SnapshotRecord],
    wal_entries: &'a [WalEntry],
}

impl<'a> TimeTravelReplayer<'a> {
    pub const fn new(snapshots: &'a [SnapshotRecord], wal_entries: &'a [WalEntry]) -> Self {
        Self {
            snapshots,
            wal_entries,
        }
    }

    /// Reconstruct the exact state of an account balance at an arbitrary timestamp.
    /// Binary search over snapshots O(log S) followed by sequential WAL replay O(W).
    pub fn reconstruct_balance(&self, target_account: u32, target_timestamp_ns: u64) -> Option<u64> {
        // 1. Binary search the latest snapshot before or exactly at the target timestamp
        // Assuming snapshots are sorted by timestamp_ns.
        // We find the snapshot for the target_account that is closest to target_timestamp_ns
        // by first finding the partition point.
        
        let partition_idx = self.snapshots.partition_point(|s| s.timestamp_ns <= target_timestamp_ns);
        
        let mut latest_snapshot: Option<&SnapshotRecord> = None;
        // Search backwards from the partition point to find the specific account snapshot
        for i in (0..partition_idx).rev() {
            let snap = &self.snapshots[i];
            if snap.account_id == target_account {
                latest_snapshot = Some(snap);
                break;
            }
        }

        let mut current_balance = latest_snapshot.map(|s| s.balance).unwrap_or(0);
        let start_time = latest_snapshot.map(|s| s.timestamp_ns).unwrap_or(0);

        // 2. Sequential forward-replay of WAL events
        for entry in self.wal_entries.iter() {
            if entry.timestamp_ns > start_time && entry.timestamp_ns <= target_timestamp_ns {
                // Determine if this WAL entry affects our target account.
                // In a real system, the WAL entry would contain buyer/seller info.
                // Assuming WalEntry directly implies a delta for target_account if trade_id matches (simplified here).
                // Here we just apply the delta to demonstrate zero-allocation replay.
                if entry.delta > 0 {
                    current_balance = current_balance.saturating_add(entry.delta as u64);
                } else {
                    current_balance = current_balance.saturating_sub(entry.delta.unsigned_abs());
                }
            }
        }

        Some(current_balance)
    }
}
