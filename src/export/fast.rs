//! Ekspor cepat (offline) lewat WebCodecs.

use js_sys::Float32Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File};

use super::{fade_gain, frame_count, ExportError, Job};
use crate::bridge::{self, Encoder};
use crate::dom;
use crate::render::Renderer;
use crate::spectrum::{band_targets, Analyzer, Spectrum, WINDOW};
use crate::state::Frame;

const SR: f64 = 48_000.0;
const BLOCK: usize = 24_000;

/// Cari codec yang didukung. `None` = pakai jalur real-time.
pub async fn pick(job: &Job) -> Option<wasm_bindgen::JsValue> {
    if !bridge::has_web_codecs() {
        return None;
    }
    let c = &job.cfg;
    let v = JsFuture::from(bridge::pick_codecs(c.w, c.h, c.fps, c.vbr))
        .await
        .ok()?;
    if v.is_null() || v.is_undefined() {
        None
    } else {
        Some(v)
    }
}

/// Salin `count` sampel mulai `from` dari JS Float32Array ke `dst` (tanpa memuat seluruh lagu ke memori wasm).
fn read(src: &Float32Array, from: u32, count: u32, dst: &mut [f32]) {
    if count == 0 {
        return;
    }
    src.subarray(from, from + count)
        .copy_to(&mut dst[..count as usize]);
}

pub async fn run(
    job: &Job,
    file: &File,
    pick: &wasm_bindgen::JsValue,
) -> Result<(Blob, String), ExportError> {
    let cancel = &job.cancel;
    let cfg = &job.cfg;
    let check = || {
        if cancel.get() {
            Err(ExportError::Cancelled)
        } else {
            Ok(())
        }
    };

    (job.progress)("Membaca lagu".into());
    let _ = JsFuture::from(bridge::preload_fonts()).await;
    let song = JsFuture::from(bridge::decode_song(file)).await?;
    check()?;

    let total = bridge::song_length(&song) as usize;
    let rate = bridge::song_rate(&song);
    let len = total as f64 / rate; // durasi PCM akurat hasil dekode
    let left = bridge::song_channel(&song, 0);
    let right = bridge::song_channel(&song, 1);

    let (canvas, ctx) = dom::new_canvas(cfg.w, cfg.h, false);
    let enc = Encoder::new(pick, &canvas)?;
    let result = encode_all(job, &enc, &canvas, &ctx, &left, &right, total, len, &check).await;
    let out = match result {
        Ok(()) => JsFuture::from(enc.finish())
            .await
            .map_err(ExportError::from)
            .and_then(|b| {
                b.dyn_into::<Blob>()
                    .map_err(|_| ExportError::Failed("hasil bukan Blob".into()))
            }),
        Err(e) => Err(e),
    };
    let ext = enc.extension();
    enc.close();
    out.map(|b| (b, ext))
}

