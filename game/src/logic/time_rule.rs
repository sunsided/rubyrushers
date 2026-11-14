//! How much time the time gems add, so that even perfect play ends.

use super::mode::TIME_GEM_SECS;

/// Most seconds time gems add in one game under [`TimeRule::Cap`].
pub const CAP_SECS: f32 = 60.0;
/// What each further time gem gives less under [`TimeRule::Fade`].
pub const FADE_STEP: f32 = 0.5;
/// The least a time gem gives under [`TimeRule::Fade`].
pub const FADE_MIN: f32 = 1.0;

/// The rule for time gems in the timed modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeRule {
    /// +5 s each until they added [`CAP_SECS`] in total, then nothing.
    #[default]
    Cap,
    /// +5 s for the first, then [`FADE_STEP`] less for each one after, down to
    /// [`FADE_MIN`].
    Fade,
}

impl TimeRule {
    /// Seconds the next time gem adds, after `taken` time gems that added `added` seconds.
    #[must_use]
    pub fn next(self, taken: u32, added: f32) -> f32 {
        match self {
            Self::Cap => TIME_GEM_SECS.min(CAP_SECS - added).max(0.0),
            Self::Fade => (TIME_GEM_SECS - FADE_STEP * taken as f32).max(FADE_MIN),
        }
    }

    /// The name used on the command line and in the URL.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cap => "cap",
            Self::Fade => "fade",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seconds the first `n` time gems add one by one.
    fn first(rule: TimeRule, n: u32) -> Vec<f32> {
        let mut added = 0.0;
        (0..n)
            .map(|taken| {
                let s = rule.next(taken, added);
                added += s;
                s
            })
            .collect()
    }

    #[test]
    fn cap_stops_at_the_cap() {
        let secs = first(TimeRule::Cap, 15);
        assert_eq!(secs.iter().sum::<f32>(), CAP_SECS);
        assert_eq!(secs[11], 5.0);
        assert_eq!(secs[12], 0.0);
    }

    #[test]
    fn fade_shrinks_to_the_minimum() {
        let secs = first(TimeRule::Fade, 12);
        assert_eq!(&secs[..4], &[5.0, 4.5, 4.0, 3.5]);
        assert_eq!(secs[8], FADE_MIN);
        assert_eq!(secs[11], FADE_MIN);
    }
}
