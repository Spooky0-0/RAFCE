use proptest::prelude::*;
use rafce::reconciliation::ComplianceAuditor;
use rafce::types::{AuditRecord, AuditResult};

proptest! {
    #[test]
    fn test_conservation_invariants(
        dec_volumes in prop::collection::vec(0..1000u64, 0..100),
        dcse_volumes in prop::collection::vec(0..1000u64, 0..100)
    ) {
        let len = std::cmp::max(dec_volumes.len(), dcse_volumes.len());
        let mut records = Vec::with_capacity(len);
        
        let mut expected_dec_total: u64 = 0;
        let mut expected_dcse_total: u64 = 0;
        
        for i in 0..len {
            let dec_v = *dec_volumes.get(i).unwrap_or(&0);
            let dcse_v = *dcse_volumes.get(i).unwrap_or(&0);
            
            // Skip wash trade patterns for the volume matching test,
            // or if it triggers compliance breach, we handle it separately.
            if dec_v > 0 && dcse_v == 0 {
                continue; 
            }
            
            expected_dec_total += dec_v;
            expected_dcse_total += dcse_v;
            
            records.push(AuditRecord {
                trade_id: i as u32,
                dec_volume: dec_v,
                dcse_settled_volume: dcse_v,
                timestamp_ns: i as u64,
            });
        }
        
        let auditor = ComplianceAuditor::new(&records);
        let result = auditor.verify_conservation_invariants();
        
        if expected_dec_total == expected_dcse_total {
            assert_eq!(result, AuditResult::Success);
        } else {
            assert_eq!(
                result, 
                AuditResult::VolumetricMismatch { 
                    delta: expected_dec_total.saturating_sub(expected_dcse_total) 
                }
            );
        }
    }

    #[test]
    fn test_wash_trade_detection(
        dec_vol in 1..1000u64,
        padding in prop::collection::vec(
            (1..1000u64, 1..1000u64), 0..50
        )
    ) {
        let mut records = Vec::new();
        // Add valid records
        for (i, (d, s)) in padding.iter().enumerate() {
            records.push(AuditRecord {
                trade_id: i as u32,
                dec_volume: *d,
                dcse_settled_volume: *s,
                timestamp_ns: 0,
            });
        }
        
        // Add one wash trade record
        records.push(AuditRecord {
            trade_id: 999,
            dec_volume: dec_vol,
            dcse_settled_volume: 0,
            timestamp_ns: 100,
        });
        
        let auditor = ComplianceAuditor::new(&records);
        let result = auditor.verify_conservation_invariants();
        
        assert_eq!(result, AuditResult::ComplianceBreach);
    }
}
