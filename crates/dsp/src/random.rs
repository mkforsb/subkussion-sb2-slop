//! RND and MUTATE: random patches shaped for playable bass percussion, and
//! the parameter locks that keep controls out of them.
//!
//! This lives in the DSP crate (it has no UI dependencies) so the analysis
//! examples can roll and render random hits exactly as the panel does.

use crate::params::{FILTER_BP, FILTER_HP, ParamKind};
use crate::util::Noise;
use crate::{ALL_PARAMS, PARAM_COUNT, Param, VoicePatch};

/// Which parameters of one channel are locked against RND and MUTATE.
pub type Locks = [bool; PARAM_COUNT];

pub const NO_LOCKS: Locks = [false; PARAM_COUNT];

/// Can this control be locked? Everything RND and MUTATE would change.
pub fn lockable(p: Param) -> bool {
    !p.is_performance()
}

/// A random patch, as produced by the RND button (and MUTATE at RND on every
/// hit). Performance controls (level, sense, pan) and `locks`ed controls keep
/// their values from `current`.
///
/// Every control draws exactly one number, in parameter order, whether or
/// not it is locked, so locking one control never changes what the others
/// roll. The draws are shaped for *bass*: pitches of 30–95 Hz, long decays,
/// a sub oscillator under most hits, lowpass filters that open on the hit,
/// a gentle low-shelf lift, and each engine's macros kept to the part of
/// their range that stays heavy (see [`macro_range`]).
pub fn random_patch(current: &VoicePatch, locks: &Locks, rng: &mut Noise) -> VoicePatch {
    let mut patch = *current;
    let mut draws = [0.0f32; PARAM_COUNT];
    for p in ALL_PARAMS {
        draws[p.index()] = rng.sample() * 0.5 + 0.5;
    }
    let bipolar = |x: f32, depth: f32| 0.5 + depth * (2.0 * x - 1.0).powi(3);
    // Often off, otherwise anywhere up to `max`.
    let sometimes = |x: f32, off: f32, max: f32| ((x - off) / (1.0 - off)).max(0.0) * max;
    let lerp = |lo: f32, hi: f32, x: f32| lo + (hi - lo) * x;
    let free = |p: Param| !p.is_performance() && !locks[p.index()];
    for p in ALL_PARAMS {
        if !free(p) {
            continue;
        }
        let x = draws[p.index()];
        patch[p.index()] = p.sanitize(match (p, p.info().kind) {
            // Lowpass almost always: it's a bass machine.
            (Param::FilterType, _) => match x {
                x if x < 0.4 => 0.0,
                x if x < 0.9 => 1.0,
                x if x < 0.96 => 2.0,
                _ => 3.0,
            },
            // Mostly an octave down, sometimes unison, now and then two.
            (Param::SubOctave, _) => match x {
                x if x < 0.35 => 0.0,
                x if x < 0.9 => 1.0,
                _ => 2.0,
            },
            (_, ParamKind::Choice(names)) => (x * names.len() as f32).floor(),
            // 30 Hz … 95 Hz, leaning low.
            (Param::Pitch, _) => 0.15 + 0.28 * x.powf(1.2),
            (Param::Fine, _) => lerp(0.3, 0.7, x),
            // 0.4 s … 3 s.
            (Param::Decay, _) => lerp(0.48, 0.87, x),
            (Param::Level1, _) => lerp(0.7, 1.0, x),
            (Param::Level2 | Param::Level3, _) => sometimes(x, 0.45, 0.85),
            // Free intervals within an octave either way.
            (Param::Tune2 | Param::Tune3, _) => crate::params::map::tune_knob(lerp(-12.0, 12.0, x)),
            (Param::Decay2 | Param::Decay3, _) => lerp(0.55, 0.92, x),
            // A sub under most hits.
            (Param::SubLevel, _) => {
                if x < 0.15 {
                    0.0
                } else {
                    lerp(0.5, 0.9, (x - 0.15) / 0.85)
                }
            }
            (Param::SubDecay, _) => lerp(0.65, 0.95, x),
            (Param::SubShape, _) => 0.5 * x * x,
            (Param::Attack, _) => 0.2 * x.powi(3),
            (Param::Hold, _) => 0.65 * x * x,
            (Param::Click, _) => 0.4 * x * x,
            (Param::Noise, _) => 0.3 * x.powi(3),
            (Param::SweepSpeed, _) => lerp(0.4, 0.85, x),
            // A downward drop of up to ~2 octaves, the kick's signature.
            (Param::SweepDepth, _) => lerp(0.5, 0.88, x),
            (Param::ChaosDepth, _) => 0.5 * x * x,
            (Param::Jitter, _) => 0.2 * x * x,
            // 400 Hz … 5 kHz, opened further by the envelope.
            (Param::Cutoff, _) => lerp(0.43, 0.8, x),
            (Param::Reso, _) => 0.6 * x * x,
            (Param::FilterEnv, _) => 0.5 + 0.45 * x * x,
            (Param::Drive, _) => 0.6 * x * x,
            (Param::SatMix, _) => lerp(0.4, 1.0, x),
            // Only ever a rumble filter.
            (Param::LowCut, _) => sometimes(x, 0.7, 0.15),
            (Param::LowFreq, _) => lerp(0.2, 0.5, x),
            (Param::LowGain, _) => lerp(0.5, 0.62, x),
            (Param::MidGain, _) => bipolar(x, 0.3),
            (Param::MidQ, _) => lerp(0.2, 0.7, x),
            (Param::HighGain, _) => lerp(0.5, 0.25, x),
            (Param::ChorusMix, _) => sometimes(x, 0.5, 0.7),
            (Param::ChorusRate, _) => lerp(0.2, 0.8, x),
            (Param::ChorusDepth, _) => lerp(0.2, 0.9, x),
            // The chorus never touches the sub.
            (Param::ChorusXover, _) => lerp(0.4, 0.8, x),
            // Reverb muddies low end: usually a touch, never drenched.
            (Param::ReverbMix, _) => 0.35 * x * x,
            (Param::ReverbDecay, _) => 0.6 * x,
            (Param::ReverbTone, _) => 0.6 * x,
            _ => x,
        });
    }
    // Each engine's macros, within their heavy range.
    let engine = patch[Param::Engine.index()] as u32;
    for (slot, p) in [Param::MacroX, Param::MacroY, Param::MacroZ]
        .into_iter()
        .enumerate()
    {
        if free(p) {
            let (lo, hi) = macro_range(engine, slot);
            patch[p.index()] = lerp(lo, hi, draws[p.index()]);
        }
    }
    // A highpass or bandpass with a lowpass's cutoff would leave little of a
    // bass: pull the cutoff down into the body of the sound, and keep the
    // resonance off the fundamental.
    let ftype = patch[Param::FilterType.index()] as u32;
    if ftype == FILTER_HP || ftype == FILTER_BP {
        if free(Param::Cutoff) {
            let x = draws[Param::Cutoff.index()];
            patch[Param::Cutoff.index()] = if ftype == FILTER_HP {
                0.15 * x
            } else {
                lerp(0.25, 0.45, x)
            };
        }
        if free(Param::Reso) {
            patch[Param::Reso.index()] = patch[Param::Reso.index()].min(0.35);
        }
    }
    // Keep oscillators 2 and 3 above 20 Hz: fold them up an octave if not.
    let base =
        crate::params::map::pitch_hz(patch[Param::Pitch.index()], patch[Param::Fine.index()]);
    for tune in [Param::Tune2, Param::Tune3] {
        if free(tune) {
            let st = crate::params::map::tune_semitones(patch[tune.index()]);
            if base * (st / 12.0).exp2() < 20.0 {
                patch[tune.index()] = crate::params::map::tune_knob(st + 12.0);
            }
        }
    }
    patch
}

