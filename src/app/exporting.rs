//! Tombol simpan: pilih jalur ekspor, tampilkan hasil, unduh / bagikan.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Blob, HtmlAnchorElement, HtmlButtonElement, HtmlInputElement, HtmlSelectElement,
    HtmlVideoElement,
};

use super::{images, Shared};
use crate::bridge;
use crate::dom::{self, by_id, html, listen_id, toast};
use crate::export::{fast, realtime, Cfg, ExportError, Job, Quality};
use crate::text::file_base;

pub fn set_rec_ui(on: bool, label: Option<&str>, ratio: &str) {
    by_id::<HtmlButtonElement>("saveBtn").set_disabled(on);
    by_id::<HtmlButtonElement>("backBtn").set_disabled(on);
    dom::set_hidden("cancelBtn", !on);
    let txt = label
        .map(str::to_string)
        .unwrap_or_else(|| format!("Simpan video {ratio}"));
    dom::set_text("saveBtn", &txt);
}

fn read_cfg(o: crate::geom::Orient) -> Cfg {
    let short = by_id::<HtmlSelectElement>("resSel")
        .value()
        .parse()
        .unwrap_or(720);
    let fps = by_id::<HtmlSelectElement>("fpsSel")
        .value()
        .parse()
        .unwrap_or(30);
    let num = |id: &str| {
        by_id::<HtmlInputElement>(id)
            .value()
            .parse::<f64>()
            .unwrap_or(0.0)
    };
    let white = by_id::<HtmlSelectElement>("fadeCol").value() == "white";
    let q = Quality::from_id(&by_id::<HtmlSelectElement>("qualSel").value());
    Cfg::new(o, short, fps, q, num("fadeIn"), num("fadeOut"), white)
}

fn start_export(app: &Shared) {
    let ctx = {
        let mut a = app.borrow_mut();
        if a.rec.is_some() {
            return;
        }
        if !a.has_audio {
            toast("Tambahkan file lagu dulu. Durasi video mengikuti lagu.");
            return;
        }
        let total = a.dur();
        if total <= 0.0 {
            toast("Durasi lagu belum terbaca. Tunggu sebentar lalu coba lagi.");
            return;
        }
        let Some(file) = a.audio_file.clone() else {
            return;
        };
        let cancel = Rc::new(Cell::new(false));
        a.rec = Some(cancel.clone());
        let _ = a.audio.pause();
        let job = Job {
            cfg: read_cfg(a.orient),
            tpl: a.tpl.clone(),
            scene: a.scene.clone(),
            orient: a.orient,
            cancel,
            progress: Rc::new(|s| dom::set_text("saveBtn", &s)),
        };
        (
            job,
            file,
            a.audio.clone(),
            a.graph.clone(),
            total,
            a.audio.current_time(),
            a.orient,
        )
    };
    let (job, file, audio, graph, total, prev_cur, orient) = ctx;
    let ratio = orient.ratio_label();
    set_rec_ui(true, Some("Menyiapkan"), ratio);

    let app = app.clone();
    spawn_local(async move {
        let mut result: Result<(Blob, String), ExportError> =
            Err(ExportError::Failed("Video gagal dibuat. Coba lagi.".into()));
        let mut fallback = true;
        if let Some(pick) = fast::pick(&job).await {
            let wake = JsFuture::from(bridge::request_wake_lock())
                .await
                .unwrap_or(wasm_bindgen::JsValue::NULL);
            let r = fast::run(&job, &file, &pick).await;
            bridge::release_wake_lock(&wake);
            match r {
                Ok(v) => {
                    result = Ok(v);
                    fallback = false;
                }
                Err(ExportError::Cancelled) => {
                    result = Err(ExportError::Cancelled);
                    fallback = false;
                }
                Err(ExportError::Failed(m)) => {
                    web_sys::console::warn_1(&format!("ekspor cepat gagal: {m}").into());
                    toast("Ekspor cepat gagal di browser ini. Beralih ke perekaman real-time.");
                }
            }
        }
        if fallback {
            result = realtime::run(&job, &audio, graph, total).await;
            let _ = audio.pause();
            audio.set_current_time(prev_cur);
        }
        app.borrow_mut().rec = None;
        set_rec_ui(false, None, ratio);
        match result {
            Ok((blob, ext)) => open_sheet(&app, blob, &ext),
            Err(ExportError::Cancelled) => toast("Ekspor dibatalkan."),
            Err(ExportError::Failed(m)) => {
                web_sys::console::warn_1(&m.clone().into());
                toast("Video gagal dibuat. Coba lagi.");
            }
        }
    });
}

fn open_sheet(app: &Shared, blob: Blob, ext: &str) {
    let (name, old) = {
        let mut a = app.borrow_mut();
        let name = format!("{}.{ext}", file_base(&a.scene.title, a.orient));
        let old = std::mem::take(&mut a.sheet_url);
        a.sheet_blob = Some(blob.clone());
        (name, old)
    };
    if !old.is_empty() {
        let _ = web_sys::Url::revoke_object_url(&old);
    }
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap_or_default();
    app.borrow_mut().sheet_url = url.clone();
    by_id::<HtmlVideoElement>("sheetVid").set_src(&url);
    let dl: HtmlAnchorElement = by_id("sheetDl");
    dl.set_href(&url);
    dl.set_download(&name);
    dl.set_hidden(false);
    dom::set_hidden("sheetShare", !bridge::can_share_file(&blob, &name));
    dom::set_text(
        "sheetMsg",
        "Video siap. Tekan Unduh video, atau tekan lama video untuk menyimpan.",
    );
    dom::set_hidden("sheet", false);
}

fn close_sheet() {
    let _ = by_id::<HtmlVideoElement>("sheetVid").pause();
    dom::set_hidden("sheet", true);
}

pub fn wire(app: &Shared) {
    let a = app.clone();
    listen_id("saveBtn", "click", move |_| start_export(&a));
    let a = app.clone();
    listen_id("cancelBtn", "click", move |_| {
        if let Some(c) = &a.borrow().rec {
            c.set(true);
        }
    });
    listen_id("sheetClose", "click", |_| close_sheet());
    listen_id("sheet", "click", |e| {
        if e.target().as_ref() == Some(html("sheet").as_ref()) {
            close_sheet();
        }
    });
    let a = app.clone();
    listen_id("sheetShare", "click", move |_| {
        let blob = a.borrow().sheet_blob.clone();
        if let Some(b) = blob {
            let name = format!(
                "{}.{}",
                file_base(&a.borrow().scene.title, a.borrow().orient),
                if b.type_().contains("mp4") {
                    "mp4"
                } else {
                    "webm"
                }
            );
            spawn_local(async move {
                let _ = JsFuture::from(bridge::share_file(&b, &name)).await;
            });
        }
    });
    let a = app.clone();
    dom::listen(&dom::document(), "keydown", move |e| {
        let Ok(k) = wasm_bindgen::JsCast::dyn_into::<web_sys::KeyboardEvent>(e) else {
            return;
        };
        if k.key() != "Escape" {
            return;
        }
        if !html("sheet").hidden() {
            close_sheet();
        }
        if !html("cropSheet").hidden() {
            images::close_crop(&a);
        }
    });
}
