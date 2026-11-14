//! Easing curves over `t` in `0.0..=1.0`.

/// Slow start and end.
#[must_use]
pub fn in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);

    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Fast start, slow end.
#[must_use]
pub fn out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Overshoots a little, then settles.
#[must_use]
pub fn out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c = 1.70158;
    1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_start_at_zero_and_end_at_one() {
        for f in [in_out_cubic, out_cubic, out_back] {
            assert!(f(0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
    }
}
