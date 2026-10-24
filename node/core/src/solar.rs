//! Sun position (NOAA approximation) and a clear-sky output model.

use std::f64::consts::PI;

/// Solar elevation in degrees for a unix timestamp at a latitude/longitude.
pub fn elevation(ts: i64, lat: f64, lon: f64) -> f64 {
    let days = ts as f64 / 86_400.0;
    let doy = (days % 365.25).floor();
    let hour_utc = (ts.rem_euclid(86_400)) as f64 / 3600.0;
    let g = 2.0 * PI / 365.0 * (doy + (hour_utc - 12.0) / 24.0);
    let decl = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2.0 * g).cos()
        + 0.000907 * (2.0 * g).sin()
        - 0.002697 * (3.0 * g).cos()
        + 0.00148 * (3.0 * g).sin();
    let eqtime = 229.18
        * (0.000075 + 0.001868 * g.cos() - 0.032077 * g.sin() - 0.014615 * (2.0 * g).cos()
            - 0.040849 * (2.0 * g).sin());
    let tst = hour_utc * 60.0 + eqtime + 4.0 * lon;
    let ha = (tst / 4.0 - 180.0).to_radians();
    let lat = lat.to_radians();
    let cos_zen = lat.sin() * decl.sin() + lat.cos() * decl.cos() * ha.cos();
    90.0 - cos_zen.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Expected clear-sky output fraction of nameplate (0..1) at a sun elevation.
pub fn clear_sky_fraction(elev_deg: f64) -> f64 {
    if elev_deg <= 0.0 {
        return 0.0;
    }
    let s = elev_deg.to_radians().sin();
    (s.powf(1.15) * 0.92).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-06-21 12:00 UTC
    const SOLSTICE_NOON: i64 = 1_782_043_200;

    #[test]
    fn lagos_is_dark_at_2am_and_bright_at_noon() {
        let (lat, lon) = (6.5, 3.4);
        assert!(elevation(SOLSTICE_NOON - 11 * 3600, lat, lon) < -30.0);
        assert!(elevation(SOLSTICE_NOON, lat, lon) > 60.0);
    }
}
