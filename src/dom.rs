//! Pembantu tipis untuk DOM. Tidak ada logika aplikasi di sini.

use std::cell::Cell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, Document, Event, EventTarget, HtmlCanvasElement, HtmlElement, Window,
};

pub fn window() -> Window {
    web_sys::window().expect("window tidak tersedia")
}

pub fn document() -> Document {
    window().document().expect("document tidak tersedia")
}

/// Ambil elemen berdasarkan id; panik dengan pesan jelas bila HTML dan kode tidak sinkron.
pub fn by_id<T: JsCast>(id: &str) -> T {
    document()
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("elemen #{id} tidak ada di index.html"))
        .dyn_into::<T>()
        .unwrap_or_else(|_| panic!("elemen #{id} bertipe tidak sesuai"))
}

pub fn html(id: &str) -> HtmlElement {
    by_id::<HtmlElement>(id)
}

pub fn set_text(id: &str, text: &str) {
    html(id).set_text_content(Some(text));
}

pub fn set_hidden(id: &str, hidden: bool) {
    html(id).set_hidden(hidden);
}

/// Pasang listener yang hidup selama aplikasi berjalan.
pub fn listen(target: &EventTarget, event: &str, f: impl FnMut(Event) + 'static) {
    let cb = Closure::<dyn FnMut(Event)>::new(f);
    target
        .add_event_listener_with_callback(event, cb.as_ref().unchecked_ref())
        .expect("add_event_listener gagal");
    cb.forget();
}

pub fn listen_id(id: &str, event: &str, f: impl FnMut(Event) + 'static) {
    listen(&html(id), event, f);
}

/// Buat kanvas 2D. `alpha=false` mempercepat komposit dan dipakai untuk ekspor.
pub fn new_canvas(w: u32, h: u32, alpha: bool) -> (HtmlCanvasElement, CanvasRenderingContext2d) {
    let cv: HtmlCanvasElement = document()
        .create_element("canvas")
        .expect("create canvas")
        .dyn_into()
        .expect("canvas");
    cv.set_width(w.max(1));
    cv.set_height(h.max(1));
    let ctx = context_2d(&cv, alpha);
    (cv, ctx)
}

pub fn context_2d(cv: &HtmlCanvasElement, alpha: bool) -> CanvasRenderingContext2d {
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"alpha".into(), &JsValue::from_bool(alpha));
    cv.get_context_with_context_options("2d", &opts)
        .ok()
        .flatten()
        .and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok())
        .expect("Canvas 2D tidak didukung browser ini")
}

thread_local! {
    static TOAST_TIMER: Cell<i32> = const { Cell::new(0) };
}

pub fn toast(msg: &str) {
    let el = html("toast");
    el.set_text_content(Some(msg));
    el.set_hidden(false);
    let w = window();
    TOAST_TIMER.with(|t| {
        if t.get() != 0 {
            w.clear_timeout_with_handle(t.get());
        }
        let cb = Closure::once_into_js(|| html("toast").set_hidden(true));
        let id = w
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 3600)
            .unwrap_or(0);
        t.set(id);
    });
}

pub fn now_ms() -> f64 {
    window().performance().map(|p| p.now()).unwrap_or(0.0)
}

pub fn request_animation_frame(cb: &Closure<dyn FnMut(f64)>) {
    let _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
}

/// Semua elemen yang cocok dengan selector.
pub fn query_all(sel: &str) -> Vec<HtmlElement> {
    let mut out = Vec::new();
    if let Ok(list) = document().query_selector_all(sel) {
        for i in 0..list.length() {
            if let Some(n) = list.item(i) {
                if let Ok(e) = n.dyn_into::<HtmlElement>() {
                    out.push(e);
                }
            }
        }
    }
    out
}

pub fn make(tag: &str, class: &str) -> HtmlElement {
    let e: HtmlElement = document()
        .create_element(tag)
        .expect("create_element")
        .dyn_into()
        .expect("HtmlElement");
    if !class.is_empty() {
        e.set_class_name(class);
    }
    e
}

pub fn set_pressed(el: &HtmlElement, on: bool) {
    let _ = el.set_attribute("aria-pressed", if on { "true" } else { "false" });
}

pub fn set_bg_image(el: &HtmlElement, url: &str) {
    let _ = el
        .style()
        .set_property("background-image", &format!("url(\"{url}\")"));
}
