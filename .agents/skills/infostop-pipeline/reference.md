# Infostop pipeline reference

## Default config (`Config::default`)

| Field | Default |
| ----- | ------- |
| `r1` / `r2` | `10.0` |
| `label_singleton` | `true` |
| `min_staying_time` | `300.0` |
| `max_time_between` | `86400.0` |
| `min_size` | `2` |
| `min_spatial_resolution` | `0.0` (off) |
| `distance_metric` | `Haversine` |
| `weighted` | `false` |
| `weight_exponent` | `1.0` |
| `seed` | `42` |

`Config::validate` requires finite positive `r1`/`r2`/times, `max_time_between > min_staying_time`, and consistent `min_size` / resolution / weight fields.

## Public API

```rust
let mut model = Infostop::builder().r1(30.0).r2(30.0).build()?;
let labels = model.fit_predict(&trace)?;
let labels = model.fit_predict_many(&[t1, t2])?;
let medians = model.label_medians()?;
```

Optional: `plot_map(&model, "stops.html")` with `--features plot`.

## Errors (`src/error.rs`)

| Variant | When |
| ------- | ---- |
| `InvalidInput` | Bad config or traces |
| `NoStopsFound` | No stationary / labeled stops for parameters |
| `NotFitted` | Method called before fit |
| `Io` | e.g. writing the map file |

## Infomap caveat

In-crate detector is greedy two-level map-equation search (no multilevel aggregation / teleportation). Partitions need not match upstream Infomap. Swap via `CommunityDetector` only when intentionally changing clustering.

## Traits (extension points)

- `DistanceMetric` — spatial distance
- `NeighborQuery` — radius search over medians
- `CommunityDetector` — graph → community labels
