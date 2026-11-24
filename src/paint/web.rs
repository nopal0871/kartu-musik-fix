//! Implementasi `Painter` di atas Canvas2D.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Path2d};

use super::Painter;
use crate::dom;
use crate::geom::{Align, RRect};
use crate::image::{Img, ImgSrc};

/// Lapisan sementara yang dipakai ulang (marquee) agar tidak membuat kanvas tiap frame.
pub type Layers =
    Rc<RefCell<HashMap<(u32, u32, u8), (HtmlCanvasElement, CanvasRenderingContext2d)>>>;

pub fn new_layers() -> Layers {
    Rc::new(RefCell::new(HashMap::new()))
}

pub struct WebPainter {
    ctx: CanvasRenderingContext2d,
    w: f64,
    h: f64,
    layers: Layers,
    depth: u8,
    filter_ok: bool,
}

impl WebPainter {
    pub fn new(ctx: CanvasRenderingContext2d, w: f64, h: f64, layers: Layers) -> Self {
        let filter_ok = js_sys::Reflect::has(&ctx, &"filter".into()).unwrap_or(false);
        WebPainter {
            ctx,
            w,
            h,
            layers,
            depth: 0,
            filter_ok,
        }
    }

    fn rr_path(&self, r: &RRect) {
        let [tl, tr, br, bl] = r.clamped();
        let c = &self.ctx;
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        c.begin_path();
        c.move_to(x + tl, y);
        c.line_to(x + w - tr, y);
        let _ = c.arc_to(x + w, y, x + w, y + tr, tr);
        c.line_to(x + w, y + h - br);
        let _ = c.arc_to(x + w, y + h, x + w - br, y + h, br);
        c.line_to(x + bl, y + h);
        let _ = c.arc_to(x, y + h, x, y + h - bl, bl);
        c.line_to(x, y + tl);
        let _ = c.arc_to(x, y, x + tl, y, tl);
        c.close_path();
    }

    fn vgrad(&self, y0: f64, y1: f64, stops: &[(f64, &str)]) {
        let g = self.ctx.create_linear_gradient(0.0, y0, 0.0, y1);
        for (o, c) in stops {
            let _ = g.add_color_stop(*o as f32, c);
        }
        self.ctx.set_fill_style_canvas_gradient(&g);
    }

    fn draw_cover(ctx: &CanvasRenderingContext2d, img: &Img, x: f64, y: f64, w: f64, h: f64) {
        let k = (w / img.w).max(h / img.h);
        let (sw, sh) = (w / k, h / k);
        let (sx, sy) = ((img.w - sw) / 2.0, (img.h - sh) / 2.0);
        let _ = match &img.src {
            ImgSrc::Image(i) => ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    i, sx, sy, sw, sh, x, y, w, h,
                ),
            ImgSrc::Canvas(c) => ctx
                .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    c, sx, sy, sw, sh, x, y, w, h,
                ),
        };
    }

    /// Cadangan blur untuk browser tanpa `ctx.filter` (Safari lama): kecilkan lalu perbesar.
    fn blur_fallback(&self, img: &Img, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let f = (r / 2.2).round().max(2.0);
        let cw = (w / f).round().max(4.0) as u32;
        let ch = (h / f).round().max(4.0) as u32;
        let (a, ag) = dom::new_canvas(cw * 4, ch * 4, true);
        Self::draw_cover(&ag, img, 0.0, 0.0, a.width() as f64, a.height() as f64);
        let (b, bg) = dom::new_canvas(cw, ch, true);
        let _ = bg
            .draw_image_with_html_canvas_element_and_dw_and_dh(&a, 0.0, 0.0, cw as f64, ch as f64);
        self.ctx.set_image_smoothing_enabled(true);
        let _ = self
            .ctx
            .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &b, 0.0, 0.0, cw as f64, ch as f64, x, y, w, h,
            );
    }
}

impl Painter for WebPainter {
    fn size(&self) -> (f64, f64) {
        (self.w, self.h)
    }

