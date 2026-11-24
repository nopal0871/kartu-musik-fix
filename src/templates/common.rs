//! Bantuan yang dipakai bersama oleh template.

use super::{BgPolicy, TemplateMeta};
use crate::geom::{Orient, RRect, Rect};
use crate::paint::Painter;
use crate::state::{BgMode, Scene};

pub const FONT: &str =
    "-apple-system, BlinkMacSystemFont, 'Inter', 'Segoe UI', system-ui, sans-serif";

/// String font Canvas: `font(600, 3.3 * u)`.
pub fn font(weight: u32, px: f64) -> String {
    format!("{weight} {px:.3}px {FONT}")
}

/// Lukisan latar sesuai kebijakan template: gambar blur, gradien warna, atau latar tetap,
/// lalu lapisan gelap. `extra_blur` (satuan U) dipakai kartu kaca iOS.
pub fn draw_background(
    p: &mut dyn Painter,
    s: &Scene,
    meta: &TemplateMeta,
    o: Orient,
    extra_blur: f64,
) {
    let (w, h) = p.size();
    let u = p.unit();
    p.fill_rect(0.0, 0.0, w, h, "#1c1c1e");
    let (img, blur, dim, color) = match &meta.bg {
        BgPolicy::Fixed {
            portrait,
            landscape,
            blur,
            dim,
            ..
        } => {
            let key = if o.is_landscape() {
                landscape
            } else {
                portrait
            };
            (s.asset(key), *blur, *dim, false)
        }
        BgPolicy::Free => match s.bg_mode {
            BgMode::Color => (None, s.blur, s.dim, true),
            BgMode::Image => (
                s.bg_img.clone().or_else(|| s.art.clone()),
                s.blur,
                s.dim,
                false,
            ),
            BgMode::Album => (s.art.clone(), s.blur, s.dim, false),
        },
    };
    if color {
        p.fill_rect_vgrad(0.0, 0.0, w, h, &[(0.0, &s.c1), (1.0, &s.c2)]);
    } else if let Some(img) = img {
        let (bx, by, bw, bh) = (-0.15 * w, -0.15 * h, 1.3 * w, 1.3 * h);
        let br = blur * 0.16 * u + extra_blur * u;
        if br <= 0.5 {
            p.image_cover(&img, bx, by, bw, bh);
        } else {
            p.image_cover_blur(&img, bx, by, bw, bh, br);
        }
    }
    p.fill_rect(0.0, 0.0, w, h, &format!("rgba(0,0,0,{})", dim / 100.0));
}

/// Bilah progres/volume sederhana: trek + isian membulat.
pub fn bar(
    p: &mut dyn Painter,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    frac: f64,
    fill: &str,
    track: &str,
) {
    p.fill_rrect(RRect::new(x, y - h / 2.0, w, h, h / 2.0), track);
    if frac > 0.0 {
        let fw = (w * frac).max(h);
        p.fill_rrect(RRect::new(x, y - h / 2.0, fw, h, h / 2.0), fill);
    }
}

/// Area sentuh berpusat di (cx, cy).
pub fn hit_box(cx: f64, cy: f64, w: f64, h: f64) -> Rect {
    Rect::centered(cx, cy, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::mock::Mock;

    #[test]
    fn bar_draws_track_and_fill_only_when_positive() {
        let mut m = Mock::new(100.0, 100.0);
        bar(&mut m, 0.0, 5.0, 50.0, 2.0, 0.0, "#fff", "#000");
        assert_eq!(m.count("fill_rrect"), 1);
        bar(&mut m, 0.0, 5.0, 50.0, 2.0, 0.5, "#fff", "#000");
        assert_eq!(m.count("fill_rrect"), 3);
    }

    #[test]
    fn font_string_has_weight_and_stack() {
        let f = font(600, 12.0);
        assert!(f.starts_with("600 12.000px ") && f.contains("Inter"));
    }
}
