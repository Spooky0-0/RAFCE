use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rafce::reconciliation::ComplianceAuditor;
use rafce::types::{AuditRecord, AuditResult};

fn generate_synthetic_records(count: usize) -> Vec<AuditRecord> {
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        records.push(AuditRecord {
            trade_id: i as u32,
            dec_volume: 100,
            dcse_settled_volume: 100,
            timestamp_ns: i as u64 * 1000,
        });
    }
    records
}

fn bench_reconciliation(c: &mut Criterion) {
    let records = generate_synthetic_records(1_000_000); // 1 million records
    
    c.bench_function("verify_conservation_invariants_1m", |b| {
        b.iter(|| {
            let auditor = ComplianceAuditor::new(black_box(&records));
            let result = auditor.verify_conservation_invariants();
            assert_eq!(result, AuditResult::Success);
        })
    });
}

criterion_group!(benches, bench_reconciliation);
criterion_main!(benches);
