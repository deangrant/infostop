---
name: infostop-pipeline
description: >-
  Infostop stop-detection pipeline, modules, labels, errors, and tests. Use when
  changing stay detection, neighbors, clustering, Infomap, the Infostop model
  API, algorithm parameters, golden tests, or plot_map behavior.
---

# Infostop pipeline

Use this skill when editing algorithm or model code in this crate.

## Pipeline

1. **Stays** — sequential stay detection per trajectory (`r1`, `min_staying_time`, `max_time_between`, `min_size`). Keep the median of each stay.
2. **Unique medians** — dedupe stay coordinates (optional `min_spatial_resolution` grid).
3. **Neighbor graph** — connect medians within `r2` (`NeighborQuery`; default brute force). Optional inverse-distance weights.
4. **Communities** — simplified two-level Infomap (`CommunityDetector` / `InfomapDetector`).
5. **Labels** — remap communities onto input points. `>= 0` = stop id; `-1` (`NON_STOP`) = non-stop. Singletons follow `label_singleton`.

## Module map

| Module | Role |
| ------ | ---- |
| `stay` / `distance` | Stays; Haversine / Euclidean |
| `neighbors` | Radius neighbor query |
| `cluster` / `infomap` | Graph edges + Infomap |
| `model` | `Infostop` orchestration |
| `config` | Hyperparameters + validation |
| `plot` | Leaflet HTML (`plot` feature) |

## Contracts

- Validate config and traces; return `Error::InvalidInput` on bad input.
- `Error::NoStopsFound` when fitting yields no labeled stops (check `r1`, times, `min_size`, `label_singleton`).
- `Error::NotFitted` for methods that need a fitted model.
- `plot_map` requires Haversine / geographic coordinates.

## Tests

- Prefer unit tests next to the module under test.
- Integration / golden: `tests/golden.rs`.
- `tests/perf_smoke.rs` is `#[ignore]` — do not treat it as default CI.

## Non-goals

Do not “fix” partitions to match external Infomap without an explicit design change. See `reference.md` for parameter defaults and error detail.
