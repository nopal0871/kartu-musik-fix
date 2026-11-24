//! Gambar yang bisa digambar ke kanvas: berasal dari berkas (<img>) atau hasil crop (<canvas>).

use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlCanvasElement, HtmlImageElement};

use crate::dom;

pub enum ImgSrc {
    Image(HtmlImageElement),
    Canvas(HtmlCanvasElement),
}

pub struct Img {
    pub src: ImgSrc,
    pub w: f64,
    pub h: f64,
}

impl Img {
    pub fn from_canvas(cv: HtmlCanvasElement) -> Img {
        let (w, h) = (cv.width() as f64, cv.height() as f64);
        Img {
            src: ImgSrc::Canvas(cv),
            w,
            h,
        }
    }

    /// Muat gambar dari URL (blob: atau relatif) dan tunggu sampai ter-decode.
    pub async fn load(url: &str) -> Result<Img, String> {
        let el = HtmlImageElement::new().map_err(|_| "gagal membuat <img>".to_string())?;
        el.set_src(url);
        JsFuture::from(el.decode())
            .await
            .map_err(|_| format!("gambar tidak bisa dibuka: {url}"))?;
        let (w, h) = (el.natural_width() as f64, el.natural_height() as f64);
        if w < 1.0 || h < 1.0 {
            return Err("gambar kosong".into());
        }
        Ok(Img {
            src: ImgSrc::Image(el),
            w,
            h,
        })
    }

    /// Salinan kecil sebagai data URL untuk thumbnail CSS.
    pub fn thumb_data_url(&self, size: u32) -> String {
        let (cv, ctx) = dom::new_canvas(size, size, true);
        let (iw, ih) = (self.w, self.h);
        let k = (size as f64 / iw).max(size as f64 / ih);
        let (sw, sh) = (size as f64 / k, size as f64 / k);
        let s = size as f64;
        let _ = match &self.src {
            ImgSrc::Image(i) => ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    i,
                    (iw - sw) / 2.0,
                    (ih - sh) / 2.0,
                    sw,
                    sh,
                    0.0,
                    0.0,
                    s,
                    s,
                ),
            ImgSrc::Canvas(c) => ctx
                .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    c,
                    (iw - sw) / 2.0,
                    (ih - sh) / 2.0,
                    sw,
                    sh,
                    0.0,
                    0.0,
                    s,
                    s,
                ),
        };
        cv.to_data_url_with_type("image/jpeg").unwrap_or_default()
    }
}

/// Gambar contoh bawaan (pengganti `makeDefaultArt` versi JS).
pub fn default_art() -> Img {
    let n = 900.0_f64;
    let (cv, g) = dom::new_canvas(900, 900, true);
    let lin = |y0: f64, y1: f64, a: &str, b: &str| {
        let gr = g.create_linear_gradient(0.0, y0, 0.0, y1);
        let _ = gr.add_color_stop(0.0, a);
        let _ = gr.add_color_stop(1.0, b);
        gr
    };
    g.set_fill_style_canvas_gradient(&lin(0.0, n * 0.6, "#b9bbbf", "#eeeeef"));
    g.fill_rect(0.0, 0.0, n, n * 0.6);
    g.set_fill_style_str("#c9cacc");
    g.fill_rect(0.0, n * 0.6, n, n * 0.045);
    g.set_fill_style_canvas_gradient(&lin(n * 0.645, n, "#e4e4e5", "#8c8d90"));
    g.fill_rect(0.0, n * 0.645, n, n * 0.355);
    if let Ok(rg) = g.create_radial_gradient(n * 0.7, n * 0.3, 0.0, n * 0.7, n * 0.3, n * 0.24) {
        let _ = rg.add_color_stop(0.0, "rgba(255,255,255,.95)");
        let _ = rg.add_color_stop(1.0, "rgba(255,255,255,0)");
        g.set_fill_style_canvas_gradient(&rg);
        g.fill_rect(0.0, 0.0, n, n);
    }
    g.set_fill_style_str("rgba(0,0,0,.32)");
    g.begin_path();
    g.move_to(n * 0.43, n * 0.83);
    g.line_to(n * 0.5, n * 0.83);
    g.line_to(n * 0.9, n * 0.87);
    g.line_to(n * 0.62, n * 0.9);
    g.close_path();
    g.fill();
    g.set_fill_style_str("#161617");
    g.fill_rect(n * 0.43, n * 0.33, n * 0.07, n * 0.5);
    // Butiran film halus
    for _ in 0..26000 {
        let dark = js_sys::Math::random() < 0.5;
        g.set_fill_style_str(if dark {
            "rgba(0,0,0,.07)"
        } else {
            "rgba(255,255,255,.08)"
        });
        g.fill_rect(
            js_sys::Math::random() * n,
            js_sys::Math::random() * n,
            1.2,
            1.2,
        );
    }
    Img::from_canvas(cv)
}

