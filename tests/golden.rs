//! In-crate golden / regression fixtures for Infostop labeling.

use infostop::{Infostop, MetricKind, NON_STOP};

fn euclidean_two_stops() -> Vec<[f64; 2]> {
    let mut trace = Vec::new();
    for i in 0..8 {
        trace.push([0.0, f64::from(i) * 0.01]);
    }
    trace.push([50.0, 0.0]);
    trace.push([60.0, 0.0]);
    for i in 0..8 {
        trace.push([100.0, f64::from(i) * 0.01]);
    }
    trace
}

#[test]
fn golden_euclidean_two_stops_labels_and_medians() {
    let mut model = Infostop::builder()
        .r1(1.0)
        .r2(5.0)
        .distance_metric(MetricKind::Euclidean)
        .min_size(2)
        .seed(42)
        .build()
        .unwrap();

    let trace = euclidean_two_stops();
    let labels = model.fit_predict(&trace).unwrap();

    // Locked regression labels for this fixture + seed.
    let expected = vec![0, 0, 0, 0, 0, 0, 0, 0, -1, -1, 1, 1, 1, 1, 1, 1, 1, 1];
    assert_eq!(labels, expected);

    let medians = model.label_medians().unwrap();
    assert_eq!(medians.len(), 2);
    assert!(medians.contains_key(&0));
    assert!(medians.contains_key(&1));
    let m0 = medians[&0];
    let m1 = medians[&1];
    assert!((m0.x - 0.0).abs() < 1e-9);
    assert!((m1.x - 100.0).abs() < 1e-9);
}

#[test]
fn golden_euclidean_two_stops_with_timestamps() {
    let mut model = Infostop::builder()
        .r1(1.0)
        .r2(5.0)
        .min_staying_time(100.0)
        .max_time_between(10_000.0)
        .distance_metric(MetricKind::Euclidean)
        .min_size(2)
        .seed(42)
        .build()
        .unwrap();

    let mut trace = Vec::new();
    for i in 0..8 {
        trace.push([0.0, f64::from(i) * 0.01, f64::from(i) * 60.0]);
    }
    // Trip (short duration between far points — not a stay)
    trace.push([50.0, 0.0, 8.0 * 60.0]);
    trace.push([60.0, 0.0, 8.0 * 60.0 + 30.0]);
    for i in 0..8 {
        let t0 = 8.0 * 60.0 + 60.0;
        trace.push([100.0, f64::from(i) * 0.01, t0 + f64::from(i) * 60.0]);
    }

    let labels = model.fit_predict(&trace).unwrap();
    assert_eq!(labels.len(), trace.len());
    assert!(labels[..8].iter().all(|&l| l >= 0));
    assert_eq!(labels[8], NON_STOP);
    assert_eq!(labels[9], NON_STOP);
    assert!(labels[10..].iter().all(|&l| l >= 0));
    assert_ne!(labels[0], labels[labels.len() - 1]);
    assert_eq!(model.label_medians().unwrap().len(), 2);
}

#[test]
fn golden_haversine_copenhagen_style() {
    let home = (55.6761, 12.5683);
    let work = (55.6867, 12.5700);

    let mut trace = Vec::new();
    for i in 0..12 {
        let jitter = f64::from(i) * 0.00001;
        trace.push([home.0 + jitter, home.1, f64::from(i) * 60.0]);
    }
    for i in 0..5 {
        let t = 12.0 * 60.0 + f64::from(i) * 60.0;
        let f = (f64::from(i) + 1.0) / 6.0;
        trace.push([
            home.0 + (work.0 - home.0) * f,
            home.1 + (work.1 - home.1) * f,
            t,
        ]);
    }
    for i in 0..12 {
        let jitter = f64::from(i) * 0.00001;
        let t = 20.0 * 60.0 + f64::from(i) * 60.0;
        trace.push([work.0 + jitter, work.1, t]);
    }

    let mut model = Infostop::builder()
        .r1(40.0)
        .r2(40.0)
        .min_staying_time(300.0)
        .min_size(2)
        .seed(42)
        .build()
        .unwrap();

    let labels = model.fit_predict(&trace).unwrap();
    assert_eq!(labels.len(), trace.len());

    let home_lab = labels[0];
    let work_lab = labels[labels.len() - 1];
    assert!(home_lab >= 0);
    assert!(work_lab >= 0);
    assert_ne!(home_lab, work_lab);
    assert!(labels[12..17].iter().all(|&l| l == NON_STOP));

    let medians = model.label_medians().unwrap();
    assert_eq!(medians.len(), 2);
    let mh = medians[&home_lab];
    let mw = medians[&work_lab];
    assert!((mh.x - home.0).abs() < 5e-4);
    assert!((mh.y - home.1).abs() < 5e-4);
    assert!((mw.x - work.0).abs() < 5e-4);
    assert!((mw.y - work.1).abs() < 5e-4);
}
