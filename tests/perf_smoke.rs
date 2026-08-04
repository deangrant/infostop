//! Performance smoke tests (ignored by default).
//!
//! Run with: `cargo test --test perf_smoke -- --ignored`

use infostop::{Infostop, MetricKind};

/// Build ~1000 stay-forming Euclidean points in a few clusters and fit Infostop.
#[test]
#[ignore = "run with cargo test -- --ignored"]
fn smoke_fit_predict_thousand_points() {
    let mut trace = Vec::with_capacity(1200);
    // 4 loci × 250 samples within r1
    for locus in 0..4 {
        let base = f64::from(locus) * 100.0;
        for i in 0..250 {
            trace.push([base, f64::from(i) * 0.001]);
        }
    }

    let mut model = Infostop::builder()
        .r1(1.0)
        .r2(5.0)
        .distance_metric(MetricKind::Euclidean)
        .min_size(2)
        .seed(42)
        .build()
        .unwrap();

    let labels = model.fit_predict(&trace).unwrap();
    assert_eq!(labels.len(), trace.len());
    assert!(labels.iter().any(|&l| l >= 0));
    assert!(!model.label_medians().unwrap().is_empty());
}
