//! Aplikasi: menghubungkan DOM, audio, gambar, template, dan ekspor.
//! Logika tampilan kartu sendiri ada di `templates/`; di sini hanya perekat.

mod exporting;
mod images;
mod media;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, File, HtmlAudioElement, HtmlCanvasElement, HtmlElement,
    HtmlInputElement, HtmlSelectElement, PointerEvent,
};

use crate::bridge::{self, AudioGraph};
use crate::dom::{self, by_id, html, listen_id, query_all};
use crate::geom::{fmt_time, Orient};
use crate::image::{Crop, Img};
use crate::render::Renderer;
use crate::spectrum::{band_targets, Spectrum, IDLE};
use crate::state::{BgMode, Frame, Scene};
use crate::templates::{self, Action, Hit, Template};

pub type Shared = Rc<RefCell<App>>;

pub struct Demo {
    pub cur: f64,
    pub dur: f64,
    pub playing: bool,
}

pub struct Orig {
    pub img: Rc<Img>,
    pub url: String,
    pub name: String,
    pub crop: Option<Crop>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Art,
    Bg,
}

impl Key {
    pub fn idx(self) -> usize {
        match self {
            Key::Art => 0,
            Key::Bg => 1,
        }
    }
}

pub struct CropSession {
    pub key: Key,
    pub zoom: f64,
    pub cx: f64,
    pub cy: f64,
    pub vw: f64,
    pub vh: f64,
    pub s: f64,
}

struct Thumb {
    id: &'static str,
    cv: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    r: Renderer,
    btn: HtmlElement,
}

pub struct App {
    pub reg: Vec<Rc<dyn Template>>,
    pub tpl: Rc<dyn Template>,
    pub orient: Orient,
    pub scene: Scene,
    pub title_auto: bool,

    pub audio: HtmlAudioElement,
    pub has_audio: bool,
    pub audio_file: Option<File>,
    pub blob_url: String,
    pub graph: Option<Rc<AudioGraph>>,
    pub demo: Demo,
    pub spec: Spectrum,
    spec_buf: Uint8Array,

    pub view_player: bool,
    /// Bendera batal saat ekspor berjalan.
    pub rec: Option<Rc<Cell<bool>>>,
    pub origs: [Option<Orig>; 2],
    pub crop: Option<CropSession>,
    drag: Option<Hit>,
    t0: f64,

    r_prev: Renderer,
    r_play: Renderer,
    prev_ctx: CanvasRenderingContext2d,
    play_ctx: CanvasRenderingContext2d,
    thumbs: Vec<Thumb>,
    thumbs_key: (u64, Orient),
    last_clock: String,
    last_play: Option<bool>,
    pub sheet_url: String,
    pub sheet_blob: Option<web_sys::Blob>,
}

const PREVIEW_SHORT: u32 = 540;
const PLAYER_SHORT: u32 = 720;
const THUMB_SHORT: u32 = 200;

impl App {
    fn new() -> App {
        let reg = templates::registry();
        let tpl = reg[0].clone();
        let audio = HtmlAudioElement::new().expect("Audio");
        audio.set_preload("metadata");
        let scene = Scene::new();
        audio.set_volume(scene.vol);
        let prev_cv: HtmlCanvasElement = by_id("cvPreview");
        let play_cv: HtmlCanvasElement = by_id("cvPlayer");
        App {
            reg,
            tpl,
            orient: Orient::Portrait,
            scene,
            title_auto: true,
            audio,
            has_audio: false,
            audio_file: None,
            blob_url: String::new(),
            graph: None,
            demo: Demo {
                cur: 1.0,
                dur: 67.0,
                playing: true,
            },
            spec: Spectrum::default(),
            spec_buf: Uint8Array::new_with_length(256),
            view_player: false,
            rec: None,
            origs: [None, None],
            crop: None,
            drag: None,
            t0: dom::now_ms(),
            r_prev: Renderer::new(),
            r_play: Renderer::new(),
            prev_ctx: dom::context_2d(&prev_cv, false),
            play_ctx: dom::context_2d(&play_cv, false),
            thumbs: Vec::new(),
            thumbs_key: (0, Orient::Portrait),
            last_clock: String::new(),
            last_play: None,
            sheet_url: String::new(),
            sheet_blob: None,
        }
    }

