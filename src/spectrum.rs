//! Spektrum 5 batang. Dipakai saat putar langsung (AnalyserNode) dan saat ekspor
//! (FFT sendiri yang meniru AnalyserNode agar video sama dengan tampilan live).

pub const WINDOW: usize = 512;
pub const IDLE: [f64; 5] = [0.06; 5];
const BANDS: [(usize, usize, f64); 5] = [
    (1, 4, 0.85),
    (4, 8, 0.95),
    (8, 16, 1.05),
    (16, 32, 1.15),
    (32, 64, 1.3),
];

/// Ubah 256 byte frekuensi (0..255) menjadi target 5 batang.
pub fn band_targets(buf: &[u8]) -> [f64; 5] {
    let mut out = [0.0; 5];
    for (i, (a, b, gain)) in BANDS.iter().enumerate() {
        let (mut sum, mut n) = (0.0, 0.0);
        for k in *a..(*b).min(buf.len()) {
            sum += buf[k] as f64;
            n += 1.0;
        }
        let v = if n > 0.0 {
            (sum / n / 255.0 * gain).min(1.0)
        } else {
            0.0
        };
        out[i] = v.powf(1.45) * 0.9;
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub struct Spectrum {
    pub v: [f64; 5],
}

impl Default for Spectrum {
    fn default() -> Self {
        Spectrum { v: IDLE }
    }
}

impl Spectrum {
    /// Satu langkah menuju target pada laju 60 fps (naik cepat, turun pelan).
    pub fn step(&mut self, target: &[f64; 5], fps: f64) {
        let k = 60.0 / fps.max(1.0);
        for i in 0..5 {
            let rate = if target[i] > self.v[i] { 0.55 } else { 0.2 };
            self.v[i] += (target[i] - self.v[i]) * (1.0 - (1.0_f64 - rate).powf(k));
        }
    }
    pub fn reset(&mut self) {
        self.v = IDLE;
    }
}

/// Tinggi relatif batang (0.22..1.0) dari nilai spektrum.
pub fn bar_frac(v: f64) -> f64 {
    0.22 + 0.78 * v.clamp(0.0, 1.0)
}

const N: usize = 512;

/// FFT radix-2 512 titik + jendela Blackman + peredaman, meniru `AnalyserNode.getByteFrequencyData`.
pub struct Analyzer {
    cos: Vec<f32>,
    sin: Vec<f32>,
    rev: Vec<usize>,
    win: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    sm: Vec<f32>,
    buf: Vec<u8>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        let bits = N.trailing_zeros();
        let cos = (0..N / 2)
            .map(|i| (2.0 * std::f64::consts::PI * i as f64 / N as f64).cos() as f32)
            .collect();
        let sin = (0..N / 2)
            .map(|i| (-(2.0 * std::f64::consts::PI * i as f64 / N as f64).sin()) as f32)
            .collect();
        let rev = (0..N)
            .map(|j| j.reverse_bits() >> (usize::BITS - bits))
            .collect();
        let al = 0.16_f64;
        let (a0, a1, a2) = ((1.0 - al) / 2.0, 0.5, al / 2.0);
        let win = (0..N)
            .map(|n| {
                let x = n as f64 / N as f64;
                (a0 - a1 * (2.0 * std::f64::consts::PI * x).cos()
                    + a2 * (4.0 * std::f64::consts::PI * x).cos()) as f32
            })
            .collect();
        Analyzer {
            cos,
            sin,
            rev,
            win,
            re: vec![0.0; N],
            im: vec![0.0; N],
            sm: vec![0.0; N / 2],
            buf: vec![0; N / 2],
        }
    }

    pub fn reset(&mut self) {
        self.sm.iter_mut().for_each(|v| *v = 0.0);
    }

    /// Byte frekuensi untuk jendela yang berakhir pada sampel `end` (mono = rata-rata L dan R).
    /// Dipakai pengujian; ekspor memanggil `bytes_window` langsung dengan 512 sampel.
    pub fn bytes(&mut self, l: &[f32], r: &[f32], end: usize, decay: f64) -> &[u8] {
        let start = end as isize - N as isize;
        let mut win = vec![0.0f32; N];
        for (i, w) in win.iter_mut().enumerate() {
            let k = start + i as isize;
            if k >= 0 && (k as usize) < l.len() {
                let kk = k as usize;
                *w = (l[kk] + r.get(kk).copied().unwrap_or(l[kk])) * 0.5;
            }
        }
        self.bytes_window(&win, decay)
    }

    /// `mono` harus berisi tepat 512 sampel (sudah di-nol-kan di luar lagu).
    pub fn bytes_window(&mut self, mono: &[f32], decay: f64) -> &[u8] {
        debug_assert_eq!(mono.len(), N);
        for i in 0..N {
            self.re[self.rev[i]] = mono[i] * self.win[i];
            self.im[self.rev[i]] = 0.0;
        }
        let mut size = 2;
        while size <= N {
            let half = size >> 1;
            let step = N / size;
            let mut s = 0;
            while s < N {
                let mut t = 0;
                for j in 0..half {
                    let (wr, wi) = (self.cos[t], self.sin[t]);
                    let (a, b) = (s + j, s + j + half);
                    let tr = wr * self.re[b] - wi * self.im[b];
                    let ti = wr * self.im[b] + wi * self.re[b];
                    self.re[b] = self.re[a] - tr;
                    self.im[b] = self.im[a] - ti;
                    self.re[a] += tr;
                    self.im[a] += ti;
                    t += step;
                }
                s += size;
            }
            size <<= 1;
        }
        let d = decay as f32;
        for q in 0..N / 2 {
            let mag = (self.re[q] * self.re[q] + self.im[q] * self.im[q]).sqrt() / N as f32;
            self.sm[q] = d * self.sm[q] + (1.0 - d) * mag;
            let db = 20.0 * (self.sm[q] as f64 + 1e-12).log10();
            let by = (255.0 * (db + 100.0) / 70.0).round();
            self.buf[q] = by.clamp(0.0, 255.0) as u8;
        }
        &self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq_bin: f64, len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| {
                (2.0 * std::f64::consts::PI * freq_bin * i as f64 / N as f64).sin() as f32 * 0.01
            })
            .collect()
    }

    #[test]
    fn silence_gives_zero_bytes() {
        let mut a = Analyzer::new();
        let z = vec![0.0f32; 4096];
        assert!(a.bytes(&z, &z, 2048, 0.0).iter().all(|&b| b == 0));
    }

    #[test]
    fn tone_peaks_at_expected_bin() {
        let mut a = Analyzer::new();
        let s = tone(10.0, 4096);
        let b = a.bytes(&s, &s, 2048, 0.0).to_vec();
        let (peak, _) = b.iter().enumerate().max_by_key(|(_, v)| **v).unwrap();
        assert_eq!(peak, 10);
        assert!(b[10] > 100 && b[10] < 255, "tidak boleh jenuh: {}", b[10]);
        assert!(b[100] < b[10]);
    }

    #[test]
    fn window_before_start_of_audio_is_zero_padded() {
        let mut a = Analyzer::new();
        let s = tone(10.0, 4096);
        // end=100 berarti sebagian besar jendela di luar audio; tidak boleh panik
        let _ = a.bytes(&s, &s, 100, 0.55);
    }

    #[test]
    fn idle_targets_converge_and_bars_stay_in_range() {
        let mut sp = Spectrum::default();
        for _ in 0..200 {
            sp.step(&[0.8; 5], 60.0);
        }
        assert!((sp.v[0] - 0.8).abs() < 0.01);
        assert!(bar_frac(2.0) <= 1.0 && bar_frac(-1.0) >= 0.22);
    }

    #[test]
    fn step_is_framerate_independent_over_one_second() {
        let (mut a, mut b) = (Spectrum::default(), Spectrum::default());
        for _ in 0..30 {
            a.step(&[1.0; 5], 30.0);
        }
        for _ in 0..60 {
            b.step(&[1.0; 5], 60.0);
        }
        assert!((a.v[0] - b.v[0]).abs() < 1e-9);
    }

    #[test]
    fn band_targets_scale_with_input() {
        let lo = band_targets(&[0u8; 256]);
        let hi = band_targets(&[255u8; 256]);
        assert!(lo.iter().all(|&v| v == 0.0));
        assert!(hi.iter().all(|&v| v > 0.5 && v <= 0.9 + 1e-9));
    }
}