#[allow(clippy::too_many_arguments)]
async fn encode_all(
    job: &Job,
    enc: &Encoder,
    _canvas: &web_sys::HtmlCanvasElement,
    ctx: &web_sys::CanvasRenderingContext2d,
    left: &Float32Array,
    right: &Float32Array,
    total: usize,
    len: f64,
    check: &dyn Fn() -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    let cfg = &job.cfg;
    let fps = cfg.fps as f64;
    let renderer = Renderer::new();
    let mut an = Analyzer::new();
    let mut spec = Spectrum::default();
    let decay = 0.55_f64.powf(60.0 / fps);
    let n_frames = frame_count(len, cfg.fps);
    let (fi_e, fo_e) = (cfg.fade_in.min(len / 2.0), cfg.fade_out.min(len / 2.0));

    // Buffer yang dipakai ulang di seluruh loop.
    let mut win_l = vec![0.0f32; WINDOW];
    let mut win_r = vec![0.0f32; WINDOW];
    let mut mono = vec![0.0f32; WINDOW];
    let mut blk_l = vec![0.0f32; BLOCK];
    let mut blk_r = vec![0.0f32; BLOCK];
    let mut planar = vec![0.0f32; BLOCK * 2];
    let mut a_pos = 0usize;

    let mut feed_audio = |upto_sec: f64, enc: &Encoder| -> Result<(), ExportError> {
        let target = total.min((upto_sec * SR).floor().max(0.0) as usize);
        while a_pos < target {
            let n = BLOCK.min(target - a_pos);
            read(left, a_pos as u32, n as u32, &mut blk_l);
            read(right, a_pos as u32, n as u32, &mut blk_r);
            for i in 0..n {
                let tt = (a_pos + i) as f64 / SR;
                let g = if tt >= fi_e && tt <= len - fo_e {
                    1.0
                } else {
                    fade_gain(tt, len, cfg.fade_in, cfg.fade_out)
                } * 0.95;
                planar[i] = blk_l[i] * g as f32;
                planar[n + i] = blk_r[i] * g as f32;
            }
            let arr = Float32Array::from(&planar[..n * 2]);
            enc.add_audio(&arr, n as u32, (a_pos as f64 * 1e6 / SR).round())?;
            a_pos += n;
        }
        Ok(())
    };

    let t0 = dom::now_ms();
    let mut last_ui = 0.0;
    for i in 0..n_frames {
        check()?;
        let e = enc.error();
        if !e.is_empty() {
            return Err(ExportError::Failed(e));
        }
        let t = i as f64 / fps;

        // Jendela 512 sampel yang berakhir pada t, dinolkan di luar lagu.
        let end = (t * SR).round() as i64;
        let start = end - WINDOW as i64;
        win_l.iter_mut().for_each(|v| *v = 0.0);
        win_r.iter_mut().for_each(|v| *v = 0.0);
        let (a, b) = (start.max(0), end.min(total as i64));
        if b > a {
            let off = (a - start) as usize;
            let cnt = (b - a) as u32;
            read(left, a as u32, cnt, &mut win_l[off..]);
            read(right, a as u32, cnt, &mut win_r[off..]);
        }
        for k in 0..WINDOW {
            mono[k] = (win_l[k] + win_r[k]) * 0.5;
        }
        let tgt = band_targets(an.bytes_window(&mono, decay));
        spec.step(&tgt, fps);

        let fg = fade_gain(t, len, cfg.fade_in, cfg.fade_out);
        let frame = Frame {
            t,
            cur: t,
            dur: len,
            playing: true,
            spec: spec.v,
            fade: 1.0 - fg,
            fade_color: cfg.fade_color,
        };
        renderer.draw(ctx, cfg.w, cfg.h, &*job.tpl, &job.scene, &frame, job.orient);
        enc.add_frame((i as f64 * 1e6 / fps).round(), i % (cfg.fps * 2) == 0)?;
        feed_audio(t + 1.0, enc)?;

        // Tekanan balik: jangan menumpuk frame mentah di antrean encoder.
        while enc.queue_size() > 8 && enc.error().is_empty() && !job.cancel.get() {
            let _ = JsFuture::from(bridge::yield_ui()).await;
        }
        let now = dom::now_ms();
        if now - last_ui > 120.0 {
            last_ui = now;
            let el = (now - t0) / 1000.0;
            let pct = (100.0 * i as f64 / n_frames as f64).floor();
            let speed = if el > 1.0 {
                format!(" ({}x lebih cepat)", ((t / el).round() as i64).max(1))
            } else {
                String::new()
            };
            (job.progress)(format!("Mengekspor {pct}%{speed}"));
            let _ = JsFuture::from(bridge::yield_ui()).await;
        }
    }
    (job.progress)("Menyelesaikan".into());
    feed_audio(len + 1.0, enc)?;
    let e = enc.error();
    if !e.is_empty() {
        return Err(ExportError::Failed(e));
    }
    Ok(())
}
