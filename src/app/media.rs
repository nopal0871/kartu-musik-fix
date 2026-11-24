//! Audio: berkas lagu, pemutaran, posisi, volume.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Event, File, HtmlInputElement};

use super::{App, Shared};
use crate::bridge::AudioGraph;
use crate::dom::{self, by_id, html, listen_id, toast};
use crate::geom::fmt_time;
use crate::text::guess_title;

impl App {
    pub fn is_playing_real(&self) -> bool {
        self.has_audio && !self.audio.paused() && !self.audio.ended()
    }
    pub fn dur(&self) -> f64 {
        if self.has_audio {
            let d = self.audio.duration();
            if d.is_finite() {
                d
            } else {
                0.0
            }
        } else {
            self.demo.dur
        }
    }
    pub fn cur(&self) -> f64 {
        if self.has_audio {
            self.audio.current_time()
        } else {
            self.demo.cur
        }
    }
    pub fn shown_playing(&self) -> bool {
        if self.has_audio {
            self.is_playing_real()
        } else {
            self.demo.playing
        }
    }

    /// Grafik WebAudio dibuat sekali (satu `<audio>` hanya boleh punya satu sumber media).
    pub fn ensure_graph(&mut self) {
        if self.graph.is_none() {
            self.graph = AudioGraph::new(&self.audio, self.scene.vol)
                .ok()
                .map(std::rc::Rc::new);
        }
    }

    fn update_dur_info(&self) {
        let d = self.dur();
        let t = if self.has_audio && d > 0.0 {
            format!(
                "Durasi video otomatis sama dengan lagu: {}. Ekspor dibuat tanpa menunggu lagu diputar, jadi jauh lebih cepat. Fade baru terlihat di video hasil ekspor.",
                fmt_time(d)
            )
        } else {
            "Durasi video otomatis sama dengan lagu. Tambahkan lagu untuk melihat durasinya. Fade baru terlihat di video hasil ekspor.".to_string()
        };
        dom::set_text("durInfo", &t);
    }
}

pub fn play_now(app: &Shared) {
    let (audio, graph) = {
        let mut a = app.borrow_mut();
        a.ensure_graph();
        (a.audio.clone(), a.graph.clone())
    };
    spawn_local(async move {
        if let Some(g) = graph {
            let _ = JsFuture::from(g.resume()).await;
        }
        match audio.play() {
            Ok(p) => {
                if JsFuture::from(p).await.is_err() {
                    toast("Lagu tidak bisa diputar. Coba file lain.");
                }
            }
            Err(_) => toast("Lagu tidak bisa diputar. Coba file lain."),
        }
    });
}

pub fn toggle_play(app: &Shared) {
    let (rec, has, real) = {
        let a = app.borrow();
        (a.rec.is_some(), a.has_audio, a.is_playing_real())
    };
    if rec {
        return;
    }
    if !has {
        let mut a = app.borrow_mut();
        a.demo.playing = !a.demo.playing;
        toast("Belum ada lagu. Hanya ikon yang berubah.");
        return;
    }
    if real {
        let _ = app.borrow().audio.pause();
    } else {
        play_now(app);
    }
}

pub fn seek_frac(app: &Shared, f: f64) {
    let mut a = app.borrow_mut();
    if a.has_audio {
        let d = a.audio.duration();
        if d.is_finite() && d > 0.0 {
            a.audio.set_current_time(f * d);
        }
    } else {
        a.demo.cur = f * a.demo.dur;
    }
}

pub fn skip(app: &Shared, delta: f64) {
    let mut a = app.borrow_mut();
    let d = a.dur();
    if a.has_audio {
        if d > 0.0 {
            let t = (a.audio.current_time() + delta).clamp(0.0, d);
            a.audio.set_current_time(t);
        }
    } else {
        a.demo.cur = (a.demo.cur + delta).clamp(0.0, a.demo.dur);
    }
}

pub fn set_vol(app: &Shared, f: f64) {
    let mut a = app.borrow_mut();
    a.scene.vol = f;
    match &a.graph {
        Some(g) => g.set_volume(f),
        None => a.audio.set_volume(f * 0.95),
    }
    a.scene.touch();
}

