//! How bassy are random hits? Rolls `N` patches exactly as RND (or MUTATE at
//! RND) does, hits each at full velocity through the whole engine (chorus and
//! reverb included) and reports where the energy sits and how long it lasts.
//!
//! ```sh
//! cargo run -p subkussion-dsp --release --example bassiness -- 400
//! ```

use subkussion_dsp::params::{ENGINE_NAMES, default_patch};
use subkussion_dsp::random::{NO_LOCKS, random_patch};
use subkussion_dsp::util::{Noise, fft};
use subkussion_dsp::{Engine, Param};

const SR: f32 = 48_000.0;
const SECONDS: f32 = 4.0;
/// Band edges in Hz.
const BANDS: [f32; 5] = [40.0, 80.0, 160.0, 500.0, 2_000.0];

struct Hit {
    engine: usize,
    /// Energy fraction per band: <40, 40–80, 80–160, 160–500, 500–2k, >2k.
    bands: [f64; 6],
    centroid: f64,
    /// Seconds until the short-term level falls 40 dB below its peak.
    length: f32,
    /// Peak short-term (50 ms) level, dBFS.
    loudness: f32,
    /// Sample peak, dBFS.
    peak: f32,
}

fn analyze(x: &[f32], engine: usize) -> Hit {
    let n = x.len().next_power_of_two();
    let mut re: Vec<f64> = x.iter().map(|&v| v as f64).collect();
    re.resize(n, 0.0);
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im, false);
    let mut bands = [0.0f64; 6];
    let (mut total, mut moment) = (0.0, 0.0);
    for bin in 1..n / 2 {
        let f = bin as f64 * SR as f64 / n as f64;
        let p = re[bin] * re[bin] + im[bin] * im[bin];
        let b = BANDS.iter().position(|&e| f < e as f64).unwrap_or(5);
        bands[b] += p;
        total += p;
        moment += p * f;
    }
    bands.iter_mut().for_each(|b| *b /= total.max(1e-30));
    let win = (0.05 * SR) as usize;
    let st: Vec<f32> = x
        .chunks(win / 5)
        .map(|c| c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32)
        .collect::<Vec<_>>()
        .windows(5)
        .map(|w| w.iter().sum::<f32>() / 5.0)
        .collect();
    let peak = st.iter().cloned().fold(0.0f32, f32::max);
    let last = st.iter().rposition(|&e| e > peak * 1e-4).unwrap_or(0);
    Hit {
        engine,
        bands,
        centroid: moment / total.max(1e-30),
        length: (last + 5) as f32 * (win / 5) as f32 / SR,
        loudness: 10.0 * peak.max(1e-12).log10(),
        peak: 20.0
            * x.iter()
                .fold(0.0f32, |m, v| m.max(v.abs()))
                .max(1e-9)
                .log10(),
    }
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * q) as usize]
}

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let seed: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut rng = Noise::new(seed);
    let mut hits = Vec::new();
    for _ in 0..n {
        let patch = random_patch(&default_patch(), &NO_LOCKS, &mut rng);
        let mut e = Engine::new(SR);
        e.load_patch(0, &patch);
        e.snap_params();
        e.trigger(0, 1.0);
        let (l, r) = e.render_vec((SR * SECONDS) as usize);
        let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)).collect();
        hits.push(analyze(&mono, patch[Param::Engine.index()] as usize));
    }
    let col = |f: &dyn Fn(&Hit) -> f64| hits.iter().map(f).collect::<Vec<f64>>();
    let show = |name: &str, mut v: Vec<f64>, unit: &str| {
        println!(
            "{name:<22} p25 {:>8.2}  median {:>8.2}  p75 {:>8.2} {unit}",
            pct(&mut v, 0.25),
            pct(&mut v, 0.5),
            pct(&mut v, 0.75)
        );
    };
    println!("{n} random hits (seed {seed}), full velocity, whole engine");
    let labels = [
        "< 40 Hz",
        "40-80 Hz",
        "80-160 Hz",
        "160-500 Hz",
        "500-2k Hz",
        "> 2 kHz",
    ];
    for (i, l) in labels.iter().enumerate() {
        show(&format!("energy {l}"), col(&|h| 100.0 * h.bands[i]), "%");
    }
    show(
        "energy < 160 Hz",
        col(&|h| 100.0 * (h.bands[0] + h.bands[1] + h.bands[2])),
        "%",
    );
    show(
        "energy 40-160 Hz",
        col(&|h| 100.0 * (h.bands[1] + h.bands[2])),
        "%",
    );
    show("spectral centroid", col(&|h| h.centroid), "Hz");
    show("length to -40 dB", col(&|h| h.length as f64), "s");
    show("peak loudness", col(&|h| h.loudness as f64), "dBFS");
    show("sample peak", col(&|h| h.peak as f64), "dBFS");
    let hot = hits.iter().filter(|h| h.peak > -6.0).count();
    println!("hits reaching the output soft clip (> -6 dBFS): {hot}/{n}");
    let bassy = hits
        .iter()
        .filter(|h| h.bands[0] + h.bands[1] + h.bands[2] > 0.5)
        .count();
    println!(
        "hits with most energy below 160 Hz: {bassy}/{n} ({:.0}%)",
        100.0 * bassy as f64 / n as f64
    );
    println!("\nper engine: median energy < 160 Hz / median centroid / median length");
    for (e, name) in ENGINE_NAMES.iter().enumerate() {
        let sel: Vec<&Hit> = hits.iter().filter(|h| h.engine == e).collect();
        if sel.is_empty() {
            continue;
        }
        let mut low: Vec<f64> = sel
            .iter()
            .map(|h| 100.0 * (h.bands[0] + h.bands[1] + h.bands[2]))
            .collect();
        let mut cen: Vec<f64> = sel.iter().map(|h| h.centroid).collect();
        let mut len: Vec<f64> = sel.iter().map(|h| h.length as f64).collect();
        println!(
            "  {name:<6} n={:<3} {:>5.1}%  {:>6.0} Hz  {:>5.2} s",
            sel.len(),
            pct(&mut low, 0.5),
            pct(&mut cen, 0.5),
            pct(&mut len, 0.5)
        );
    }
}
