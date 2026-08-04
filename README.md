# Infostop

Infostop is a Rust crate. Use Infostop to find **stops** in location **trajectories**.

A trajectory is a sequence of location **points**. Each point has a position. A point can also have a time value.

## Technical overview

Infostop works in two steps.

1. Find **stays** in each trajectory. A stay is a group of points that stay near the same place. The distance limit for a stay is `r1`. Infostop keeps the median position of each stay.
2. Connect stay medians that are near each other. The distance limit for a connection is `r2`. Infostop then clusters connected stays into **stops** (simplified two-level Infomap; not identical to upstream Infomap).

Each input point gets one **label**:

- A label `>= 0` is a stop id.
- The label `-1` means the point is not part of a stop (for example, movement).

## Requirements

- Use Rust and Cargo.
- This crate has no required third-party dependencies.
- Enable the optional `plot` feature to write a map file.

## Install

Add the crate to your project.

```toml
[dependencies]
infostop = "0.1"
```

Add the crate with map output.

```toml
[dependencies]
infostop = { version = "0.1", features = ["plot"] }
```

## Usage

### One trajectory

1. Create a model.
2. Set the parameters that you need.
3. Call `fit_predict` with one trajectory.
4. Call `label_medians` to get the stop positions.

Use `[x, y]` points with `MetricKind::Euclidean`. Use `[latitude, longitude]` points with the default `Haversine` metric.

```rust
use infostop::{Infostop, MetricKind};

fn main() -> infostop::Result<()> {
    let mut model = Infostop::builder()
        .r1(30.0)
        .r2(30.0)
        .distance_metric(MetricKind::Euclidean)
        .build()?;

    let trace = [
        [0.0, 0.0],
        [0.1, 0.0],
        [0.2, 0.0],
        [50.0, 0.0],
        [100.0, 0.0],
        [100.1, 0.0],
        [100.2, 0.0],
    ];

    let labels = model.fit_predict(&trace)?;
    let medians = model.label_medians()?;
    println!("{labels:?}");
    println!("{medians:?}");
    Ok(())
}
```

### Trajectory with time

Give each point as `[x, y, time]` or `[latitude, longitude, time]`.

Use the same time unit for the data and for the time parameters. The default time unit is seconds.

```rust
use infostop::Infostop;

fn main() -> infostop::Result<()> {
    let mut model = Infostop::new();
    let trace = [
        [55.6761, 12.5683, 0.0],
        [55.6762, 12.5683, 60.0],
        [55.6761, 12.5684, 600.0],
    ];
    let labels = model.fit_predict(&trace)?;
    println!("{labels:?}");
    Ok(())
}
```

### Several trajectories

Call `fit_predict_many` when you have more than one trajectory. Infostop finds stops that the trajectories can share.

```rust
use infostop::{Infostop, MetricKind};

fn main() -> infostop::Result<()> {
    let mut model = Infostop::builder()
        .distance_metric(MetricKind::Euclidean)
        .build()?;

    let t1 = [[0.0, 0.0], [0.1, 0.0], [0.2, 0.0]];
    let t2 = [[0.0, 0.05], [0.05, 0.0], [0.1, 0.05]];
    let labels = model.fit_predict_many(&[t1.as_slice(), t2.as_slice()])?;
    println!("{labels:?}");
    Ok(())
}
```

### Map output

1. Enable the `plot` feature.
2. Fit a model with `Haversine` latitude and longitude data.
3. Call `plot_map`.
4. Open the HTML file in a browser.

The generated HTML loads pinned Leaflet JS/CSS from a CDN with Subresource Integrity.
Basemap tiles still load from OpenStreetMap and are not integrity-checked.

```rust
use infostop::{plot_map, Infostop};

fn main() -> infostop::Result<()> {
    let mut model = Infostop::new();
    let trace = [
        [55.6761, 12.5683, 0.0],
        [55.6762, 12.5683, 60.0],
        [55.6761, 12.5684, 600.0],
    ];
    model.fit_predict(&trace)?;
    plot_map(&model, "stops.html")?;
    Ok(())
}
```

## Input and output

### Input

- One point with no time: `[x, y]` or `[latitude, longitude]`.
- One point with time: `[x, y, time]` or `[latitude, longitude, time]`.
- One trajectory: a list of points.
- Several trajectories: a list of trajectories for `fit_predict_many`.
- Within one trajectory, every point must include a timestamp, or none of them may. Mixed timestamps are rejected.

### Output

- `fit_predict` returns one label for each point.
- `fit_predict_many` returns one label list for each trajectory.
- `label_medians` returns the median position for each stop id.
- Label `>= 0` means a stop.
- Label `-1` means not a stop.

## Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `r1` | `10` | Maximum distance from the stay median for a point to join that stay. |
| `r2` | `10` | Maximum distance between stay medians for a network connection. |
| `min_staying_time` | `300` | Minimum stay duration. Infostop ignores this value when the data has no time. |
| `max_time_between` | `86400` | Maximum time gap between two points in the same stay. |
| `min_size` | `2` | Minimum number of points in a stay. The value must be greater than `1`. |
| `label_singleton` | `true` | If `true`, give a label to an isolated stay. If `false`, use `-1`. |
| `min_spatial_resolution` | `0` | Grid step in the same units as coordinates, applied before exact unique filtering of stay positions. Use `0` to disable (bit-identical coordinates only). |
| `distance_metric` | `Haversine` | Use `Haversine` for geographic data. Use `Euclidean` for planar data. |
| `weighted` | `false` | If `true`, use inverse distance as edge weight. |
| `seed` | `42` | Random seed for clustering. |

If the trajectory has no time values, only `r1` and `min_size` control stay detection.

## Examples

Run the basic example.

```bash
cargo run --example basic
```

Run the map example.

```bash
cargo run --example plot_stops --features plot
```

Run the tests.

```bash
cargo test
```

## License

This project uses the MIT license. See [LICENSE](LICENSE).
