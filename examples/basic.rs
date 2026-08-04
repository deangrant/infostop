//! Basic Infostop usage on a synthetic Euclidean trajectory.

use infostop::{Infostop, MetricKind};

fn main() -> infostop::Result<()> {
    let mut model = Infostop::builder()
        .r1(5.0)
        .r2(10.0)
        .distance_metric(MetricKind::Euclidean)
        .min_size(2)
        .build()?;

    // Stay near origin, short trip, stay near x=100.
    let mut trace = Vec::new();
    for i in 0..10 {
        trace.push([0.0, f64::from(i) * 0.05]);
    }
    trace.push([40.0, 0.0]);
    trace.push([70.0, 0.0]);
    for i in 0..10 {
        trace.push([100.0, f64::from(i) * 0.05]);
    }

    let labels = model.fit_predict(&trace)?;
    println!("points: {}", labels.len());
    println!("labels: {labels:?}");

    let medians = model.label_medians()?;
    println!("stop medians:");
    let mut keys: Vec<_> = medians.keys().copied().collect();
    keys.sort_unstable();
    for k in keys {
        let p = medians[&k];
        println!("  stop {k}: ({:.4}, {:.4})", p.x, p.y);
    }
    Ok(())
}
