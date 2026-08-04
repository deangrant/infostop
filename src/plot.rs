use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::cluster::CommunityDetector;
use crate::distance::MetricKind;
use crate::error::{Error, Result};
use crate::model::Infostop;
use crate::neighbors::NeighborQuery;
use crate::types::NON_STOP;

/// Pinned CDN head with Subresource Integrity for Leaflet 1.9.4 and leaflet.heat 0.2.0.
///
/// Hashes are sha384 of the exact unpkg bytes for those versions (verified 2026-08-04).
const CDN_HEAD: &str = r#"  <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" integrity="sha384-sHL9NAb7lN7rfvG5lfHpm643Xkcjzp4jFvuavGOndn6pjVqS6ny56CAt3nsEVT4H" crossorigin="anonymous"/>
  <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js" integrity="sha384-cxOPjt7s7Iz04uaHJceBmS+qpjv2JkIHNVcuOrM+YHwZOmJGBXI00mdUXEq65HTH" crossorigin="anonymous"></script>
  <script src="https://unpkg.com/leaflet.heat@0.2.0/dist/leaflet-heat.js" integrity="sha384-mFKkGiGvT5vo1fEyGCD3hshDdKmW3wzXW/x+fWriYJArD0R3gawT6lMvLboM22c0" crossorigin="anonymous"></script>"#;

/// Write a Leaflet HTML map visualizing stop locations from a fitted model.
///
/// Requires `distance_metric == Haversine` (geographic coordinates).
///
/// Open the resulting file in a browser to inspect stops. Leaflet JS/CSS are
/// loaded from a pinned CDN URL with Subresource Integrity. Basemap raster
/// tiles still come from OpenStreetMap and are not integrity-checked.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if the model is not using Haversine,
/// [`Error::NotFitted`] if the model has not been fitted,
/// [`Error::NoStopsFound`] if there are no stationary points,
/// or [`Error::Io`] if writing `path` fails.
pub fn plot_map<P, D, N>(model: &Infostop<D, N>, path: P) -> Result<()>
where
    P: AsRef<Path>,
    D: CommunityDetector,
    N: NeighborQuery,
{
    if model.config().distance_metric != MetricKind::Haversine {
        return Err(Error::InvalidInput(
            "plot_map requires Haversine / geographic coordinates".into(),
        ));
    }

    let points = model.stationary_points()?;
    let labels = model.stationary_labels()?;
    let medians = model.label_medians()?;

    if points.is_empty() {
        return Err(Error::NoStopsFound);
    }

    let mut lat_sum = 0.0;
    let mut lon_sum = 0.0;
    for p in points {
        lat_sum += p.x;
        lon_sum += p.y;
    }
    #[allow(clippy::cast_precision_loss)]
    // map center; point counts stay modest
    let center_lat = lat_sum / points.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let center_lon = lon_sum / points.len() as f64;

    let mut markers = String::new();
    for (p, &lab) in points.iter().zip(labels.iter()) {
        if lab == NON_STOP {
            continue;
        }
        let color = color_for_label(lab);
        let _ = writeln!(
            markers,
            "L.circleMarker([{}, {}], {{radius: 5, color: '{color}', fillColor: '{color}', fillOpacity: 0.7}}).addTo(map).bindPopup('stop {lab}');",
            p.x,
            p.y,
        );
    }

    let mut median_markers = String::new();
    for (&lab, p) in &medians {
        let color = color_for_label(lab);
        let _ = writeln!(
            median_markers,
            "L.marker([{}, {}]).addTo(map).bindPopup('median stop {lab}');\nL.circle([{}, {}], {{radius: 25, color: '{color}', fillColor: '{color}', fillOpacity: 0.15}}).addTo(map);",
            p.x,
            p.y,
            p.x,
            p.y,
        );
    }

    let heat_pts: Vec<String> = points
        .iter()
        .zip(labels.iter())
        .filter(|(_, &lab)| lab != NON_STOP)
        .map(|(p, _)| format!("[{}, {}, 0.6]", p.x, p.y))
        .collect();
    let heat_js = heat_pts.join(",\n");

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>Infostop map</title>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
{CDN_HEAD}
  <style>html, body, #map {{ height: 100%; margin: 0; }}</style>
</head>
<body>
<div id="map"></div>
<script>
const map = L.map('map').setView([{center_lat}, {center_lon}], 14);
L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
  maxZoom: 19,
  attribution: '&copy; OpenStreetMap'
}}).addTo(map);
{median_markers}
{markers}
const heat = L.heatLayer([
{heat_js}
], {{radius: 25, blur: 15, maxZoom: 17}});
heat.addTo(map);
</script>
</body>
</html>
"#
    );

    fs::write(path.as_ref(), html)?;
    Ok(())
}

fn color_for_label(label: i32) -> &'static str {
    const COLORS: &[&str] = &[
        "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#a65628",
        "#f781bf", "#999999", "#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3",
    ];
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )] // palette index from rem_euclid into a small const table
    let idx = label.rem_euclid(COLORS.len() as i32) as usize;
    COLORS[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::MetricKind;

    #[test]
    fn cdn_head_includes_sri() {
        assert!(CDN_HEAD.contains("integrity=\"sha384-"));
        assert!(CDN_HEAD.contains("crossorigin=\"anonymous\""));
        assert_eq!(CDN_HEAD.matches("integrity=").count(), 3);
        assert_eq!(CDN_HEAD.matches("crossorigin=").count(), 3);
    }

    #[cfg(feature = "plot")]
    #[test]
    fn plot_rejects_euclidean_model() {
        let mut model = Infostop::builder()
            .r1(1.0)
            .r2(5.0)
            .distance_metric(MetricKind::Euclidean)
            .min_size(2)
            .build()
            .unwrap();
        let trace = [
            [0.0, 0.0],
            [0.1, 0.0],
            [0.2, 0.0],
            [100.0, 0.0],
            [100.1, 0.0],
            [100.2, 0.0],
        ];
        model.fit_predict(&trace).unwrap();
        let err =
            plot_map(&model, std::env::temp_dir().join("infostop_eucl.html"));
        assert!(matches!(err, Err(Error::InvalidInput(_))));
    }

    #[cfg(feature = "plot")]
    #[test]
    fn plot_requires_fitted_model() {
        let model = Infostop::new();
        let err =
            plot_map(&model, std::env::temp_dir().join("infostop_nofit.html"));
        assert_eq!(err, Err(Error::NotFitted));
    }

    #[cfg(feature = "plot")]
    #[test]
    fn plot_writes_html_with_leaflet_and_coords() {
        let mut model = Infostop::builder()
            .r1(100.0)
            .r2(100.0)
            .min_staying_time(50.0)
            .min_size(2)
            .build()
            .unwrap();
        let trace = [
            [55.6761, 12.5683, 0.0],
            [55.6762, 12.5683, 60.0],
            [55.6761, 12.5684, 120.0],
        ];
        model.fit_predict(&trace).unwrap();

        let path = std::env::temp_dir().join("infostop_plot_fixture.html");
        plot_map(&model, &path).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("leaflet"));
        assert!(html.contains("L.map"));
        assert!(html.contains("55.676"));
        let _ = fs::remove_file(&path);
    }
}