/// The part of each engine's macro ranges (X, Y, Z) that RND draws from:
/// where the engine stays heavy rather than thin or fizzy.
pub fn macro_range(engine: u32, slot: usize) -> (f32, f32) {
    use crate::params::*;
    const RANGES: [[(f32, f32); 3]; ENGINE_COUNT] = [
        // SYNC: any wave, sync up to ×2.8, a modest zap.
        [(0.0, 1.0), (0.0, 0.5), (0.0, 0.6)],
        // REESE: up to ~25 cents of detune, a strong centre, saw-ish.
        [(0.1, 0.55), (0.3, 0.9), (0.0, 0.6)],
        // FM: ratios 0.5 … 2, a moderate index that decays.
        [(0.2, 0.6), (0.2, 0.65), (0.05, 0.6)],
        // PD: distortion that relaxes to a sine.
        [(0.2, 0.8), (0.0, 1.0), (0.3, 1.0)],
        // TABLE: dark to saw, a bright attack that darkens.
        [(0.0, 0.85), (0.5, 0.85), (0.3, 0.7)],
        // WT-FM: ratios 0.5 … 2, a moderate index.
        [(0.0, 0.8), (0.2, 0.6), (0.2, 0.6)],
        // BOOM: everything goes.
        [(0.0, 0.8), (0.0, 0.7), (0.0, 0.5)],
        // FOLD: a few folds that close with the envelope.
        [(0.1, 0.6), (0.35, 0.65), (0.5, 1.0)],
    ];
    RANGES[(engine as usize).min(ENGINE_COUNT - 1)][slot.min(2)]
}

