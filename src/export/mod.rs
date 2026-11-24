//! Ekspor video. Dua jalur:
//! * `fast`: frame dan suara dirender offline (tanpa menunggu lagu diputar) lalu dikirim ke
//!   WebCodecs. Jauh lebih cepat dari lagunya dan hasilnya deterministik.
//! * `realtime`: cadangan lewat MediaRecorder untuk browser tanpa WebCodecs.
//!
//! Semua perhitungan (frame, FFT, fade, jadwal) ada di Rust; `web/bridge.js` hanya menjembatani API browser.

pub mod fast;
pub mod realtime;

use std::cell::Cell;
use std::rc::Rc;

use crate::geom::Orient;
use crate::state::Scene;
use crate::templates::Template;

/// Pengaturan yang dibaca dari dashboard.
#[derive(Clone, Debug)]
pub struct Cfg {
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub fade_in: f64,
    pub fade_out: f64,
    pub fade_color: &'static str,
    pub vbr: f64,
}

/// Preset kualitas: pengali bitrate terhadap piksel x fps. `Mid` = rumus versi HTML (0.14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Low,
    Mid,
    High,
}

impl Quality {
    pub fn from_id(s: &str) -> Quality {
        match s {
            "low" => Quality::Low,
            "high" => Quality::High,
            _ => Quality::Mid,
        }
    }
    fn factor(self) -> f64 {
        match self {
            Quality::Low => 0.08,
            Quality::Mid => 0.14,
            Quality::High => 0.24,
        }
    }
}

impl Cfg {
    pub fn new(
        orient: Orient,
        short: u32,
        fps: u32,
        q: Quality,
        fade_in: f64,
        fade_out: f64,
        white: bool,
    ) -> Cfg {
        let (w, h) = orient.dims(short);
        Cfg {
            w,
            h,
            fps,
            fade_in,
            fade_out,
            fade_color: if white { "#ffffff" } else { "#000000" },
            vbr: bitrate(w, h, fps, q),
        }
    }
}

/// Bitrate video (bit/dtk): sebanding dengan piksel x fps, dijepit 1.5..40 Mbps.
pub fn bitrate(w: u32, h: u32, fps: u32, q: Quality) -> f64 {
    (w as f64 * h as f64 * fps as f64 * q.factor())
        .round()
        .clamp(1.5e6, 40e6)
}

/// Penguatan fade in/out (smoothstep). `p` = posisi (detik) dari `len`.
pub fn fade_gain(p: f64, len: f64, fi: f64, fo: f64) -> f64 {
    let fi = fi.min(len / 2.0);
    let fo = fo.min(len / 2.0);
    let mut a: f64 = 1.0;
    if fi > 0.0 {
        a = a.min(p / fi);
    }
    if fo > 0.0 {
        a = a.min((len - p) / fo);
    }
    let a = a.clamp(0.0, 1.0);
    a * a * (3.0 - 2.0 * a)
}

/// Jumlah frame video untuk lagu `len` detik.
pub fn frame_count(len: f64, fps: u32) -> u32 {
    ((len * fps as f64 - 0.001).ceil() as i64).max(1) as u32
}

#[derive(Debug)]
pub enum ExportError {
    Cancelled,
    Failed(String),
}

impl From<wasm_bindgen::JsValue> for ExportError {
    fn from(v: wasm_bindgen::JsValue) -> Self {
        let msg = v
            .as_string()
            .or_else(|| {
                js_sys::Reflect::get(&v, &"message".into())
                    .ok()
                    .and_then(|m| m.as_string())
            })
            .unwrap_or_else(|| "kesalahan tidak diketahui".into());
        ExportError::Failed(msg)
    }
}

/// Bahan yang dibutuhkan kedua jalur ekspor (disalin dari aplikasi sebelum `await` pertama).
pub struct Job {
    pub cfg: Cfg,
    pub tpl: Rc<dyn Template>,
    pub scene: Scene,
    pub orient: Orient,
    pub cancel: Rc<Cell<bool>>,
    /// Laporan kemajuan (teks untuk tombol simpan).
    pub progress: Rc<dyn Fn(String)>,
}

pub fn fmt_hms(sec: f64) -> String {
    crate::geom::fmt_time(sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_is_zero_at_edges_and_one_in_middle() {
        assert_eq!(fade_gain(0.0, 100.0, 2.0, 3.0), 0.0);
        assert_eq!(fade_gain(100.0, 100.0, 2.0, 3.0), 0.0);
        assert_eq!(fade_gain(50.0, 100.0, 2.0, 3.0), 1.0);
        let mid = fade_gain(1.0, 100.0, 2.0, 3.0);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(fade_gain(0.0, 100.0, 0.0, 0.0), 1.0);
    }

    #[test]
    fn fade_is_monotonic_during_fade_in() {
        let mut prev = -1.0;
        for i in 0..=20 {
            let g = fade_gain(i as f64 * 0.1, 60.0, 2.0, 0.0);
            assert!(g >= prev);
            prev = g;
        }
    }

    #[test]
    fn fades_are_capped_to_half_the_song() {
        // fade 10 dtk pada lagu 4 dtk dibatasi 2 dtk: titik tengah harus penuh.
        assert_eq!(fade_gain(2.0, 4.0, 10.0, 10.0), 1.0);
    }

    #[test]
    fn frame_count_matches_duration() {
        assert_eq!(frame_count(10.0, 30), 300);
        assert_eq!(frame_count(10.01, 30), 301);
        assert_eq!(frame_count(0.0, 30), 1);
    }

    #[test]
    fn bitrate_is_clamped() {
        assert_eq!(bitrate(540, 960, 24, Quality::Low), 1.5e6);
        assert_eq!(bitrate(1440, 2560, 60, Quality::High), 40e6);
        let b = bitrate(720, 1280, 30, Quality::Mid);
        assert!(b > 2.5e6 && b < 30e6);
    }

    #[test]
    fn mid_quality_matches_original_html_formula() {
        // versi HTML: round(clamp(W*H*fps*0.14, 2.5e6, 30e6)); pada 720p/30fps tidak ada jepitan.
        assert_eq!(
            bitrate(720, 1280, 30, Quality::Mid),
            (720.0 * 1280.0 * 30.0 * 0.14_f64).round()
        );
        assert!(bitrate(720, 1280, 30, Quality::Low) < bitrate(720, 1280, 30, Quality::Mid));
        assert!(bitrate(720, 1280, 30, Quality::Mid) < bitrate(720, 1280, 30, Quality::High));
    }

    #[test]
    fn cfg_follows_orientation() {
        let c = Cfg::new(Orient::Landscape, 720, 30, Quality::Mid, 1.0, 1.5, true);
        assert_eq!((c.w, c.h), (1280, 720));
        assert_eq!(c.fade_color, "#ffffff");
    }
}