    fn frame(&self, t: f64) -> Frame {
        let d = self.dur();
        let c = if d > 0.0 {
            self.cur().min(d)
        } else {
            self.cur()
        };
        Frame {
            t,
            cur: c,
            dur: d,
            playing: self.shown_playing(),
            spec: self.spec.v,
            fade: 0.0,
            fade_color: "#000",
        }
    }

    /// Spektrum live: dari AnalyserNode saat lagu diputar, datar saat tidak.
    fn step_spectrum(&mut self, now: f64) {
        let tgt = if self.is_playing_real() {
            match &self.graph {
                Some(g) => {
                    g.bytes(&self.spec_buf);
                    let mut b = [0u8; 256];
                    self.spec_buf.copy_to(&mut b);
                    band_targets(&b)
                }
                None => {
                    let mut t = [0.0; 5];
                    for (j, v) in t.iter_mut().enumerate() {
                        *v = 0.05
                            + 0.2
                                * (now / (150.0 + j as f64 * 37.0) + j as f64 * 1.9)
                                    .sin()
                                    .max(0.0);
                    }
                    t
                }
            }
        } else {
            IDLE
        };
        self.spec.step(&tgt, 60.0);
    }

    fn draw_visible(&mut self, now: f64) {
        self.step_spectrum(now);
        let t = (now - self.t0) / 1000.0;
        let f = self.frame(t);
        self.update_clock_ui();
        let tpl = self.tpl.clone();
        if self.view_player {
            let (w, h) = self.orient.dims(PLAYER_SHORT);
            self.r_play
                .draw(&self.play_ctx, w, h, &*tpl, &self.scene, &f, self.orient);
        } else {
            let (w, h) = self.orient.dims(PREVIEW_SHORT);
            self.r_prev
                .draw(&self.prev_ctx, w, h, &*tpl, &self.scene, &f, self.orient);
            self.draw_thumbs();
        }
    }

    fn draw_thumbs(&mut self) {
        let key = (self.scene.rev, self.orient);
        if key == self.thumbs_key {
            return;
        }
        self.thumbs_key = key;
        let (w, h) = self.orient.dims(THUMB_SHORT);
        let f = Frame::demo();
        for th in &self.thumbs {
            if let Some(t) = templates::find(&self.reg, th.id) {
                th.r.draw(&th.ctx, w, h, &*t, &self.scene, &f, self.orient);
            }
        }
    }

    /// Daftar area sentuh template aktif pada kartu.
    fn hit_at(&self, xu: f64, yu: f64) -> Option<Hit> {
        self.tpl
            .hits(&self.scene, self.orient)
            .into_iter()
            .find(|h| h.rect.contains(xu, yu))
    }
}

/* ------------------------------------------------------------------ */

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let app: Shared = Rc::new(RefCell::new(App::new()));

    // Font Inter harus siap sebelum teks pertama digambar; setelah siap, gambar ulang lapisan statis.
    {
        let a = app.clone();
        let cb = Closure::<dyn FnMut()>::new(move || {
            if let Ok(mut s) = a.try_borrow_mut() {
                s.scene.touch();
            }
        });
        bridge::on_fonts_change(&cb);
        cb.forget();
        let _ = bridge::preload_fonts();
    }

    build_catalog(&app);
    wire_dashboard(&app);
    media::wire(&app);
    images::wire(&app);
    exporting::wire(&app);
    apply_template(&app);
    images::load_defaults(&app);
    load_template_assets(&app);
    show_view(&app, false);
    set_hidden_all();
    run_loop(app);
}

fn set_hidden_all() {
    dom::set_hidden("viewDash", false);
    if let Some(b) = dom::document().get_element_by_id("boot") {
        b.remove();
    }
}

fn run_loop(app: Shared) {
    let cell: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let c2 = cell.clone();
    *cell.borrow_mut() = Some(Closure::new(move |now: f64| {
        if let Ok(mut a) = app.try_borrow_mut() {
            a.draw_visible(now);
        }
        if let Some(cb) = c2.borrow().as_ref() {
            dom::request_animation_frame(cb);
        }
    }));
    {
        let first = cell.borrow();
        if let Some(cb) = first.as_ref() {
            dom::request_animation_frame(cb);
        }
    }
}

