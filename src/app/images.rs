//! Foto album, gambar latar, dan crop manual.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    File, HtmlButtonElement, HtmlImageElement, HtmlInputElement, PointerEvent, WheelEvent,
};

use super::media::wire_drop;
use super::{paint_bg_ui, App, CropSession, Key, Orig, Shared};
use crate::dom::{self, by_id, html, listen_id, toast};
use crate::geom::Orient;
use crate::image::{apply_crop, default_art, Crop, Img};
use crate::state::BgMode;

const DEFAULT_NAME: &str = "contoh";

/// (rasio lebar/tinggi, ukuran keluaran, judul dialog)
fn target(key: Key, o: Orient) -> (f64, (u32, u32), String) {
    match (key, o) {
        (Key::Art, _) => (1.0, (1200, 1200), "Crop foto album (1:1)".into()),
        (Key::Bg, Orient::Portrait) => {
            (9.0 / 16.0, (1080, 1920), "Crop gambar latar (9:16)".into())
        }
        (Key::Bg, Orient::Landscape) => {
            (16.0 / 9.0, (1920, 1080), "Crop gambar latar (16:9)".into())
        }
    }
}

/// Foto album contoh bawaan.
pub fn load_defaults(app: &Shared) {
    let art = Rc::new(default_art());
    let url = match &art.src {
        crate::image::ImgSrc::Canvas(c) => c
            .to_data_url_with_type_and_encoder_options("image/jpeg", &0.9.into())
            .unwrap_or_default(),
        crate::image::ImgSrc::Image(i) => i.src(),
    };
    {
        let mut a = app.borrow_mut();
        a.origs[Key::Art.idx()] = Some(Orig {
            img: art.clone(),
            url,
            name: DEFAULT_NAME.into(),
            crop: None,
        });
        a.scene.art = Some(art.clone());
        a.scene.art_custom = false;
        a.scene.touch();
    }
    dom::set_bg_image(&html("artThumb"), &art.thumb_data_url(128));
    by_id::<HtmlButtonElement>("artCrop").set_disabled(false);
}

impl App {
    /// Terapkan crop dari gambar asli `key` ke keluaran bawaan targetnya.
    pub fn apply_crop_to(&mut self, key: Key, crop: Crop) {
        let (aspect, out, _) = target(key, self.orient);
        let Some(orig) = self.origs[key.idx()].as_mut() else {
            return;
        };
        let (img, used) = apply_crop(&orig.img, crop, aspect, out);
        orig.crop = Some(used);
        let custom = orig.name != DEFAULT_NAME;
        let img = Rc::new(img);
        let thumb = img.thumb_data_url(128);
        match key {
            Key::Art => {
                self.scene.art = Some(img);
                self.scene.art_custom = custom;
                dom::set_bg_image(&html("artThumb"), &thumb);
            }
            Key::Bg => {
                self.scene.bg_img = Some(img);
                self.scene.bg_mode = BgMode::Image;
                dom::set_bg_image(&html("bgThumb"), &thumb);
                paint_bg_ui(self);
            }
        }
        by_id::<HtmlButtonElement>("artCrop").set_disabled(self.origs[0].is_none());
        by_id::<HtmlButtonElement>("bgCrop").set_disabled(self.origs[1].is_none());
        self.scene.touch();
    }
}

/// Setelah orientasi berubah, gambar latar sendiri di-crop ulang ke rasio baru (dipusatkan).
pub fn recrop_bg(app: &Shared) {
    let mut a = app.borrow_mut();
    let Some(o) = a.origs[Key::Bg.idx()].as_ref() else {
        return;
    };
    let mode = a.scene.bg_mode;
    let c = Crop::centered(o.img.w, o.img.h);
    a.apply_crop_to(Key::Bg, c);
    a.scene.bg_mode = mode;
    paint_bg_ui(&a);
}

