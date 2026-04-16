use std::time::{Duration, Instant};

/// Decide whether a `Pressed` transition should fire the upload and what the
/// new `last_press` state should be.
///
/// * `prev` — timestamp of the previous `Pressed` transition, if any.
/// * `now` — timestamp of the current `Pressed` transition.
/// * `window` — max gap between two presses that counts as a double-tap.
///
/// Returns `(fire, next_state)`. If `fire` is true, a double-tap was detected
/// and the caller should trigger the upload AND reset state to `None`. If
/// `fire` is false, the caller stores `next_state` as the new `last_press`.
pub fn should_fire(
    prev: Option<Instant>,
    now: Instant,
    window: Duration,
) -> (bool, Option<Instant>) {
    match prev {
        Some(p) if now.duration_since(p) <= window => (true, None),
        _ => (false, Some(now)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_press_only_stores_timestamp() {
        let now = Instant::now();
        let (fire, next) = should_fire(None, now, Duration::from_millis(400));
        assert!(!fire);
        assert_eq!(next, Some(now));
    }

    #[test]
    fn second_press_inside_window_fires_and_clears() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(399);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(fire);
        assert_eq!(next, None);
    }

    #[test]
    fn second_press_at_exact_window_boundary_fires() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(400);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(fire);
        assert_eq!(next, None);
    }

    #[test]
    fn second_press_outside_window_rearms_timestamp() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(401);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(!fire);
        assert_eq!(next, Some(t1));
    }
}