/// Muat aset milik template (mis. latar IG) sekali; kartu digambar ulang setelah masing-masing siap.
fn load_template_assets(app: &Shared) {
    let (assets, store) = {
        let a = app.borrow();
        let list: Vec<(&'static str, &'static str)> = a
            .reg
            .iter()
            .flat_map(|t| t.meta().assets.iter().map(|x| (x.key, x.url)))
            .collect();
        (list, a.scene.assets.clone())
    };
    for (key, url) in assets {
        let (app, store) = (app.clone(), store.clone());
        wasm_bindgen_futures::spawn_local(async move {
            match Img::load(url).await {
                Ok(img) => {
                    store.borrow_mut().insert(key.to_string(), Rc::new(img));
                    if let Ok(mut a) = app.try_borrow_mut() {
                        a.scene.touch();
                    }
                }
                Err(e) => web_sys::console::warn_1(&format!("aset {key}: {e}").into()),
            }
        });
    }
}

/* ---------- Katalog template (dibangun otomatis dari registry) ---------- */

fn build_catalog(app: &Shared) {
    let host = html("catalog");
    host.set_inner_html("");
    let reg: Vec<Rc<dyn Template>> = app.borrow().reg.clone();
    let mut thumbs = Vec::new();
    for t in &reg {
        let m = t.meta();
        let btn = dom::make("button", "tpl-card");
        let _ = btn.set_attribute("type", "button");
        let _ = btn.set_attribute("data-tpl", m.id);
        let _ = btn.set_attribute("aria-label", &format!("Template {}", m.name));
        let wrap = dom::make("span", "tpl-thumb");
        let (w, h) = Orient::Portrait.dims(THUMB_SHORT);
        let (cv, _) = dom::new_canvas(w, h, false);
        let _ = cv.set_attribute("aria-hidden", "true");
        let _ = wrap.append_child(&cv);
        let name = dom::make("span", "tpl-name");
        name.set_text_content(Some(m.name));
        let desc = dom::make("span", "tpl-desc");
        desc.set_text_content(Some(m.description));
        let tags = dom::make("span", "tpl-tags");
        for tg in m.tags {
            let s = dom::make("span", "tpl-tag");
            s.set_text_content(Some(tg));
            let _ = tags.append_child(&s);
        }
        for n in [&wrap, &name, &desc, &tags] {
            let _ = btn.append_child(n);
        }
        let _ = host.append_child(&btn);
        let id = m.id;
        let a = app.clone();
        dom::listen(&btn, "click", move |_| {
            set_template(&a, id);
            dom::set_hidden("templateSheet", true);
        });
        let ctx = dom::context_2d(&cv, false);
        thumbs.push(Thumb {
            id: m.id,
            cv,
            ctx,
            r: Renderer::new(),
            btn,
        });
    }
    app.borrow_mut().thumbs = thumbs;
}

pub fn set_template(app: &Shared, id: &str) {
    {
        let mut a = app.borrow_mut();
        if a.rec.is_some() {
            return;
        }
        match templates::find(&a.reg, id) {
            Some(t) if t.meta().id != a.tpl.meta().id => a.tpl = t,
            _ => return,
        }
    }
    apply_template(app);
}

pub fn set_orient(app: &Shared, o: Orient) {
    {
        let mut a = app.borrow_mut();
        if a.rec.is_some() || a.orient == o {
            return;
        }
        a.orient = o;
    }
    apply_template(app);
    // Latar buatan sendiri di-crop ulang ke rasio baru.
    images::recrop_bg(app);
}

