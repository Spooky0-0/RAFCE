#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    Success,
    VolumetricMismatch { delta: u64 },
    ComplianceBreach,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuditRecord {
    pub trade_id: u32,
    pub dec_volume: u64,
    pub dcse_settled_volume: u64,
    pub timestamp_ns: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SnapshotRecord {
    pub account_id: u32,
    pub balance: u64,
    pub timestamp_ns: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WalEntry {
    pub trade_id: u32,
    pub delta: i64, // positive for credit, negative for debit
    pub timestamp_ns: u64,
}
