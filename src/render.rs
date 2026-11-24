//! Renderer: satu jalur gambar untuk pratinjau, katalog, player, dan ekspor video.
//! Lapisan statis template di-cache pada kanvas luar-layar dan hanya dibuat ulang saat
//! `Scene::rev`, ukuran, orientasi, atau template berubah.

use std::cell::RefCell;

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dom;
use crate::geom::Orient;
use crate::paint::web::{new_layers, Layers, WebPainter};
use crate::paint::Painter;
use crate::state::{Frame, Scene};
use crate::templates::Template;

struct Cached {
    key: (String, u64, u32, u32, Orient),
    cv: HtmlCanvasElement,
}

pub struct Renderer {
    layers: Layers,
    cache: RefCell<Option<Cached>>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {
            layers: new_layers(),
            cache: RefCell::new(None),
        }
    }

    fn static_layer(
        &self,
        tpl: &dyn Template,
        s: &Scene,
        o: Orient,
        w: u32,
        h: u32,
    ) -> HtmlCanvasElement {
        let key = (tpl.meta().id.to_string(), s.rev, w, h, o);
        if let Some(c) = &*self.cache.borrow() {
            if c.key == key {
                return c.cv.clone();
            }
        }
        // Gunakan ulang kanvas bila ukurannya sama (hindari alokasi besar saat slider digeser).
        let (cv, ctx) = match self.cache.borrow_mut().take() {
            Some(old) if old.key.2 == w && old.key.3 == h => {
                let ctx = dom::context_2d(&old.cv, true);
                ctx.clear_rect(0.0, 0.0, w as f64, h as f64);
                (old.cv, ctx)
            }
            _ => dom::new_canvas(w, h, true),
        };
        let mut p = WebPainter::new(ctx, w as f64, h as f64, self.layers.clone());
        tpl.draw_static(&mut p, s, o);
        *self.cache.borrow_mut() = Some(Cached {
            key,
            cv: cv.clone(),
        });
        cv
    }

    /// Gambar satu frame penuh ke `ctx` berukuran w x h.
    pub fn draw(
        &self,
        ctx: &CanvasRenderingContext2d,
        w: u32,
        h: u32,
        tpl: &dyn Template,
        s: &Scene,
        f: &Frame,
        o: Orient,
    ) {
        let st = self.static_layer(tpl, s, o, w, h);
        let _ = ctx.draw_image_with_html_canvas_element(&st, 0.0, 0.0);
        let mut p = WebPainter::new(ctx.clone(), w as f64, h as f64, self.layers.clone());
        tpl.draw_dynamic(&mut p, s, f, o);
        if f.fade > 0.0 {
            p.save();
            ctx.set_global_alpha(f.fade.min(1.0));
            p.fill_rect(0.0, 0.0, w as f64, h as f64, f.fade_color);
            ctx.set_global_alpha(1.0);
            p.restore();
        }
    }
}