/// Sinkronkan seluruh UI dengan template + orientasi aktif.
pub fn apply_template(app: &Shared) {
    build_fields(app);
    let mut a = app.borrow_mut();
    let (o, tpl) = (a.orient, a.tpl.clone());
    let m = tpl.meta();
    let land = o.is_landscape();

    // Kontrol latar mengikuti kebijakan template.
    let free = templates::bg_is_free(m);
    dom::set_hidden("grpBg", !free);
    dom::set_hidden("grpBgLocked", free);
    if let templates::BgPolicy::Fixed { note, .. } = &m.bg {
        dom::set_text("bgLockedNote", note);
    }
    match m.art_note {
        Some(n) => {
            dom::set_text("artNote", n);
            dom::set_hidden("artNote", false);
        }
        None => dom::set_hidden("artNote", true),
    }
    dom::set_text("tplDesc", &format!("{}: {}", m.name, m.description));

    for b in query_all("#orSeg button") {
        let on = b.get_attribute("data-orient").as_deref() == Some(o.id());
        dom::set_pressed(&b, on);
    }
    html("viewDash")
        .class_list()
        .toggle_with_force("land", land)
        .ok();
    html("playSlot")
        .class_list()
        .toggle_with_force("land", land)
        .ok();

    for (id, short) in [("cvPreview", PREVIEW_SHORT), ("cvPlayer", PLAYER_SHORT)] {
        let cv: HtmlCanvasElement = by_id(id);
        let (w, h) = o.dims(short);
        cv.set_width(w);
        cv.set_height(h);
    }
    let (tw, th) = o.dims(THUMB_SHORT);
    for t in &a.thumbs {
        t.cv.set_width(tw);
        t.cv.set_height(th);
        dom::set_pressed(&t.btn, t.id == m.id);
    }
    a.thumbs_key = (0, o);

    let r = o.ratio_label();
    dom::set_text("capPrev", &format!("Pratinjau {r}"));
    dom::set_text("bgCrop", &format!("Atur crop latar ({r})"));
    dom::set_text("saveBtn", &format!("Simpan video {r}"));
    let sel: HtmlSelectElement = by_id("resSel");
    let opts = sel.options();
    for i in 0..opts.length() {
        if let Some(op) = opts
            .item(i)
            .and_then(|e| e.dyn_into::<web_sys::HtmlOptionElement>().ok())
        {
            let short: u32 = op.value().parse().unwrap_or(720);
            let (w, h) = o.dims(short);
            let tag = match short {
                540 => "540p",
                720 => "HD",
                1080 => "Full HD",
                _ => "2K",
            };
            op.set_text(&format!("{w} x {h} ({tag})"));
        }
    }
    paint_bg_ui(&a);
    a.scene.touch();
}

/// Kolom isian tambahan template aktif (device, label atas, caption, ...).
fn build_fields(app: &Shared) {
    let host = html("songFields");
    for e in query_all("#songFields .dyn") {
        e.remove();
    }
    let (tpl, o) = {
        let a = app.borrow();
        (a.tpl.clone(), a.orient)
    };
    for spec in tpl.meta().fields_for(o) {
        // Nilai awal hanya diisi sekali sehingga ketikan pengguna bertahan saat ganti template.
        let value = {
            let mut a = app.borrow_mut();
            a.scene
                .fields
                .entry(spec.id.to_string())
                .or_insert_with(|| spec.default.to_string())
                .clone()
        };
        let wrap = dom::make(
            "div",
            if spec.full {
                "field full dyn"
            } else {
                "field dyn"
            },
        );
        let lbl = dom::make("label", "lbl");
        let _ = lbl.set_attribute("for", &format!("f-{}", spec.id));
        let txt = dom::make("span", "");
        txt.set_text_content(Some(spec.label));
        let _ = lbl.append_child(&txt);
        let counter = if spec.counter {
            let c = dom::make("output", "");
            c.set_id(&format!("cnt-{}", spec.id));
            c.set_text_content(Some(&format!("{}/{}", value.chars().count(), spec.max_len)));
            let _ = lbl.append_child(&c);
            Some(c)
        } else {
            None
        };
        let input: HtmlInputElement = dom::make("input", "").dyn_into().expect("input");
        input.set_type("text");
        input.set_id(&format!("f-{}", spec.id));
        input.set_max_length(spec.max_len as i32);
        input.set_placeholder(spec.placeholder);
        input.set_value(&value);
        let _ = input.set_attribute("autocomplete", "off");
        let (id, max, a) = (spec.id, spec.max_len as usize, app.clone());
        let inp = input.clone();
        dom::listen(&input, "input", move |_| {
            let v = crate::text::limit_chars(&inp.value(), max);
            if v != inp.value() {
                inp.set_value(&v);
            }
            if let Some(c) = &counter {
                c.set_text_content(Some(&format!("{}/{}", v.chars().count(), max)));
            }
            if let Ok(mut s) = a.try_borrow_mut() {
                s.scene.fields.insert(id.to_string(), v);
                s.scene.touch();
            }
        });
        let _ = wrap.append_child(&lbl);
        let _ = wrap.append_child(&input);
        let _ = host.append_child(&wrap);
    }
}

