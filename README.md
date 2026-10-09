# Subkussion SB-2

A two-channel **bass percussion** synthesizer: kicks, 808s, sub drops, toms,
Reese hits and growling one-shots. SB-2 is SB-1 with the bass turned up (see
[What SB-2 changes](#what-sb-2-changes)). It is a sibling of
[Xenokussion XK-1](https://github.com/mkforsb/xenokussion-xk1-slop) and keeps
its frame: two channels, a trigger pad per channel, a plate reverb per channel,
MUTATE and RND. Xenokussion's eight engines are replaced by eight oscillator
engines built for low end (virtual analog, unison, FM, phase distortion,
wavetable and hybrids). Every channel runs **three freely tuned oscillators**
of its engine plus a **sub oscillator**, and adds a Moog-style ladder
filter, a saturator, a four-band EQ and a stereo chorus. Every control that RND and MUTATE can change
can be **locked** with Ctrl-click.

It's written in Rust with a [Dioxus](https://dioxuslabs.com) UI and builds for
two targets from one codebase:

| Target | Audio | How |
| --- | --- | --- |
| Web (wasm) | WebAudio `AudioWorklet` | `make serve-web` |
| Native Linux | PulseAudio (also PipeWire's pulse server) | `make run-desktop` |

## Building

Requirements:

- Rust (stable) with `rustup target add wasm32-unknown-unknown`
- Dioxus CLI 0.7: `cargo binstall dioxus-cli` or grab `dx` from the Dioxus releases
- Desktop only (Debian/Ubuntu package names): `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libpulse-dev pkg-config`

```sh
make serve-web      # http://127.0.0.1:8080, click once to enable audio
make run-desktop    # native window
make web desktop    # release bundles in target/dx/subkussion/release/
make test lint
make renders        # offline-render every preset to ./renders/*.wav
make bench          # engine speed as a real-time factor
make calibrate      # loudness survey of every engine
```

make bassiness      # how bassy RND hits are: energy by band, centroid, length
```

For the desktop build, `SUBKUSSION_LATENCY_MS` (default 20) sets the
PulseAudio target latency.

## What SB-2 changes

The goal: with both MUTATE knobs at RND, nearly every hit should be bass.
`make bassiness` rolls 400 RND patches exactly as the panel does, hits each at
full velocity through the whole engine (chorus and reverb included) and
measures it:

| 400 RND hits (seed 1) | SB-1 | SB-2 |
| --- | --- | --- |
| Hits with most of their energy below 160 Hz | 50% | **92%** |
| Median energy below 160 Hz | 49% | **91%** |
| Median energy in the 40–160 Hz punch region | 17% | **59%** |
| Median spectral centroid | 315 Hz | **91 Hz** |
| Median length (to −40 dB) | 0.57 s | **0.96 s** |
| Worst engine (SYNC), energy below 160 Hz | 2.5% | **81%** |

A second seed gives the same picture (94% bassy, centroid 82 Hz), and a test
keeps it that way: at least 85% of RND hits must put most of their energy
below 160 Hz.

The changes:

- **A sub oscillator per channel** (the SUB section): a sine 0, 1 or 2 octaves
  below OSC 1 that follows the pitch sweep, with its own LEVEL and DECAY, and
  a SHAPE knob (a `tanh` drive of up to +24 dB) whose harmonics carry it on
  small speakers.
  - It joins after the ladder filter, so no filter setting can take the
    fundamental away.
  - It never drops below 30 Hz. The octave drop is chosen from the base pitch
    only, never the sweep, so it can't jump mid-hit.
  - Like the other oscillators, it counts towards the stacking
    normalization: adding a sub makes a hit heavier, not just louder.
- **Engines anchored lower:**
  - SYNC blends its hidden master's sine back in as the sync ratio rises (up
    to half the mix): the zap stays, the note doesn't thin out.
  - TABLE's brightest frame, BUZZ, is replaced by **FAT**, a saw with a
    doubled fundamental and a strong octave.
  - The BP and HP ladder modes lost their extra make-up gain, so a resonant
    peak on the fundamental can't jump out.
- **Longer envelopes:** DECAY runs from 30 ms to 6 s (was 25 ms – 4 s), so
  every setting is about a third longer, and HOLD goes up to 0.8 s.
- **RND shaped for bass:**
  - Pitches of 30–95 Hz, leaning low.
  - Decays of 0.4–3 s and more hold.
  - The sub on in 85% of rolls, mostly an octave down.
  - Lowpass about 90% of the time; highpass and bandpass are rare, kept low
    and kept off the resonance.
  - A low-shelf lift of 0 to +4 dB, high-shelf cuts only, little reverb, and
    a chorus crossover that always keeps the sub mono.
  - Each engine's macros are drawn only from the heavy part of their range
    (`random::macro_range`): FM and WT-FM ratios of 0.5–2, sync up to ×2.8,
    folds that close with the envelope.
- **Presets** got sub layers where they fit, three new sub studies (*Sub
  Pressure*, *Tape Sub*, *Reese Abyss*) and a fresh level match.

## Playing it

- **Pads**: click a channel's STRIKE pad. Where you click sets the velocity:
  near the top is a hard hit, near the bottom a soft one. Velocity sets the
  level, and on most engines also the brightness (FM index, drive, fold
  depth, sync sweep, distortion).
- **Keyboard**: `Z X C V` trigger channel 1 at increasing velocity,
  `N M , .` do the same for channel 2, and `Space` triggers both.
- **Knobs/faders**: drag vertically. Hold `Shift` for fine control, use the
  scroll wheel to nudge, and double-click to reset.
- **Locks**: **Ctrl-click** (or ⌘-click) any knob, fader, switch or engine
  button to lock it. A locked control shows a padlock and turns rose, and
  RND and MUTATE leave it exactly where it is. Ctrl-click again to unlock.
  The padlock counter in the channel header shows how many controls are
  locked; click it to unlock them all. Locks belong to the channel, not the
  patch: loading a preset or copying a channel changes locked values but
  keeps them locked. They are saved with the rest of the panel.
  - Everything that RND can change is lockable. OUTPUT, SENSE and PAN are
    never randomized, so they have no lock.
  - Locking never changes what the free controls roll. Every control still
    draws its own random number, in a fixed order, whether or not it's
    locked.
  - Conditional rules follow the locks. If the filter TYPE is locked to HP
    or BP, RND still keeps the cutoff low enough to leave a bass, unless the
    cutoff is locked too.
- **MUTATE** (the knob under each pad) changes the channel's sound on every
  hit. Each trigger draws a fresh random patch, as RND would, and moves each
  free control that fraction of the way towards it:
  `clamp(current + (random − current) × MUTATE)`. At **RND** every hit is a
  new sound. Lock the PITCH and the ENGINE, set MUTATE to 30% and you get a
  kick that keeps its note but evolves its tone with every hit.
- **RND's ranges** are shaped for heavy bass percussion (see above). A test
  renders 1,000 random patches through the real DSP and checks every one is
  audible and bounded; another checks that at least 85% are bass.
- **Presets** (34), **RND** and **→ CHn** (copy a channel to the other) are in
  each channel's header. The whole panel is saved automatically: to
  `localStorage` on the web, and to `~/.config/subkussion-sb2/state.txt` on the
  desktop. The presets are level-matched to within about ±2.5 dB of
  short-term loudness.
- The **HIT** display is an offline render of one hit of the current patch,
  and the **EQ** display is the EQ's actual response, computed by the same
  filter code the voice runs.

## The oscillators

Every engine is an oscillator type. A channel runs three of them, **OSC 1–3**:

- **PITCH** (16 Hz – 1 kHz) and **FINE** tune OSC 1. PITCH is continuous, not
  quantized to notes.
- **TUNE** sets OSC 2 and OSC 3 relative to OSC 1, ±24 semitones,
  continuous. Shift-drag for hundredths of a semitone. Pure intervals
  (+7.02 for 3:2), stretched octaves, slow beating (+0.1) and inharmonic
  ratios are all a turn away. *Gamelan Bass*, *Just Fifth Stab* and *Beating
  Tom* show this off.
- **LEVEL** sets each oscillator's level. Stacked oscillators are scaled back
  towards the loudness of one.
- **DECAY** for OSC 2 and OSC 3 is a multiple of the main DECAY (30 ms – 6
  s): ×0.06 … ×2. A body that outlasts the knock, or a bright partial that
  dies first.
- **SUB**: a sine an octave or two below OSC 1 (never below 30 Hz) with its
  own LEVEL, DECAY multiple and SHAPE (drive). It is mixed in after the
  filter (see below).
- All of them share the envelope's **ATTACK** (0.5–100 ms) and **HOLD** (up
  to 0.8 s, the flat top of an 808), and they all follow the **SWEEP**, the
  chaos modulator and the per-hit **JITTER**.

## The engines

The three big knobs under the faders are the engine's macros, shared by all
three oscillators. They are relabelled for each engine, and so is the chaos
modulator's DEST switch.

| Engine | Model | X | Y | Z |
| --- | --- | --- | --- | --- |
| **SYNC** | Virtual analog: polyBLEP sine/tri/saw/square, hard-synced to a hidden master at the pitch, whose sine is blended back in as the ratio rises | WAVE: sine → tri → saw → square | SYNC: slave ratio ×1 … ×8 | SWEEP: extra octaves of sync ratio at the envelope peak (the sync "zap") |
| **REESE** | Five detuned polyBLEP voices per oscillator, starting at fixed spread phases | DETUNE: ±0 … 80 cents | BLEND: side voices vs centre | WAVE: saw → square |
| **FM** | Two-operator: a sine modulating a sine, with its own index envelope | RATIO: 0.25 … 8, continuous | INDEX: 0 … 12 rad | MOD DEC: index decay ×0.03 … ×1 of the oscillator's decay |
| **PD** | Casio CZ-style phase distortion: a sine read through a bent phase, or a windowed resonant sine | DIST | WAVE: saw → square → reso → pulse | D.ENV: how much of DIST follows the envelope |
| **TABLE** | Band-limited wavetable: SINE, OCTAVE, TRI, ORGAN, NASAL, SQUARE, SAW, FAT | POS | SCAN: bipolar envelope sweep of POS | BEND: bipolar sinusoidal phase warp |
| **WT-FM** | The wavetable as carrier, phase-modulated by a sine | POS | INDEX: 0 … 6 rad, decaying | RATIO: 0.25 … 8 |
| **BOOM** | 808 body: a sine into a `tanh` shaper | DRIVE: up to +30 dB, following the envelope, so the growl cleans up into a pure sine tail | PUNCH: an extra 0–3 octave pitch drop over ~6 ms | ASYM: biases the shaper (even harmonics) |
| **FOLD** | West-coast sine wavefolder | FOLD: ×1 … ×10 | SYMM: bipolar bias | F.ENV: how much of FOLD follows the envelope, like a low-pass gate |

### Anti-aliasing

The voices run at 2× the output rate and a 47-tap windowed-sinc filter
decimates each one. That means a partial between fs/2 and fs − 24 kHz folds
back above 24 kHz, where the decimator removes it. Each engine is built so its
partials stay below that limit:

- **polyBLEP.** SYNC and REESE correct every jump, the hard-sync resets
  included, with a two-sample residual.
- **Smooth shapes.** PD's waves, BOOM's shaper and FOLD's folder only have
  slope breaks, never jumps.
- **Mipmapped tables.** The wavetables are stored at ten bandwidths (512 …
  1 harmonics). Each level is sampled at 8 points per cycle of its top
  harmonic, so linear interpolation stays clean, and the tables are built
  with an inverse FFT.
  - Oscillators crossfade continuously between two adjacent levels. A hard
    switch clicks: the bright frames' top partials line up into a spike
    every cycle.
  - Phase-modulating a whole table multiplies harmonic *k*'s modulation index
    by *k*. So WT-FM picks its level for `k·(carrier + index·modulator)` plus
    a Carson sideband, not just the carrier.
  - BEND uses a sinusoidal warp (phase modulation at ratio 1), not a kinked
    line.
- **Regression test.** A test runs every engine at bright settings through an
  FFT and requires the inharmonic energy (aliases) in 20 Hz – 20 kHz to stay
  below −45 dB. Measured: SYNC −55 dB, PD −60, TABLE −51, WT-FM −66, FM −85,
  BOOM −89, FOLD −89, REESE −94.

## The rest of the channel

```
 PITCH ─┬─ SWEEP ─┬─ CHAOS ─┬─ JITTER ─► pitch ─┬── +TUNE2 ─┬── +TUNE3
        ▼                                     OSC 1      OSC 2      OSC 3    (the engine, ×3, each × LEVEL × its envelope)
 TRIGGER ─► AHD envelopes ─────────────────────┴──────────┴──────────┴─► Σ + CLICK + NOISE
                                                                          │
   ┌──────────────────────────────────────────────────────────────────────┘
   ▼
 [SAT pre] ─► LADDER ─► + ─► EQ ─► [SAT post] ─► OUTPUT ─► CHORUS (mono → stereo) ─┬───────────► PAN ─► mix
                        ▲                                                           └─► PLATE ─┘
 pitch ─► ÷ 2^OCT ─► SUB (sine → shaper) × LEVEL × its envelope
```

- **ENVELOPE · TRANSIENT**: ATTACK and HOLD, then each oscillator decays at
  its own rate.
  - CLICK is one cycle of a windowed sine, the beater tick. It runs 0.6 ms
    on a soft hit and 0.2 ms on a hard one.
  - NOISE is a lowpassed noise burst that velocity opens up.
- **SWEEP**: TIME and a bipolar DEPTH (±6 octaves). Above centre the hit
  starts high and falls to PITCH, the kick's signature.
- **CHAOS**: a slow Lorenz attractor used as an LFO that never repeats. It
  has RATE and DEPTH, a DEST (pitch, X, Y, Z or cutoff) and JITTER, which
  applies per-hit random offsets.
- **LADDER FILTER**: a 4-pole transistor ladder in zero-delay-feedback form.
  - The feedback is solved linearly, then passed through a `tanh`, so
    self-oscillation is bounded.
  - **LP12** and **LP24** tap poles 2 and 4. **BP** and **HP** are
    Xpander-style mixes of the stages, so they resonate too.
  - Feedback is taken against half the input, so resonance costs half as
    much bass as on a plain ladder, and the passband is partly made up.
  - ENV moves the cutoff by up to ±8 octaves with OSC 1's envelope.
- **SATURATOR**: DRIVE (0 … +36 dB), MIX (parallel) and four curves:
  - **TAPE** (`tanh`).
  - **TUBE** (asymmetric, even harmonics).
  - **HARD** (cubic clip).
  - **FOLD** (sine folder).

  An output trim keeps a −6 dBFS signal at −6 dBFS at any drive. **POS**
  puts it before the filter, or after the EQ so the EQ drives it. DRIVE at
  zero is a true bypass.
- **EQ**, with generous low-end range:
  - **LO CUT**: 12 dB/oct highpass, 10–400 Hz; off at zero.
  - **LOW** shelf: 20–800 Hz, ±18 dB.
  - **MID** bell: 25 Hz – 8 kHz, ±18 dB, Q 0.3–9.
  - **HIGH** shelf: 0.8–16 kHz, ±18 dB.

  RBJ biquads; a flat band is bypassed.
- **CHORUS**: three taps swept 120° apart by one LFO. Left hears taps 1 and
  2, right hears taps 3 and 2 inverted: wide, with a hollow comb against the
  dry signal.
  - **X-OVER** (20–600 Hz, off at zero) splits the signal with a 4th-order
    Linkwitz–Riley crossover. Everything below it skips the chorus and stays
    mono and in phase, so the sub keeps its weight. A test checks that a
    40 Hz tone stays correlated across L/R while 2 kHz is spread.
- **REVERB**: Xenokussion's per-channel Dattorro plate, unchanged, fed with
  the chorus output. AMOUNT (post fader), DECAY, TONE and PRE. PAN acts as a
  balance on the stereo signal.
- **Performance**: with both channels hitting, three oscillators each and the
  whole strip busy, the slowest engine renders about 29× faster than real
  time on one desktop core (see `make bench`).

## Layout

```
crates/dsp        engine, the 8 engines, wavetables, transient, chaos modulator, ladder,
                  saturator, EQ, chorus, plate reverb, parameters, presets and the hit
                  preview, RND/MUTATE/locks; no dependencies, 70 unit tests
  src/engines/      sync, reese, fm, pd, table, wtfm, boom, fold; tables (the wavetable bank)
  examples/         render (presets → WAV), bench, calibrate (loudness survey),
                    analyze (RMS/pitch/brightness over time), probe (which knob made it quiet?),
                    bassiness (how bassy RND hits are)
crates/worklet    C ABI over the engine, compiled to a standalone wasm module with no imports
crates/app        Dioxus UI with `web` and `desktop` features; panel state and persistence
  build.rs          (web) builds crates/worklet for wasm32 and embeds it
  src/audio/web.rs     AudioContext + AudioWorkletNode; the module is compiled on the main thread
  src/audio/worklet.js the AudioWorkletProcessor that calls the wasm engine
  src/audio/pulse.rs   audio thread + pa_simple playback stream
```

The UI never touches the audio engine directly. It sends small commands (set
a parameter, trigger, master volume): over the worklet's `MessagePort` on the
web, and over an `mpsc` channel on the desktop. Both backends render in
blocks of 128 frames. Locks live only in the UI; the engine never sees them.

## Inspiration and sources

- Roland **TR-808** bass drum (the sine-with-pitch-drop kick, the long
  "boom") and the overdriven 808 of trap and drill
- The **Reese** bass (Kevin Saunderson, "Just Want Another Chance", 1988) and
  its drum & bass descendants
- Casio **CZ** series phase distortion: <https://en.wikipedia.org/wiki/Phase_distortion_synthesis>
- Buchla / west-coast wavefolding
- Oberheim **Xpander** ladder mode mixing; V. Zavalishin, *The Art of VA
  Filter Design* (ZDF ladder)
- V. Välimäki et al., polyBLEP oscillators; R. Bristow-Johnson, *Audio EQ
  Cookbook*
- J. Dattorro, "Effect Design, Part 1: Reverberator and Other Filters", JAES 45(9), 1997 (plate reverb)
- E. N. Lorenz, "Deterministic Nonperiodic Flow", J. Atmos. Sci. 20, 1963 (the chaos modulator)