fn load_art(app: &Shared, f: File) {
    load_image_file(app, Key::Art, f);
}
fn load_bg(app: &Shared, f: File) {
    load_image_file(app, Key::Bg, f);
}

fn load_image_file(app: &Shared, key: Key, f: File) {
    let app = app.clone();
    spawn_local(async move {
        let url = web_sys::Url::create_object_url_with_blob(&f).unwrap_or_default();
        let img = match Img::load(&url).await {
            Ok(i) => i,
            Err(_) => {
                let _ = web_sys::Url::revoke_object_url(&url);
                toast("Gambar tidak bisa dibuka. Gunakan JPG, PNG, atau WebP.");
                return;
            }
        };
        let (iw, ih) = (img.w, img.h);
        let mismatch;
        {
            let mut a = app.borrow_mut();
            if let Some(old) = a.origs[key.idx()].take() {
                if old.url.starts_with("blob:") {
                    let _ = web_sys::Url::revoke_object_url(&old.url);
                }
            }
            a.origs[key.idx()] = Some(Orig {
                img: Rc::new(img),
                url,
                name: f.name(),
                crop: None,
            });
            a.apply_crop_to(key, Crop::centered(iw, ih));
            dom::set_text(
                if key == Key::Art { "artName" } else { "bgName" },
                &f.name(),
            );
            let want = target(key, a.orient).0;
            mismatch = ((iw / ih) - want).abs() / want > 0.02;
        }
        if mismatch {
            open_crop(&app, key);
            toast("Rasio foto berbeda. Atur crop sesuai keinginan.");
        }
    });
}

/* ---------- Dialog crop ---------- */

fn crop_layout(a: &mut App) {
    let Some(cr) = a.crop.as_mut() else { return };
    let Some(o) = a.origs[cr.key.idx()].as_ref() else {
        return;
    };
    let (iw, ih) = (o.img.w, o.img.h);
    let s = (cr.vw / iw).max(cr.vh / ih) * cr.zoom;
    cr.cx = cr.cx.clamp(
        cr.vw / (2.0 * s),
        (iw - cr.vw / (2.0 * s)).max(cr.vw / (2.0 * s)),
    );
    cr.cy = cr.cy.clamp(
        cr.vh / (2.0 * s),
        (ih - cr.vh / (2.0 * s)).max(cr.vh / (2.0 * s)),
    );
    cr.s = s;
    let st = html("cropImg").style();
    let _ = st.set_property("width", &format!("{}px", iw * s));
    let _ = st.set_property("height", &format!("{}px", ih * s));
    let _ = st.set_property("left", &format!("{}px", cr.vw / 2.0 - cr.cx * s));
    let _ = st.set_property("top", &format!("{}px", cr.vh / 2.0 - cr.cy * s));
}

pub fn open_crop(app: &Shared, key: Key) {
    let mut a = app.borrow_mut();
    let (aspect, _, title) = target(key, a.orient);
    let Some(o) = a.origs[key.idx()].as_ref() else {
        return;
    };
    let win = dom::window();
    let (iw_, ih_) = (
        win.inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0),
        win.inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(700.0),
    );
    let max_w = 360.0_f64.min(iw_ - 64.0);
    let max_h = 200.0_f64.max(ih_ * 0.5);
    let w = max_w.min(max_h * aspect);
    let h = w / aspect;
    let view = html("cropView").style();
    let _ = view.set_property("width", &format!("{w}px"));
    let _ = view.set_property("height", &format!("{h}px"));
    by_id::<HtmlImageElement>("cropImg").set_src(&o.url);
    let c = o.crop.unwrap_or_else(|| Crop::centered(o.img.w, o.img.h));
    dom::set_text("cropTitle", &title);
    by_id::<HtmlInputElement>("cropZoom").set_value(&c.zoom.to_string());
    a.crop = Some(CropSession {
        key,
        zoom: c.zoom,
        cx: c.cx,
        cy: c.cy,
        vw: w,
        vh: h,
        s: 1.0,
    });
    dom::set_hidden("cropSheet", false);
    crop_layout(&mut a);
}

