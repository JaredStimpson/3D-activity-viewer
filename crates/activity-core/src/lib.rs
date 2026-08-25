use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use thiserror::Error;

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPoint {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub cumulative_distance_meters: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityStats {
    pub distance_meters: f64,
    pub elevation_gain_meters: f64,
    pub elevation_loss_meters: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_elevation_meters: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_elevation_meters: Option<f64>,
    pub bounds: [f64; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub name: String,
    pub points: Vec<ActivityPoint>,
    pub stats: ActivityStats,
}

#[derive(Debug, Error)]
pub enum ActivityError {
    #[error("the GPX file is not valid XML: {0}")]
    InvalidXml(String),
    #[error("the GPX file needs at least two valid track points")]
    InsufficientPoints,
}

pub fn parse_gpx(source: &str) -> Result<Activity, ActivityError> {
    let document = roxmltree::Document::parse(source)
        .map_err(|error| ActivityError::InvalidXml(error.to_string()))?;
    let name = document
        .descendants()
        .find(|node| node.has_tag_name("name"))
        .and_then(|node| node.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Imported activity")
        .to_owned();

    let mut points = Vec::new();
    for node in document
        .descendants()
        .filter(|node| node.has_tag_name("trkpt"))
    {
        let Some(latitude) = node
            .attribute("lat")
            .and_then(|value| value.parse::<f64>().ok())
        else {
            continue;
        };
        let Some(longitude) = node
            .attribute("lon")
            .and_then(|value| value.parse::<f64>().ok())
        else {
            continue;
        };
        if !latitude.is_finite()
            || !longitude.is_finite()
            || latitude.abs() > 90.0
            || longitude.abs() > 180.0
        {
            continue;
        }

        let child_text = |tag: &str| {
            node.children()
                .find(|child| child.has_tag_name(tag))
                .and_then(|child| child.text())
                .map(str::trim)
        };
        let point = ActivityPoint {
            latitude,
            longitude,
            elevation: child_text("ele").and_then(|value| value.parse::<f64>().ok()),
            timestamp: child_text("time").map(str::to_owned),
            cumulative_distance_meters: 0.0,
        };

        let duplicate = points.last().is_some_and(|previous: &ActivityPoint| {
            (previous.latitude - point.latitude).abs() < f64::EPSILON
                && (previous.longitude - point.longitude).abs() < f64::EPSILON
        });
        if !duplicate {
            points.push(point);
        }
    }

    if points.len() < 2 {
        return Err(ActivityError::InsufficientPoints);
    }

    let mut distance = 0.0;
    let mut elevation_gain = 0.0;
    let mut elevation_loss = 0.0;
    for index in 1..points.len() {
        distance += haversine_distance(&points[index - 1], &points[index]);
        points[index].cumulative_distance_meters = distance;
        if let (Some(previous), Some(current)) =
            (points[index - 1].elevation, points[index].elevation)
        {
            let delta = current - previous;
            if delta > 0.0 {
                elevation_gain += delta;
            } else {
                elevation_loss -= delta;
            }
        }
    }

    let elevations: Vec<_> = points.iter().filter_map(|point| point.elevation).collect();
    let min_elevation = elevations.iter().copied().reduce(f64::min);
    let max_elevation = elevations.iter().copied().reduce(f64::max);
    let min_lon = points
        .iter()
        .map(|point| point.longitude)
        .fold(f64::INFINITY, f64::min);
    let min_lat = points
        .iter()
        .map(|point| point.latitude)
        .fold(f64::INFINITY, f64::min);
    let max_lon = points
        .iter()
        .map(|point| point.longitude)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_lat = points
        .iter()
        .map(|point| point.latitude)
        .fold(f64::NEG_INFINITY, f64::max);
    let duration_seconds = points
        .first()
        .and_then(|point| point.timestamp.as_deref())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .zip(
            points
                .last()
                .and_then(|point| point.timestamp.as_deref())
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok()),
        )
        .map(|(start, finish)| (finish - start).num_milliseconds().max(0) as f64 / 1_000.0);

    Ok(Activity {
        name,
        points,
        stats: ActivityStats {
            distance_meters: distance,
            elevation_gain_meters: elevation_gain,
            elevation_loss_meters: elevation_loss,
            duration_seconds,
            min_elevation_meters: min_elevation,
            max_elevation_meters: max_elevation,
            bounds: [min_lon, min_lat, max_lon, max_lat],
        },
    })
}

pub fn haversine_distance(a: &ActivityPoint, b: &ActivityPoint) -> f64 {
    let radians = |degrees: f64| degrees * PI / 180.0;
    let delta_lat = radians(b.latitude - a.latitude);
    let delta_lon = radians(b.longitude - a.longitude);
    let lat_a = radians(a.latitude);
    let lat_b = radians(b.latitude);
    let h = (delta_lat / 2.0).sin().powi(2)
        + lat_a.cos() * lat_b.cos() * (delta_lon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_METERS * h.sqrt().asin()
}

pub fn point_at_progress(activity: &Activity, progress: f64) -> ActivityPoint {
    let target = activity.stats.distance_meters * progress.clamp(0.0, 1.0);
    let index = activity
        .points
        .iter()
        .position(|point| point.cumulative_distance_meters >= target)
        .unwrap_or(activity.points.len() - 1);
    if index == 0 {
        return activity.points[0].clone();
    }
    let before = &activity.points[index - 1];
    let after = &activity.points[index];
    let span =
        (after.cumulative_distance_meters - before.cumulative_distance_meters).max(f64::EPSILON);
    let t = (target - before.cumulative_distance_meters) / span;
    ActivityPoint {
        latitude: before.latitude + (after.latitude - before.latitude) * t,
        longitude: before.longitude + (after.longitude - before.longitude) * t,
        elevation: match (before.elevation, after.elevation) {
            (Some(a), Some(b)) => Some(a + (b - a) * t),
            (_, value) => value,
        },
        timestamp: after.timestamp.clone(),
        cumulative_distance_meters: target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GPX: &str = r#"<?xml version="1.0"?><gpx><trk><name>Test Ridge</name><trkseg>
      <trkpt lat="36.0000" lon="-121.0000"><ele>10</ele><time>2026-01-01T10:00:00Z</time></trkpt>
      <trkpt lat="36.0100" lon="-121.0000"><ele>35</ele><time>2026-01-01T10:10:00Z</time></trkpt>
      <trkpt lat="36.0200" lon="-120.9950"><ele>20</ele><time>2026-01-01T10:20:00Z</time></trkpt>
    </trkseg></trk></gpx>"#;

    #[test]
    fn parses_track_and_statistics() {
        let activity = parse_gpx(GPX).expect("valid activity");
        assert_eq!(activity.name, "Test Ridge");
        assert_eq!(activity.points.len(), 3);
        assert!(activity.stats.distance_meters > 2_200.0);
        assert_eq!(activity.stats.elevation_gain_meters, 25.0);
        assert_eq!(activity.stats.elevation_loss_meters, 15.0);
        assert_eq!(activity.stats.duration_seconds, Some(1_200.0));
        assert_eq!(activity.stats.bounds, [-121.0, 36.0, -120.995, 36.02]);
    }

    #[test]
    fn evaluates_progress_by_physical_distance() {
        let activity = parse_gpx(GPX).expect("valid activity");
        let point = point_at_progress(&activity, 0.5);
        assert!(
            (point.cumulative_distance_meters - activity.stats.distance_meters * 0.5).abs() < 0.01
        );
    }

    #[test]
    fn rejects_empty_tracks() {
        assert!(matches!(
            parse_gpx("<gpx />"),
            Err(ActivityError::InsufficientPoints)
        ));
    }
}