/// Pengaturan crop: zoom >= 1 dan pusat (cx, cy) dalam piksel gambar asli.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    pub zoom: f64,
    pub cx: f64,
    pub cy: f64,
}

impl Crop {
    pub fn centered(iw: f64, ih: f64) -> Crop {
        Crop {
            zoom: 1.0,
            cx: iw / 2.0,
            cy: ih / 2.0,
        }
    }

    /// Skala gambar → viewport (vw x vh) untuk zoom saat ini.
    pub fn scale(&self, iw: f64, ih: f64, vw: f64, vh: f64) -> f64 {
        (vw / iw).max(vh / ih) * self.zoom
    }

    /// Jepit pusat agar jendela crop tidak keluar dari gambar.
    pub fn clamped(&self, iw: f64, ih: f64, vw: f64, vh: f64) -> Crop {
        let s = self.scale(iw, ih, vw, vh);
        let (hw, hh) = (vw / (2.0 * s), vh / (2.0 * s));
        Crop {
            zoom: self.zoom,
            cx: self.cx.clamp(hw, (iw - hw).max(hw)),
            cy: self.cy.clamp(hh, (ih - hh).max(hh)),
        }
    }
}

/// Terapkan crop ke kanvas keluaran `out_w` x `out_h` dengan rasio `aspect` (lebar/tinggi).
pub fn apply_crop(src: &Img, crop: Crop, aspect: f64, out: (u32, u32)) -> (Img, Crop) {
    let (vw, vh) = (100.0 * aspect, 100.0);
    let c = crop.clamped(src.w, src.h, vw, vh);
    let s = c.scale(src.w, src.h, vw, vh);
    let (sw, sh) = (vw / s, vh / s);
    let (cv, ctx) = dom::new_canvas(out.0, out.1, true);
    let (sx, sy) = (c.cx - sw / 2.0, c.cy - sh / 2.0);
    let (dw, dh) = (out.0 as f64, out.1 as f64);
    let _ = match &src.src {
        ImgSrc::Image(i) => ctx
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                i, sx, sy, sw, sh, 0.0, 0.0, dw, dh,
            ),
        ImgSrc::Canvas(k) => ctx
            .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                k, sx, sy, sw, sh, 0.0, 0.0, dw, dh,
            ),
    };
    (Img::from_canvas(cv), c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_is_clamped_inside_image() {
        let c = Crop {
            zoom: 1.0,
            cx: -500.0,
            cy: 99999.0,
        };
        let r = c.clamped(2000.0, 1000.0, 100.0, 100.0);
        // skala = max(100/2000, 100/1000) = 0.1 → setengah jendela = 500 px
        assert_eq!(r.cx, 500.0);
        assert_eq!(r.cy, 500.0);
    }

    #[test]
    fn zoom_shrinks_window() {
        let c = Crop {
            zoom: 2.0,
            cx: 0.0,
            cy: 0.0,
        };
        let r = c.clamped(1000.0, 1000.0, 100.0, 100.0);
        assert_eq!((r.cx, r.cy), (250.0, 250.0));
    }
}
