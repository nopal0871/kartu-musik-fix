//! Cadangan: rekam kanvas + suara secara real-time dengan MediaRecorder.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::{Function, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, HtmlAudioElement};

use super::{fade_gain, ExportError, Job};
use crate::bridge::{self, AudioGraph, Recorder};
use crate::dom;
use crate::geom::fmt_time;
use crate::render::Renderer;
use crate::spectrum::{band_targets, Spectrum, IDLE};
use crate::state::Frame;

pub async fn run(
    job: &Job,
    audio: &HtmlAudioElement,
    graph: Option<Rc<AudioGraph>>,
    len: f64,
) -> Result<(Blob, String), ExportError> {
    let cfg = job.cfg.clone();
    let mime = bridge::pick_mime();
    if mime.is_null() {
        return Err(ExportError::Failed(
            "Browser ini belum mendukung perekaman video. Coba Chrome, Edge, atau Safari terbaru."
                .into(),
        ));
    }
    let mime = mime.as_string().unwrap_or_default();
    let _ = JsFuture::from(bridge::preload_fonts()).await;

    let (canvas, ctx) = dom::new_canvas(cfg.w, cfg.h, true);
    let renderer = Rc::new(Renderer::new());
    let g0 = fade_gain(0.0, len, cfg.fade_in, cfg.fade_out);
    let first = Frame {
        t: 0.0,
        cur: 0.0,
        dur: len,
        playing: true,
        spec: IDLE,
        fade: 1.0 - g0,
        fade_color: cfg.fade_color,
    };
    renderer.draw(
        &ctx, cfg.w, cfg.h, &*job.tpl, &job.scene, &first, job.orient,
    );

    if let Some(g) = &graph {
        let _ = JsFuture::from(g.resume()).await;
        g.set_rec_gain(g0 * 0.95);
    }
    let rec = Recorder::new(&canvas, cfg.fps, graph.as_deref(), cfg.vbr, &mime)?;

    audio.pause().ok();
    audio.set_current_time(0.0);
    match audio.play() {
        Ok(p) => {
            if JsFuture::from(p).await.is_err() {
                return Err(ExportError::Failed(
                    "Lagu tidak bisa diputar. Coba file lain.".into(),
                ));
            }
        }
        Err(_) => {
            return Err(ExportError::Failed(
                "Lagu tidak bisa diputar. Coba file lain.".into(),
            ))
        }
    }
    let wake = JsFuture::from(bridge::request_wake_lock())
        .await
        .unwrap_or(JsValue::NULL);
    rec.start();

    // Promise yang selesai saat lagu habis atau dibatalkan.
    let resolve: Rc<RefCell<Option<Function>>> = Rc::new(RefCell::new(None));
    let done = {
        let r = resolve.clone();
        js_sys::Promise::new(&mut move |res, _| {
            *r.borrow_mut() = Some(res);
        })
    };
    let ending = Rc::new(Cell::new(false));
    let spec = Rc::new(RefCell::new(Spectrum::default()));
    let buf = Uint8Array::new_with_length(256);

    let tick = {
        let (audio, ctx, renderer, resolve, ending, spec) = (
            audio.clone(),
            ctx.clone(),
            renderer.clone(),
            resolve.clone(),
            ending.clone(),
            spec.clone(),
        );
        let (tpl, scene, orient, cancel, progress) = (
            job.tpl.clone(),
            job.scene.clone(),
            job.orient,
            job.cancel.clone(),
            job.progress.clone(),
        );
        let graph_js = graph.clone();
        let cfg = cfg.clone();
        Closure::<dyn FnMut()>::new(move || {
            let finish = |resolve: &Rc<RefCell<Option<Function>>>| {
                if let Some(f) = resolve.borrow_mut().take() {
                    let _ = f.call0(&JsValue::NULL);
                }
            };
            if ending.get() {
                return;
            }
            if cancel.get() {
                ending.set(true);
                finish(&resolve);
                return;
            }
            let played = audio.current_time().max(0.0);
            {
                let mut sp = spec.borrow_mut();
                let tgt = match &graph_js {
                    Some(g) => {
                        g.bytes(&buf);
                        let mut b = [0u8; 256];
                        buf.copy_to(&mut b);
                        band_targets(&b)
                    }
                    None => IDLE,
                };
                sp.step(&tgt, cfg.fps as f64);
            }
            let fg = fade_gain(played.min(len), len, cfg.fade_in, cfg.fade_out);
            if let Some(g) = &graph_js {
                g.set_rec_gain(fg * 0.95);
            }
            let frame = Frame {
                t: played,
                cur: played.min(len),
                dur: len,
                playing: true,
                spec: spec.borrow().v,
                fade: 1.0 - fg,
                fade_color: cfg.fade_color,
            };
            renderer.draw(&ctx, cfg.w, cfg.h, &*tpl, &scene, &frame, orient);
            progress(format!(
                "Merekam {} / {}",
                fmt_time(played.min(len)),
                fmt_time(len)
            ));
            if played >= len - 0.05 || audio.ended() {
                ending.set(true);
                let r = resolve.clone();
                let cb = Closure::once_into_js(move || {
                    if let Some(f) = r.borrow_mut().take() {
                        let _ = f.call0(&JsValue::NULL);
                    }
                });
                let _ = dom::window()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 300);
            }
        })
    };
    let stop_tick = bridge::make_ticker(&tick, 1000.0 / cfg.fps as f64);

    let _ = JsFuture::from(done).await;
    let _ = stop_tick.call0(&JsValue::NULL);
    drop(tick);
    audio.pause().ok();
    if let Some(g) = &graph {
        g.set_rec_gain(0.95);
    }
    let _ = JsFuture::from(rec.stop()).await;
    bridge::release_wake_lock(&wake);

    if job.cancel.get() {
        return Err(ExportError::Cancelled);
    }
    let blob = rec.blob();
    if blob.size() <= 0.0 {
        return Err(ExportError::Failed("Video kosong. Coba lagi.".into()));
    }
    let ext = if blob.type_().contains("mp4") {
        "mp4"
    } else {
        "webm"
    };
    Ok((blob, ext.to_string()))
}
