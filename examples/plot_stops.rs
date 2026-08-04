//! Fit Infostop on a synthetic geographic trajectory and write a Leaflet map.

use infostop::{plot_map, Infostop};

fn main() -> infostop::Result<()> {
    // Two stops in Copenhagen (approx), with a trip between them.
    let home = (55.6761, 12.5683);
    let work = (55.6867, 12.5700);

    let mut trace = Vec::new();
    for i in 0..12 {
        let jitter = (i as f64) * 0.00001;
        trace.push([home.0 + jitter, home.1, i as f64 * 60.0]);
    }
    // Trip samples
    for i in 0..5 {
        let t = 12.0 * 60.0 + i as f64 * 60.0;
        let f = (i as f64 + 1.0) / 6.0;
        trace.push([
            home.0 + (work.0 - home.0) * f,
            home.1 + (work.1 - home.1) * f,
            t,
        ]);
    }
    for i in 0..12 {
        let jitter = (i as f64) * 0.00001;
        let t = 20.0 * 60.0 + i as f64 * 60.0;
        trace.push([work.0 + jitter, work.1, t]);
    }

    let mut model = Infostop::builder()
        .r1(40.0)
        .r2(40.0)
        .min_staying_time(300.0)
        .min_size(2)
        .build()?;

    let labels = model.fit_predict(&trace)?;
    let n_stops = labels.iter().filter(|&&l| l >= 0).count();
    println!("labeled {} / {} points as stops", n_stops, labels.len());

    let out = "stops.html";
    plot_map(&model, out)?;
    println!("wrote {out} — open it in a browser");
    Ok(())
}
