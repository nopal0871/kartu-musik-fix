//! Lapisan gambar abstrak. Template hanya berbicara dengan `Painter`, sehingga
//! logika tampilan bisa diuji di luar browser (lihat `mock`) dan implementasi
//! Canvas2D (`web`) bisa diganti tanpa menyentuh template.

use crate::geom::{Align, RRect};
use crate::image::Img;

pub mod web;

#[cfg(test)]
pub mod mock;

pub trait Painter {
    /// Ukuran kanvas dalam piksel.
    fn size(&self) -> (f64, f64);
    /// Piksel per satuan U (1% lebar).
    fn unit(&self) -> f64 {
        self.size().0 / 100.0
    }

    fn save(&mut self);
    fn restore(&mut self);
    fn clip_rrect(&mut self, r: RRect);

    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: &str);
    /// Gradien vertikal sepanjang tinggi persegi panjang.
    fn fill_rect_vgrad(&mut self, x: f64, y: f64, w: f64, h: f64, stops: &[(f64, &str)]);
    fn fill_rrect(&mut self, r: RRect, color: &str);
    fn fill_rrect_vgrad(&mut self, r: RRect, stops: &[(f64, &str)]);
    fn stroke_rrect(&mut self, r: RRect, width: f64, color: &str);
    fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, color: &str);
    fn fill_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, color: &str);

    /// Gambar mengisi penuh persegi panjang (object-fit: cover, dipusatkan).
    fn image_cover(&mut self, img: &Img, x: f64, y: f64, w: f64, h: f64);
    /// Sama seperti `image_cover` tetapi diburamkan `blur_px` piksel.
    fn image_cover_blur(&mut self, img: &Img, x: f64, y: f64, w: f64, h: f64, blur_px: f64);

    fn text(&mut self, s: &str, x: f64, y_mid: f64, font: &str, color: &str, align: Align);
    fn measure(&mut self, s: &str, font: &str) -> f64;

    /// Gambar path SVG. (tx, ty) = pergeseran, `scale` = skala, `stroke` = (warna, lebar dalam satuan path).
    #[allow(clippy::too_many_arguments)]
    fn svg_path(
        &mut self,
        d: &str,
        tx: f64,
        ty: f64,
        scale: f64,
        fill: Option<&str>,
        stroke: Option<(&str, f64)>,
    );

    /// Gambar ke lapisan sementara berukuran w x h lalu tempel di (x, y).
    /// Bila `fade_l`/`fade_r` > 0 tepi kiri/kanan dipudarkan (masker marquee).
    #[allow(clippy::too_many_arguments)]
    fn layer(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fade_l: f64,
        fade_r: f64,
        f: &mut dyn FnMut(&mut dyn Painter),
    );
}