/// Segarkan kontrol latar (hanya relevan saat template mengizinkan latar bebas).
pub fn paint_bg_ui(a: &App) {
    let s = &a.scene;
    for b in query_all("#bgSeg button") {
        let on = b
            .get_attribute("data-mode")
            .as_deref()
            .and_then(BgMode::from_id)
            == Some(s.bg_mode);
        dom::set_pressed(&b, on);
    }
    let color = s.bg_mode == BgMode::Color;
    dom::set_hidden("blurField", color);
    dom::set_hidden("paneImage", s.bg_mode != BgMode::Image);
    dom::set_hidden("paneColor", !color);
    by_id::<HtmlInputElement>("blur").set_value(&s.blur.to_string());
    by_id::<HtmlInputElement>("dim").set_value(&s.dim.to_string());
    dom::set_text("blurOut", &format!("{}", s.blur.round()));
    dom::set_text("dimOut", &format!("{}%", s.dim.round()));
}

/* ---------- Tampilan dashboard / player ---------- */

pub fn show_view(app: &Shared, player: bool) {
    app.borrow_mut().view_player = player;
    dom::set_hidden("viewDash", player);
    dom::set_hidden("viewPlayer", !player);
    let (on, off) = if player {
        ("st2", "st1")
    } else {
        ("st1", "st2")
    };
    let _ = html(on).set_attribute("aria-current", "step");
    let _ = html(off).remove_attribute("aria-current");
    dom::window().scroll_to_with_x_and_y(0.0, 0.0);
    if player {
        let _ = by_id::<HtmlCanvasElement>("cvPlayer").focus();
    }
}

fn input_value(id: &str) -> String {
    by_id::<HtmlInputElement>(id).value()
}

fn wire_dashboard(app: &Shared) {
    listen_id("openTemplates", "click", |_| dom::set_hidden("templateSheet", false));
    listen_id("closeTemplates", "click", |_| dom::set_hidden("templateSheet", true));
    for (id, is_title) in [("f-title", true), ("f-artist", false)] {
        let a = app.clone();
        listen_id(id, "input", move |_| {
            let v = input_value(id);
            let mut s = a.borrow_mut();
            if is_title {
                s.title_auto = v.trim().is_empty(); // diketik manual: jangan ditimpa lagu berikutnya
                s.scene.title = v;
            } else {
                s.scene.artist = v;
            }
            s.scene.touch();
        });
    }
    for b in query_all("#orSeg button") {
        let a = app.clone();
        let o = b
            .get_attribute("data-orient")
            .as_deref()
            .and_then(Orient::from_id);
        dom::listen(&b, "click", move |_| {
            if let Some(o) = o {
                set_orient(&a, o);
            }
        });
    }
    for b in query_all("#bgSeg button") {
        let a = app.clone();
        let mode = b
            .get_attribute("data-mode")
            .as_deref()
            .and_then(BgMode::from_id);
        dom::listen(&b, "click", move |_| {
            if let Some(m) = mode {
                let mut s = a.borrow_mut();
                s.scene.bg_mode = m;
                s.scene.touch();
                paint_bg_ui(&s);
            }
        });
    }
    for id in ["blur", "dim", "c1", "c2"] {
        let a = app.clone();
        listen_id(id, "input", move |_| {
            let v = input_value(id);
            let mut s = a.borrow_mut();
            match id {
                "blur" => s.scene.blur = v.parse().unwrap_or(60.0),
                "dim" => s.scene.dim = v.parse().unwrap_or(20.0),
                "c1" => s.scene.c1 = v,
                _ => s.scene.c2 = v,
            }
            s.scene.touch();
            paint_bg_ui(&s);
        });
    }
    for id in ["fadeIn", "fadeOut"] {
        listen_id(id, "input", move |_| {
            let v: f64 = input_value(id).parse().unwrap_or(0.0);
            dom::set_text(&format!("{id}Out"), &format!("{v:.1} dtk"));
        });
    }
    let a = app.clone();
    listen_id("goPlayer", "click", move |_| show_view(&a, true));
    let a = app.clone();
    listen_id("backBtn", "click", move |_| show_view(&a, false));
    wire_player(app);
}

