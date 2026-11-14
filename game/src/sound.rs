//! Sound effects, synthesised once at start-up (no audio files).

use game_toolkit::prelude::*;
use synth::Preset;

/// Notes of the match sound per cascade depth: a rising pentatonic run.
const CASCADE_NOTES: [u8; 9] = [72, 74, 76, 79, 81, 84, 86, 88, 91];

/// The rendered effects.
pub struct Sound {
    /// Sample rate of every buffer.
    rate: u32,
    /// A match, per cascade depth.
    matches: Vec<Vec<f32>>,
    /// A swap.
    swap: Vec<f32>,
    /// A swap that lines nothing up.
    reject: Vec<f32>,
    /// A special gem going off.
    blast: Vec<f32>,
    /// Gems landing.
    land: Vec<f32>,
    /// A time gem cleared.
    time: Vec<f32>,
    /// The game ended.
    over: Vec<f32>,
    /// A button or start.
    click: Vec<f32>,
}

/// Which effect to play.
#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    /// A match at this cascade depth (1 = the player's own).
    Match(u32),
    /// A swap.
    Swap,
    /// A swap that lines nothing up.
    Reject,
    /// A special gem going off.
    Blast,
    /// Gems landing.
    Land,
    /// A time gem cleared.
    Time,
    /// The game ended.
    Over,
    /// A button or start.
    Click,
}

impl Sound {
    /// Renders every effect.
    #[must_use]
    pub fn new() -> Self {
        let mut s = Synth::new(44_100);
        let matches = CASCADE_NOTES
            .iter()
            .map(|&n| {
                let mut v = s.render(Preset::MusicBox.params(), n, 0.06, 0.25);
                // A fifth on top for a brighter chime.
                let top = s.render(Preset::MusicBox.params(), n + 7, 0.04, 0.2);

                for (a, b) in v.iter_mut().zip(top) {
                    *a = 0.7 * *a + 0.5 * b;
                }

                v
            })
            .collect();
        Self {
            rate: s.sample_rate(),
            matches,
            swap: s.render(Preset::PluckyPulse.params(), 67, 0.02, 0.05),
            reject: s.render(Preset::SlideBass.params(), 45, 0.08, 0.1),
            blast: s.thud(),
            land: s.render(Preset::TriBass.params(), 40, 0.02, 0.06),
            time: s.coin(),
            over: s.render_phrase(
                &[
                    (0.0, synth::AudioEvent::NoteOn(synth::MidiNote(72))),
                    (0.15, synth::AudioEvent::NoteOff(synth::MidiNote(72))),
                    (0.15, synth::AudioEvent::NoteOn(synth::MidiNote(67))),
                    (0.3, synth::AudioEvent::NoteOff(synth::MidiNote(67))),
                    (0.3, synth::AudioEvent::NoteOn(synth::MidiNote(64))),
                    (0.7, synth::AudioEvent::NoteOff(synth::MidiNote(64))),
                ],
                Preset::TriPad.params(),
                0.5,
            ),
            click: s.blip(),
        }
    }

    /// Plays `sfx` if audio is running.
    pub fn play(&self, audio: Option<&mut Audio>, sfx: Sfx) {
        let Some(audio) = audio else {
            return;
        };

        let (buf, volume) = match sfx {
            Sfx::Match(depth) => {
                let i = (depth.max(1) as usize - 1).min(self.matches.len() - 1);
                (&self.matches[i], 0.5)
            }
            Sfx::Swap => (&self.swap, 0.25),
            Sfx::Reject => (&self.reject, 0.4),
            Sfx::Blast => (&self.blast, 0.7),
            Sfx::Land => (&self.land, 0.2),
            Sfx::Time => (&self.time, 0.4),
            Sfx::Over => (&self.over, 0.5),
            Sfx::Click => (&self.click, 0.3),
        };

        // Before the first gesture on the web, or without a device, this just fails.
        let _ = audio.play_samples_with(buf, self.rate, PlayOptions::default().volume(volume));
    }
}

impl Default for Sound {
    fn default() -> Self {
        Self::new()
    }
}
