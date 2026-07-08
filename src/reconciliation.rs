use crate::types::{AuditRecord, AuditResult};

pub struct ComplianceAuditor<'a> {
    record_buffer: &'a [AuditRecord],
}

impl<'a> ComplianceAuditor<'a> {
    pub const fn new(buffer: &'a [AuditRecord]) -> Self {
        Self { record_buffer: buffer }
    }

    /// Performs an in-place, zero-allocation volumetric reconciliation check
    /// over the entire execution window. Time Complexity: O(N), Space Complexity: O(1)
    pub fn verify_conservation_invariants(&self) -> AuditResult {
        let mut idx = 0;
        let len = self.record_buffer.len();
        let mut total_dec: u64 = 0;
        let mut total_dcse: u64 = 0;

        while idx < len {
            let record = &self.record_buffer[idx];
            
            // Fast-path safety guard: Detect wash trading patterns preserved in logs
            if record.dec_volume > 0 && record.dcse_settled_volume == 0 {
                return AuditResult::ComplianceBreach;
            }

            total_dec += record.dec_volume;
            total_dcse += record.dcse_settled_volume;
            idx += 1;
        }

        if total_dec == total_dcse {
            AuditResult::Success
        } else {
            AuditResult::VolumetricMismatch {
                delta: total_dec.saturating_sub(total_dcse),
            }
        }
    }
}
