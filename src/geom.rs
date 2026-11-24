//! Tipe geometri dasar. Seluruh tata letak memakai satuan "U" = 1% lebar kartu
//! (sama dengan satuan `cqw` pada versi HTML), sehingga kartu tampil identik di
//! resolusi berapa pun.

/// Orientasi kanvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orient {
    Portrait,
    Landscape,
}

impl Orient {
    pub const ALL: [Orient; 2] = [Orient::Portrait, Orient::Landscape];

    /// Tinggi kartu dalam satuan U (lebar selalu 100).
    pub fn hh(self) -> f64 {
        match self {
            Orient::Portrait => 100.0 * 16.0 / 9.0,
            Orient::Landscape => 100.0 * 9.0 / 16.0,
        }
    }

    pub fn is_landscape(self) -> bool {
        self == Orient::Landscape
    }

    pub fn ratio_label(self) -> &'static str {
        match self {
            Orient::Portrait => "9:16",
            Orient::Landscape => "16:9",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Orient::Portrait => "portrait",
            Orient::Landscape => "landscape",
        }
    }

    pub fn from_id(s: &str) -> Option<Orient> {
        match s {
            "portrait" => Some(Orient::Portrait),
            "landscape" => Some(Orient::Landscape),
            _ => None,
        }
    }

    /// Ukuran piksel untuk sisi pendek `short`.
    pub fn dims(self, short: u32) -> (u32, u32) {
        let long = (short as f64 * 16.0 / 9.0).round() as u32;
        match self {
            Orient::Portrait => (short, long),
            Orient::Landscape => (long, short),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
    /// Persegi panjang berpusat di (cx, cy).
    pub fn centered(cx: f64, cy: f64, w: f64, h: f64) -> Self {
        Rect::new(cx - w / 2.0, cy - h / 2.0, w, h)
    }
}

/// Persegi panjang dengan radius sudut per sudut: kiri-atas, kanan-atas, kanan-bawah, kiri-bawah.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub r: [f64; 4],
}

impl RRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64, r: f64) -> Self {
        RRect {
            x,
            y,
            w,
            h,
            r: [r; 4],
        }
    }
    pub fn with_radii(x: f64, y: f64, w: f64, h: f64, r: [f64; 4]) -> Self {
        RRect { x, y, w, h, r }
    }
    /// Radius dibatasi agar tidak melebihi setengah sisi terpendek.
    pub fn clamped(&self) -> [f64; 4] {
        let m = (self.w.min(self.h) / 2.0).max(0.0);
        [
            self.r[0].min(m),
            self.r[1].min(m),
            self.r[2].min(m),
            self.r[3].min(m),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Format `m:ss` seperti pada versi lama.
pub fn fmt_time(sec: f64) -> String {
    let s = if sec.is_finite() && sec > 0.0 {
        sec.floor() as u64
    } else {
        0
    };
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dims_follow_orientation() {
        assert_eq!(Orient::Portrait.dims(720), (720, 1280));
        assert_eq!(Orient::Landscape.dims(720), (1280, 720));
        assert_eq!(Orient::Portrait.dims(1440), (1440, 2560));
    }

    #[test]
    fn time_format() {
        assert_eq!(fmt_time(0.0), "0:00");
        assert_eq!(fmt_time(67.9), "1:07");
        assert_eq!(fmt_time(f64::NAN), "0:00");
        assert_eq!(fmt_time(3599.0), "59:59");
    }

    #[test]
    fn radii_are_clamped() {
        let r = RRect::new(0.0, 0.0, 10.0, 4.0, 50.0);
        assert_eq!(r.clamped(), [2.0; 4]);
    }
}