/// Move every free control `amount` (0..=1) of the way towards a fresh random
/// patch: `clamp(current + (random - current) * amount)`. At 1 this is exactly
/// [`random_patch`]; choice controls round to the nearest position. Locked
/// controls never move.
pub fn mutate_patch(
    current: &VoicePatch,
    amount: f32,
    locks: &Locks,
    rng: &mut Noise,
) -> VoicePatch {
    let target = random_patch(current, locks, rng);
    if amount >= 1.0 {
        return target;
    }
    let amount = amount.max(0.0);
    let mut patch = *current;
    for p in ALL_PARAMS {
        let (c, t) = (current[p.index()], target[p.index()]);
        patch[p.index()] = p.sanitize(c + (t - c) * amount);
    }
    patch
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ENGINE_COUNT, default_patch};

    fn is_sanitized(patch: &VoicePatch) -> bool {
        ALL_PARAMS
            .iter()
            .all(|p| p.sanitize(patch[p.index()]) == patch[p.index()])
    }

    #[test]
    fn mutate_off_changes_nothing() {
        let current = default_patch();
        assert_eq!(
            mutate_patch(&current, 0.0, &NO_LOCKS, &mut Noise::new(7)),
            current
        );
    }

    #[test]
    fn mutate_at_rnd_equals_the_rnd_button() {
        let current = default_patch();
        let a = mutate_patch(&current, 1.0, &NO_LOCKS, &mut Noise::new(7));
        let b = random_patch(&current, &NO_LOCKS, &mut Noise::new(7));
        assert_eq!(a, b);
        assert_ne!(a, current);
    }

    #[test]
    fn partial_mutation_moves_part_way_towards_the_random_patch() {
        let current = default_patch();
        for seed in 1..50 {
            let target = random_patch(&current, &NO_LOCKS, &mut Noise::new(seed));
            let half = mutate_patch(&current, 0.5, &NO_LOCKS, &mut Noise::new(seed));
            assert!(is_sanitized(&half));
            for p in ALL_PARAMS {
                let (c, t, h) = (current[p.index()], target[p.index()], half[p.index()]);
                match p.info().kind {
                    ParamKind::Continuous => {
                        assert!((h - (c + (t - c) * 0.5)).abs() < 1e-6, "{p:?}")
                    }
                    ParamKind::Choice(_) => assert_eq!(h, p.sanitize(c + (t - c) * 0.5), "{p:?}"),
                }
            }
        }
    }

    #[test]
    fn mutation_keeps_level_sense_and_pan() {
        let mut current = default_patch();
        current[Param::Level.index()] = 0.33;
        current[Param::Sense.index()] = 0.66;
        current[Param::Pan.index()] = 0.1;
        let mut rng = Noise::new(3);
        for amount in [0.25, 0.5, 1.0] {
            let m = mutate_patch(&current, amount, &NO_LOCKS, &mut rng);
            for p in [Param::Level, Param::Sense, Param::Pan] {
                assert_eq!(m[p.index()], current[p.index()]);
            }
        }
    }

    #[test]
    fn locked_controls_never_move() {
        let mut current = default_patch();
        current[Param::Engine.index()] = 3.0;
        current[Param::Pitch.index()] = 0.123;
        current[Param::FilterType.index()] = FILTER_HP as f32;
        current[Param::Cutoff.index()] = 0.9;
        let mut locks = NO_LOCKS;
        for p in [
            Param::Engine,
            Param::Pitch,
            Param::FilterType,
            Param::Cutoff,
        ] {
            locks[p.index()] = true;
        }
        let mut rng = Noise::new(17);
        let mut patch = current;
        for i in 0..500 {
            patch = mutate_patch(&patch, [0.3, 1.0][i % 2], &locks, &mut rng);
            for p in [
                Param::Engine,
                Param::Pitch,
                Param::FilterType,
                Param::Cutoff,
            ] {
                assert_eq!(patch[p.index()], current[p.index()], "{p:?}");
            }
        }
        // …while the free controls do move.
        assert_ne!(patch[Param::Decay.index()], current[Param::Decay.index()]);
    }

    #[test]
    fn locking_one_control_leaves_the_other_rolls_unchanged() {
        let current = default_patch();
        let mut locks = NO_LOCKS;
        locks[Param::Decay.index()] = true;
        locks[Param::MacroY.index()] = true;
        let a = random_patch(&current, &NO_LOCKS, &mut Noise::new(42));
        let b = random_patch(&current, &locks, &mut Noise::new(42));
        for p in ALL_PARAMS {
            if !locks[p.index()] {
                assert_eq!(a[p.index()], b[p.index()], "{p:?}");
            }
        }
    }

    #[test]
    fn a_locked_highpass_still_gets_a_low_cutoff() {
        let mut current = default_patch();
        current[Param::FilterType.index()] = FILTER_HP as f32;
        let mut locks = NO_LOCKS;
        locks[Param::FilterType.index()] = true;
        let mut rng = Noise::new(9);
        for _ in 0..100 {
            let p = random_patch(&current, &locks, &mut rng);
            assert!(p[Param::Cutoff.index()] <= 0.15);
        }
    }

    #[test]
    fn repeated_mutation_stays_in_range() {
        let mut patch = default_patch();
        let mut rng = Noise::new(11);
        for i in 0..1000 {
            patch = mutate_patch(&patch, [0.1, 0.5, 0.9][i % 3], &NO_LOCKS, &mut rng);
            assert!(is_sanitized(&patch));
        }
    }

    /// RND (and MUTATE at RND) must always give something audible and sane,
    /// whichever engine it lands on.
    #[test]
    fn random_patches_are_playable() {
        use crate::voice::Voice;
        let mut rng = Noise::new(2024);
        let mut engines = [0usize; ENGINE_COUNT];
        let fs = 48_000.0;
        for i in 0..1_000 {
            let patch = random_patch(&default_patch(), &NO_LOCKS, &mut rng);
            engines[patch[Param::Engine.index()] as usize] += 1;
            let mut v = Voice::new(fs, i);
            for p in ALL_PARAMS {
                v.set_param(p, patch[p.index()]);
            }
            v.snap_params();
            v.trigger(0.9);
            let out: Vec<f32> = (0..(fs * 0.4) as usize).map(|_| v.tick()).collect();
            let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(out.iter().all(|x| x.is_finite()), "patch {i}");
            assert!(
                peak > 0.02 && peak < 4.0,
                "patch {i}: peak {peak} {patch:?}"
            );
        }
        assert!(engines.iter().all(|&n| n > 60), "engine spread {engines:?}");
    }

    /// SB-2's point: RND hits are bass. At least 85% of them put most of
    /// their energy below 160 Hz (measured on the voice, before the chorus
    /// and reverb).
    #[test]
    fn random_hits_are_bassy() {
        use crate::voice::Voice;
        let fs = 24_000.0;
        let n = 1 << 14;
        let mut rng = Noise::new(77);
        let mut bassy = 0;
        let hits = 200;
        for i in 0..hits {
            let patch = random_patch(&default_patch(), &NO_LOCKS, &mut rng);
            let mut v = Voice::new(fs, i);
            for p in ALL_PARAMS {
                v.set_param(p, patch[p.index()]);
            }
            v.snap_params();
            v.trigger(1.0);
            let mut re: Vec<f64> = (0..n).map(|_| v.tick() as f64).collect();
            let mut im = vec![0.0; n];
            crate::util::fft(&mut re, &mut im, false);
            let (mut low, mut total) = (0.0, 0.0);
            for bin in 1..n / 2 {
                let p = re[bin] * re[bin] + im[bin] * im[bin];
                total += p;
                if (bin as f32 * fs / n as f32) < 160.0 {
                    low += p;
                }
            }
            if low > 0.5 * total {
                bassy += 1;
            }
        }
        assert!(bassy * 100 >= hits * 85, "{bassy}/{hits} bassy hits");
    }
}
