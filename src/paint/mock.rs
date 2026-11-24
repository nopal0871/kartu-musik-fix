//! Painter palsu untuk unit test: merekam perintah gambar sebagai teks.

use super::Painter;
use crate::geom::{Align, RRect};
use crate::image::Img;

pub struct Mock {
    pub w: f64,
    pub h: f64,
    pub log: Vec<String>,
    pub depth: u32,
    pub max_depth: u32,
}

impl Mock {
    pub fn new(w: f64, h: f64) -> Self {
        Mock {
            w,
            h,
            log: Vec::new(),
            depth: 0,
            max_depth: 0,
        }
    }
    pub fn count(&self, prefix: &str) -> usize {
        self.log.iter().filter(|l| l.starts_with(prefix)).count()
    }
    pub fn has_text(&self, s: &str) -> bool {
        self.log
            .iter()
            .any(|l| l.starts_with("text ") && l.contains(s))
    }
}

impl Painter for Mock {
    fn size(&self) -> (f64, f64) {
        (self.w, self.h)
    }
    fn save(&mut self) {
        self.log.push("save".into());
    }
    fn restore(&mut self) {
        self.log.push("restore".into());
    }
    fn clip_rrect(&mut self, _r: RRect) {
        self.log.push("clip".into());
    }
    fn fill_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, c: &str) {
        self.log.push(format!("fill_rect {c}"));
    }
    fn fill_rect_vgrad(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _s: &[(f64, &str)]) {
        self.log.push("fill_rect_vgrad".into());
    }
    fn fill_rrect(&mut self, _r: RRect, c: &str) {
        self.log.push(format!("fill_rrect {c}"));
    }
    fn fill_rrect_vgrad(&mut self, _r: RRect, _s: &[(f64, &str)]) {
        self.log.push("fill_rrect_vgrad".into());
    }
    fn stroke_rrect(&mut self, _r: RRect, _w: f64, c: &str) {
        self.log.push(format!("stroke_rrect {c}"));
    }
    fn fill_circle(&mut self, _x: f64, _y: f64, _r: f64, c: &str) {
        self.log.push(format!("circle {c}"));
    }
    fn fill_ellipse(&mut self, _x: f64, _y: f64, _rx: f64, _ry: f64, c: &str) {
        self.log.push(format!("ellipse {c}"));
    }
    fn image_cover(&mut self, _i: &Img, _x: f64, _y: f64, _w: f64, _h: f64) {
        self.log.push("image".into());
    }
    fn image_cover_blur(&mut self, _i: &Img, _x: f64, _y: f64, _w: f64, _h: f64, b: f64) {
        self.log.push(format!("image_blur {b:.1}"));
    }
    fn text(&mut self, s: &str, _x: f64, _y: f64, _f: &str, _c: &str, _a: Align) {
        self.log.push(format!("text {s}"));
    }
    /// Lebar tiruan: 0.5 x ukuran font per karakter.
    fn measure(&mut self, s: &str, font: &str) -> f64 {
        let px: f64 = font
            .split_whitespace()
            .find_map(|t| t.strip_suffix("px").and_then(|n| n.parse().ok()))
            .unwrap_or(10.0);
        s.chars().count() as f64 * px * 0.5
    }
    fn svg_path(
        &mut self,
        d: &str,
        _x: f64,
        _y: f64,
        _s: f64,
        f: Option<&str>,
        st: Option<(&str, f64)>,
    ) {
        self.log.push(format!(
            "path {} fill={} stroke={}",
            d.len(),
            f.is_some(),
            st.is_some()
        ));
    }
    fn layer(
        &mut self,
        _x: f64,
        _y: f64,
        w: f64,
        h: f64,
        fl: f64,
        fr: f64,
        f: &mut dyn FnMut(&mut dyn Painter),
    ) {
        self.log
            .push(format!("layer {w:.0}x{h:.0} fade={fl:.1}/{fr:.1}"));
        let mut inner = Mock::new(w, h);
        f(&mut inner);
        self.log
            .extend(inner.log.into_iter().map(|l| format!("  {l}")));
    }
}
