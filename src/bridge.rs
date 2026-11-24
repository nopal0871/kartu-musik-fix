//! Binding Rust ke `web/bridge.js` (API browser tanpa binding stabil).

use js_sys::{Float32Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::{Blob, File, HtmlAudioElement, HtmlCanvasElement};

#[wasm_bindgen(module = "/web/bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = preloadFonts)]
    pub fn preload_fonts() -> Promise;
    #[wasm_bindgen(js_name = onFontsChange)]
    pub fn on_fonts_change(cb: &Closure<dyn FnMut()>);
    #[wasm_bindgen(js_name = yieldUi)]
    pub fn yield_ui() -> Promise;
    #[wasm_bindgen(js_name = hasWebCodecs)]
    pub fn has_web_codecs() -> bool;
    #[wasm_bindgen(js_name = requestWakeLock)]
    pub fn request_wake_lock() -> Promise;
    #[wasm_bindgen(js_name = releaseWakeLock)]
    pub fn release_wake_lock(lock: &JsValue);

    #[wasm_bindgen(js_name = decodeSong)]
    pub fn decode_song(file: &File) -> Promise;
    #[wasm_bindgen(js_name = songChannels)]
    pub fn song_channels(buf: &JsValue) -> u32;
    #[wasm_bindgen(js_name = songLength)]
    pub fn song_length(buf: &JsValue) -> u32;
    #[wasm_bindgen(js_name = songRate)]
    pub fn song_rate(buf: &JsValue) -> f64;
    #[wasm_bindgen(js_name = songChannel)]
    pub fn song_channel(buf: &JsValue, i: u32) -> Float32Array;

    pub type AudioGraph;
    #[wasm_bindgen(constructor, catch)]
    pub fn new(audio: &HtmlAudioElement, vol: f64) -> Result<AudioGraph, JsValue>;
    #[wasm_bindgen(method)]
    pub fn resume(this: &AudioGraph) -> Promise;
    #[wasm_bindgen(method, js_name = setVolume)]
    pub fn set_volume(this: &AudioGraph, v: f64);
    #[wasm_bindgen(method, js_name = setRecGain)]
    pub fn set_rec_gain(this: &AudioGraph, v: f64);
    #[wasm_bindgen(method)]
    pub fn bytes(this: &AudioGraph, out: &Uint8Array);

    #[wasm_bindgen(js_name = pickCodecs)]
    pub fn pick_codecs(width: u32, height: u32, fps: u32, bitrate: f64) -> Promise;

    pub type Encoder;
    #[wasm_bindgen(constructor, catch)]
    pub fn new(pick: &JsValue, canvas: &HtmlCanvasElement) -> Result<Encoder, JsValue>;
    #[wasm_bindgen(method)]
    pub fn extension(this: &Encoder) -> String;
    #[wasm_bindgen(method)]
    pub fn error(this: &Encoder) -> String;
    #[wasm_bindgen(method, js_name = queueSize)]
    pub fn queue_size(this: &Encoder) -> u32;
    #[wasm_bindgen(method, js_name = addFrame, catch)]
    pub fn add_frame(this: &Encoder, ts_us: f64, key: bool) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_name = addAudio, catch)]
    pub fn add_audio(
        this: &Encoder,
        data: &Float32Array,
        n: u32,
        ts_us: f64,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(method)]
    pub fn finish(this: &Encoder) -> Promise;
    #[wasm_bindgen(method)]
    pub fn close(this: &Encoder);

    #[wasm_bindgen(js_name = pickMime)]
    pub fn pick_mime() -> JsValue;

    pub type Recorder;
    #[wasm_bindgen(constructor, catch)]
    pub fn new(
        canvas: &HtmlCanvasElement,
        fps: u32,
        graph: Option<&AudioGraph>,
        bitrate: f64,
        mime: &str,
    ) -> Result<Recorder, JsValue>;
    #[wasm_bindgen(method)]
    pub fn start(this: &Recorder);
    #[wasm_bindgen(method)]
    pub fn stop(this: &Recorder) -> Promise;
    #[wasm_bindgen(method)]
    pub fn blob(this: &Recorder) -> Blob;

    #[wasm_bindgen(js_name = makeTicker)]
    pub fn make_ticker(f: &Closure<dyn FnMut()>, ms: f64) -> js_sys::Function;

    #[wasm_bindgen(js_name = canShareFile)]
    pub fn can_share_file(blob: &Blob, name: &str) -> bool;
    #[wasm_bindgen(js_name = shareFile)]
    pub fn share_file(blob: &Blob, name: &str) -> Promise;
}
