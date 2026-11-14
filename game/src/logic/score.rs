//! Points for clearing gems.

use super::power::Power;

/// Points per cleared gem.
pub const PER_GEM: u64 = 10;

/// Bonus for a match that earns `reward`.
#[must_use]
pub const fn reward_bonus(reward: Power) -> u64 {
    match reward {
        Power::None => 0,
        Power::Bomb => 50,
        Power::Cross => 100,
        Power::Wild => 250,
    }
}

/// Bonus per special gem that goes off.
pub const BLAST_BONUS: u64 = 30;

/// Seconds after a match of the player's within which the next one raises the speed
/// streak.
pub const SPEED_WINDOW: f32 = 2.5;
/// The highest speed streak.
pub const MAX_SPEED: u32 = 6;

/// `points` with the speed streak's bonus: +50% per level.
#[must_use]
pub fn with_speed(points: u64, speed: u32) -> u64 {
    points * u64::from(2 + speed) / 2
}

/// The speed streak's multiplier for display, e.g. "1.5".
#[must_use]
pub fn speed_factor(speed: u32) -> f32 {
    (2 + speed) as f32 / 2.0
}

/// Points for one cascade step: `cleared` gems, the rewards earned and the specials set
/// off, all multiplied by the cascade depth (1 for the player's own match).
#[must_use]
pub fn points(cleared: usize, rewards: &[Power], blasts: usize, cascade: u32) -> u64 {
    let base = cleared as u64 * PER_GEM
        + rewards.iter().map(|&r| reward_bonus(r)).sum::<u64>()
        + blasts as u64 * BLAST_BONUS;
    base * u64::from(cascade)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascades_multiply() {
        assert_eq!(points(3, &[Power::None], 0, 1), 30);
        assert_eq!(points(3, &[Power::None], 0, 3), 90);
        assert_eq!(points(4, &[Power::Bomb], 1, 2), (40 + 50 + 30) * 2);
    }

    #[test]
    fn speed_adds_half_per_level() {
        assert_eq!(with_speed(100, 0), 100);
        assert_eq!(with_speed(100, 1), 150);
        assert_eq!(with_speed(100, MAX_SPEED), 400);
    }
}
