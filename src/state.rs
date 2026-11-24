//! Keadaan yang dibaca template saat menggambar.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::image::Img;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgMode {
    Album,
    Image,
    Color,
}

impl BgMode {
    pub fn from_id(s: &str) -> Option<BgMode> {
        match s {
            "album" => Some(BgMode::Album),
            "image" => Some(BgMode::Image),
            "color" => Some(BgMode::Color),
            _ => None,
        }
    }
}

/// Aset bersama (mis. latar tetap milik template) yang dimuat sekali saat start.
pub type Assets = Rc<RefCell<HashMap<String, Rc<Img>>>>;

/// Semua pengaturan pengguna. `rev` naik setiap ada perubahan sehingga lapisan statis
/// bisa di-cache dan dibuat ulang hanya ketika perlu.
#[derive(Clone)]
pub struct Scene {
    pub rev: u64,
    pub title: String,
    pub artist: String,
    /// Kolom tambahan milik template (device, h1, h2, caption, ...), diindeks dengan id kolom.
    pub fields: HashMap<String, String>,
    pub blur: f64,
    pub dim: f64,
    pub vol: f64,
    pub bg_mode: BgMode,
    pub c1: String,
    pub c2: String,
    pub art: Option<Rc<Img>>,
    /// `true` bila foto album dipilih pengguna (bukan gambar contoh).
    pub art_custom: bool,
    pub bg_img: Option<Rc<Img>>,
    pub assets: Assets,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            rev: 1,
            title: String::new(),
            artist: String::new(),
            fields: HashMap::new(),
            blur: 60.0,
            dim: 20.0,
            vol: 0.48,
            bg_mode: BgMode::Album,
            c1: "#4a3a52".into(),
            c2: "#10131a".into(),
            art: None,
            art_custom: false,
            bg_img: None,
            assets: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn touch(&mut self) {
        self.rev += 1;
    }

    pub fn disp_title(&self) -> &str {
        let t = self.title.trim();
        if t.is_empty() {
            "Judul Lagu"
        } else {
            t
        }
    }

    pub fn disp_artist(&self) -> &str {
        let t = self.artist.trim();
        if t.is_empty() {
            "Nama Artis"
        } else {
            t
        }
    }

    /// Nilai kolom template ("" bila belum ada).
    pub fn field(&self, id: &str) -> &str {
        self.fields.get(id).map(String::as_str).unwrap_or("")
    }

    pub fn asset(&self, key: &str) -> Option<Rc<Img>> {
        self.assets.borrow().get(key).cloned()
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// Keadaan per frame (waktu, posisi lagu, spektrum, fade).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// Waktu animasi (detik) untuk marquee dan animasi kecil.
    pub t: f64,
    pub cur: f64,
    pub dur: f64,
    pub playing: bool,
    pub spec: [f64; 5],
    /// 0 = tidak ada fade, 1 = tertutup penuh.
    pub fade: f64,
    pub fade_color: &'static str,
}

impl Frame {
    /// Frame contoh untuk pratinjau statis (katalog).
    pub fn demo() -> Frame {
        Frame {
            t: 1.0,
            cur: 1.0,
            dur: 67.0,
            playing: true,
            spec: [0.35, 0.7, 0.5, 0.85, 0.45],
            fade: 0.0,
            fade_color: "#000",
        }
    }

    pub fn progress(&self) -> f64 {
        if self.dur > 0.0 {
            (self.cur.min(self.dur) / self.dur).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}