fn load_audio(app: &Shared, f: File) {
    let mut a = app.borrow_mut();
    let _ = a.audio.pause();
    if !a.blob_url.is_empty() {
        let _ = web_sys::Url::revoke_object_url(&a.blob_url);
    }
    let url = web_sys::Url::create_object_url_with_blob(&f).unwrap_or_default();
    a.audio.set_src(&url);
    a.blob_url = url;
    a.has_audio = true;
    a.ensure_graph();
    dom::set_text("audName", &f.name());
    dom::set_text("audHint", "Memuat lagu");
    // Judul otomatis dari nama berkas; tidak menimpa judul yang diketik sendiri.
    let guess = guess_title(&f.name());
    if !guess.is_empty() && (a.title_auto || a.scene.title.trim().is_empty()) {
        a.scene.title = guess.clone();
        a.title_auto = true;
        by_id::<HtmlInputElement>("f-title").set_value(&guess);
        a.scene.touch();
    }
    a.audio_file = Some(f);
    a.audio.load();
    a.update_dur_info();
}

pub fn wire_drop(
    app: &Shared,
    btn_id: &'static str,
    input_id: &'static str,
    prefix: &'static str,
    msg: &'static str,
    handler: fn(&Shared, File),
) {
    let input: HtmlInputElement = by_id(input_id);
    let btn = html(btn_id);
    let i2 = input.clone();
    dom::listen(&btn, "click", move |_| i2.click());
    let (a, i3) = (app.clone(), input.clone());
    dom::listen(&input, "change", move |_| {
        if let Some(f) = i3.files().and_then(|l| l.get(0)) {
            handler(&a, f);
        }
        i3.set_value("");
    });
    for ev in ["dragenter", "dragover"] {
        let b = btn.clone();
        dom::listen(&btn, ev, move |e: Event| {
            e.prevent_default();
            let _ = b.class_list().add_1("over");
        });
    }
    let (a, b) = (app.clone(), btn.clone());
    dom::listen(&btn, "dragleave", {
        let b = b.clone();
        move |e: Event| {
            e.prevent_default();
            let _ = b.class_list().remove_1("over");
        }
    });
    dom::listen(&btn, "drop", move |e: Event| {
        e.prevent_default();
        let _ = b.class_list().remove_1("over");
        let Ok(de) = e.dyn_into::<web_sys::DragEvent>() else {
            return;
        };
        let Some(f) = de
            .data_transfer()
            .and_then(|d| d.files())
            .and_then(|l| l.get(0))
        else {
            return;
        };
        if f.type_().starts_with(prefix) {
            handler(&a, f)
        } else {
            toast(msg)
        }
    });
}

pub fn wire(app: &Shared) {
    wire_drop(
        app,
        "audDrop",
        "audFile",
        "audio",
        "File itu bukan audio.",
        load_audio,
    );

    // Tombol Putar/Stop di dashboard
    let a = app.clone();
    listen_id("btnPlay", "click", move |_| {
        let (rec, has, real) = {
            let s = a.borrow();
            (s.rec.is_some(), s.has_audio, s.is_playing_real())
        };
        if rec {
            return;
        }
        if !has {
            toast("Tambahkan file lagu terlebih dahulu.");
            return;
        }
        if real {
            let s = a.borrow();
            let _ = s.audio.pause();
            s.audio.set_current_time(0.0);
        } else {
            play_now(&a);
        }
    });

    // Peristiwa <audio>
    let audio = app.borrow().audio.clone();
    for ev in ["loadedmetadata", "durationchange"] {
        let a = app.clone();
        dom::listen(&audio, ev, move |_| {
            if let Ok(s) = a.try_borrow() {
                let d = s.audio.duration();
                if s.has_audio && d.is_finite() {
                    dom::set_text(
                        "audHint",
                        &format!("Durasi {}. Klik untuk mengganti lagu.", fmt_time(d)),
                    );
                    s.update_dur_info();
                }
            }
        });
    }
    let a = app.clone();
    dom::listen(&audio, "error", move |_| {
        if let Ok(mut s) = a.try_borrow_mut() {
            if s.audio_file.is_none() {
                return;
            }
            s.has_audio = false;
            dom::set_text("audName", "Tambahkan file lagu");
            dom::set_text(
                "audHint",
                "File tidak bisa dibaca. Coba MP3, M4A, atau WAV.",
            );
            s.update_dur_info();
            toast("File lagu tidak bisa dibaca.");
        }
    });
}