/// Interaksi pada kanvas player: area sentuh berasal dari `Template::hits`.
fn wire_player(app: &Shared) {
    let cv: HtmlCanvasElement = by_id("cvPlayer");
    let to_units = {
        let cv = cv.clone();
        move |e: &PointerEvent, hh: f64| {
            let r = cv.get_bounding_client_rect();
            let x = (e.client_x() as f64 - r.left()) / r.width().max(1.0) * 100.0;
            let y = (e.client_y() as f64 - r.top()) / r.height().max(1.0) * hh;
            (x, y)
        }
    };
    let apply = |a: &Shared, h: &Hit, xu: f64| {
        let frac = ((xu - h.rect.x) / h.rect.w.max(0.001)).clamp(0.0, 1.0);
        match h.action {
            Action::Seek => media::seek_frac(a, frac),
            Action::Volume => media::set_vol(a, frac),
            _ => {}
        }
    };

    let (a, c, tu) = (app.clone(), cv.clone(), to_units.clone());
    dom::listen(&cv, "pointerdown", move |e| {
        let Ok(pe) = e.dyn_into::<PointerEvent>() else {
            return;
        };
        let hh = a.borrow().orient.hh();
        let (x, y) = tu(&pe, hh);
        let hit = a.borrow().hit_at(x, y);
        let Some(hit) = hit else { return };
        pe.prevent_default();
        match hit.action {
            Action::Seek | Action::Volume => {
                let _ = c.set_pointer_capture(pe.pointer_id());
                a.borrow_mut().drag = Some(hit);
                apply(&a, &hit, x);
            }
            Action::PlayPause => media::toggle_play(&a),
            Action::Skip(d) => media::skip(&a, d),
        }
    });

    let (a, c, tu) = (app.clone(), cv.clone(), to_units);
    dom::listen(&cv, "pointermove", move |e| {
        let Ok(pe) = e.dyn_into::<PointerEvent>() else {
            return;
        };
        let hh = a.borrow().orient.hh();
        let (x, y) = tu(&pe, hh);
        let drag = a.borrow().drag;
        if let Some(h) = drag {
            apply(&a, &h, x);
        } else {
            let over = a.borrow().hit_at(x, y).is_some();
            let _ = c
                .style()
                .set_property("cursor", if over { "pointer" } else { "default" });
        }
    });
    for ev in ["pointerup", "pointercancel"] {
        let a = app.clone();
        dom::listen(&cv, ev, move |_| a.borrow_mut().drag = None);
    }

    let a = app.clone();
    dom::listen(&cv, "keydown", move |e| {
        let Ok(k) = e.dyn_into::<web_sys::KeyboardEvent>() else {
            return;
        };
        let key = k.key();
        let handled = match key.as_str() {
            " " | "Enter" => {
                media::toggle_play(&a);
                true
            }
            "ArrowRight" => {
                media::skip(&a, 10.0);
                true
            }
            "ArrowLeft" => {
                media::skip(&a, -10.0);
                true
            }
            "ArrowUp" | "ArrowDown" => {
                let v = a.borrow().scene.vol + if key == "ArrowUp" { 0.05 } else { -0.05 };
                media::set_vol(&a, v.clamp(0.0, 1.0));
                true
            }
            _ => false,
        };
        if handled {
            k.prevent_default();
        }
    });
}

/// Pembantu: baris jam di dashboard + tombol putar, hanya disentuh DOM saat berubah.
impl App {
    fn update_clock_ui(&mut self) {
        let (d, c) = (self.dur(), self.cur());
        let c = if d > 0.0 { c.min(d) } else { c };
        let txt = if self.has_audio {
            format!("{} / {}", fmt_time(c), fmt_time(d))
        } else {
            "Belum ada lagu".to_string()
        };
        if txt != self.last_clock {
            dom::set_text("clock", &txt);
            self.last_clock = txt;
        }
        let real = self.is_playing_real();
        if self.last_play != Some(real) {
            self.last_play = Some(real);
            dom::set_text("playLbl", if real { "Stop" } else { "Putar" });
            html("playIco").set_inner_html(if real {
                "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\"><rect x=\"3\" y=\"3\" width=\"10\" height=\"10\" rx=\"2\" fill=\"currentColor\"/></svg>"
            } else {
                "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\"><path d=\"M4 2.5v11l9-5.5z\" fill=\"currentColor\"/></svg>"
            });
        }
    }
}