    fn save(&mut self) {
        self.ctx.save();
    }
    fn restore(&mut self) {
        self.ctx.restore();
    }
    fn clip_rrect(&mut self, r: RRect) {
        self.rr_path(&r);
        self.ctx.clip();
    }

    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: &str) {
        self.ctx.set_fill_style_str(color);
        self.ctx.fill_rect(x, y, w, h);
    }
    fn fill_rect_vgrad(&mut self, x: f64, y: f64, w: f64, h: f64, stops: &[(f64, &str)]) {
        self.vgrad(y, y + h, stops);
        self.ctx.fill_rect(x, y, w, h);
    }
    fn fill_rrect(&mut self, r: RRect, color: &str) {
        self.rr_path(&r);
        self.ctx.set_fill_style_str(color);
        self.ctx.fill();
    }
    fn fill_rrect_vgrad(&mut self, r: RRect, stops: &[(f64, &str)]) {
        self.vgrad(r.y, r.y + r.h, stops);
        self.rr_path(&r);
        self.ctx.fill();
    }
    fn stroke_rrect(&mut self, r: RRect, width: f64, color: &str) {
        self.rr_path(&r);
        self.ctx.set_line_width(width);
        self.ctx.set_stroke_style_str(color);
        self.ctx.stroke();
    }
    fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, color: &str) {
        self.ctx.begin_path();
        let _ = self
            .ctx
            .arc(cx, cy, radius.max(0.0), 0.0, std::f64::consts::TAU);
        self.ctx.set_fill_style_str(color);
        self.ctx.fill();
    }
    fn fill_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, color: &str) {
        self.ctx.begin_path();
        let _ = self.ctx.ellipse(
            cx,
            cy,
            rx.max(0.0),
            ry.max(0.0),
            0.0,
            0.0,
            std::f64::consts::TAU,
        );
        self.ctx.set_fill_style_str(color);
        self.ctx.fill();
    }

    fn image_cover(&mut self, img: &Img, x: f64, y: f64, w: f64, h: f64) {
        Self::draw_cover(&self.ctx, img, x, y, w, h);
    }
    fn image_cover_blur(&mut self, img: &Img, x: f64, y: f64, w: f64, h: f64, blur_px: f64) {
        if blur_px <= 0.5 {
            Self::draw_cover(&self.ctx, img, x, y, w, h);
        } else if self.filter_ok {
            self.ctx.save();
            self.ctx.set_filter(&format!("blur({blur_px:.2}px)"));
            Self::draw_cover(&self.ctx, img, x, y, w, h);
            self.ctx.restore();
        } else {
            self.blur_fallback(img, x, y, w, h, blur_px);
        }
    }

    fn text(&mut self, s: &str, x: f64, y_mid: f64, font: &str, color: &str, align: Align) {
        self.ctx.set_font(font);
        self.ctx.set_fill_style_str(color);
        self.ctx.set_text_baseline("middle");
        self.ctx.set_text_align(match align {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        });
        let _ = self.ctx.fill_text(s, x, y_mid);
    }
    fn measure(&mut self, s: &str, font: &str) -> f64 {
        self.ctx.set_font(font);
        self.ctx.measure_text(s).map(|m| m.width()).unwrap_or(0.0)
    }

    fn svg_path(
        &mut self,
        d: &str,
        tx: f64,
        ty: f64,
        scale: f64,
        fill: Option<&str>,
        stroke: Option<(&str, f64)>,
    ) {
        let Ok(path) = Path2d::new_with_path_string(d) else {
            return;
        };
        let c = &self.ctx;
        c.save();
        let _ = c.translate(tx, ty);
        let _ = c.scale(scale, scale);
        c.set_line_join("round");
        c.set_line_cap("round");
        if let Some(col) = fill {
            c.set_fill_style_str(col);
            c.fill_with_path_2d(&path);
        }
        if let Some((col, w)) = stroke {
            c.set_stroke_style_str(col);
            c.set_line_width(w);
            c.stroke_with_path(&path);
        }
        c.restore();
    }

    fn layer(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fade_l: f64,
        fade_r: f64,
        f: &mut dyn FnMut(&mut dyn Painter),
    ) {
        let (lw, lh) = (w.ceil().max(1.0) as u32, h.ceil().max(1.0) as u32);
        let key = (lw, lh, self.depth);
        let (cv, lctx) = {
            let mut map = self.layers.borrow_mut();
            let e = map
                .entry(key)
                .or_insert_with(|| dom::new_canvas(lw, lh, true));
            (e.0.clone(), e.1.clone())
        };
        lctx.set_global_composite_operation("source-over").ok();
        lctx.clear_rect(0.0, 0.0, lw as f64, lh as f64);
        {
            let mut lp = WebPainter {
                ctx: lctx.clone(),
                w: lw as f64,
                h: lh as f64,
                layers: self.layers.clone(),
                depth: self.depth + 1,
                filter_ok: self.filter_ok,
            };
            f(&mut lp);
        }
        if fade_l > 0.0 || fade_r > 0.0 {
            let g = lctx.create_linear_gradient(0.0, 0.0, lw as f64, 0.0);
            let fw = lw as f64;
            if fade_l > 0.0 {
                let _ = g.add_color_stop(0.0, "rgba(0,0,0,0)");
                let _ = g.add_color_stop((fade_l / fw).min(1.0) as f32, "#000");
            } else {
                let _ = g.add_color_stop(0.0, "#000");
            }
            let _ = g.add_color_stop((1.0 - fade_r / fw).max(0.0) as f32, "#000");
            let _ = g.add_color_stop(1.0, "rgba(0,0,0,0)");
            lctx.set_global_composite_operation("destination-in").ok();
            lctx.set_fill_style_canvas_gradient(&g);
            lctx.fill_rect(0.0, 0.0, fw, lh as f64);
            lctx.set_global_composite_operation("source-over").ok();
        }
        let _ = self.ctx.draw_image_with_html_canvas_element(&cv, x, y);
    }
}