pub fn close_crop(app: &Shared) {
    app.borrow_mut().crop = None;
    dom::set_hidden("cropSheet", true);
}

pub fn wire(app: &Shared) {
    wire_drop(
        app,
        "artDrop",
        "artFile",
        "image",
        "File itu bukan gambar.",
        load_art,
    );
    wire_drop(
        app,
        "bgDrop",
        "bgFile",
        "image",
        "File itu bukan gambar.",
        load_bg,
    );
    by_id::<HtmlButtonElement>("artCrop").set_disabled(true);

    let a = app.clone();
    listen_id("artCrop", "click", move |_| open_crop(&a, Key::Art));
    let a = app.clone();
    listen_id("bgCrop", "click", move |_| open_crop(&a, Key::Bg));
    let a = app.clone();
    listen_id("cropCancel", "click", move |_| close_crop(&a));
    let a = app.clone();
    listen_id("cropOk", "click", move |_| {
        {
            let mut s = a.borrow_mut();
            if let Some(cr) = s.crop.as_ref() {
                let (key, c) = (
                    cr.key,
                    Crop {
                        zoom: cr.zoom,
                        cx: cr.cx,
                        cy: cr.cy,
                    },
                );
                s.apply_crop_to(key, c);
            }
        }
        close_crop(&a);
    });

    let a = app.clone();
    listen_id("cropZoom", "input", move |_| {
        let mut s = a.borrow_mut();
        let z: f64 = by_id::<HtmlInputElement>("cropZoom")
            .value()
            .parse()
            .unwrap_or(1.0);
        if let Some(c) = s.crop.as_mut() {
            c.zoom = z;
        }
        crop_layout(&mut s);
    });

    let a = app.clone();
    listen_id("cropView", "wheel", move |e| {
        e.prevent_default();
        let Ok(we) = e.dyn_into::<WheelEvent>() else {
            return;
        };
        let mut s = a.borrow_mut();
        if let Some(c) = s.crop.as_mut() {
            c.zoom = (c.zoom * if we.delta_y() < 0.0 { 1.06 } else { 1.0 / 1.06 }).clamp(1.0, 4.0);
            by_id::<HtmlInputElement>("cropZoom").set_value(&c.zoom.to_string());
        }
        crop_layout(&mut s);
    });

    // Geser foto dengan pointer
    let drag: Rc<Cell<Option<(f64, f64, f64, f64)>>> = Rc::new(Cell::new(None));
    let (a, d) = (app.clone(), drag.clone());
    listen_id("cropView", "pointerdown", move |e| {
        let Ok(pe) = e.dyn_into::<PointerEvent>() else {
            return;
        };
        if let Some(c) = a.borrow().crop.as_ref() {
            d.set(Some((
                pe.client_x() as f64,
                pe.client_y() as f64,
                c.cx,
                c.cy,
            )));
        }
        let v = html("cropView");
        let _ = v.class_list().add_1("drag");
        let _ = v.set_pointer_capture(pe.pointer_id());
        pe.prevent_default();
    });
    let (a, d) = (app.clone(), drag.clone());
    listen_id("cropView", "pointermove", move |e| {
        let Ok(pe) = e.dyn_into::<PointerEvent>() else {
            return;
        };
        let Some((x0, y0, cx0, cy0)) = d.get() else {
            return;
        };
        let mut s = a.borrow_mut();
        if let Some(c) = s.crop.as_mut() {
            let k = c.s.max(1e-6);
            c.cx = cx0 - (pe.client_x() as f64 - x0) / k;
            c.cy = cy0 - (pe.client_y() as f64 - y0) / k;
        }
        crop_layout(&mut s);
    });
    for ev in ["pointerup", "pointercancel"] {
        let d = drag.clone();
        listen_id("cropView", ev, move |_| {
            d.set(None);
            let _ = html("cropView").class_list().remove_1("drag");
        });
    }
}
