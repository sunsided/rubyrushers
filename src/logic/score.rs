//! Points for clearing gems.

use super::power::Power;

/// Points per cleared gem.
pub const PER_GEM: u64 = 10;

/// Bonus for a match that earns `reward`.
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

/// Points for one cascade step: `cleared` gems, the rewards earned and the specials set
/// off, all multiplied by the cascade depth (1 for the player's own match).
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
}
