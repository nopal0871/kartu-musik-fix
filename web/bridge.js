// Jembatan tipis ke API browser yang belum punya binding Rust yang stabil
// (WebCodecs, muxer MP4/WebM, MediaRecorder, grafik WebAudio).
// Seluruh logika aplikasi (render frame, FFT, fade, jadwal) tetap ada di Rust.

const W = globalThis;

/* ---------- Font ---------- */
export function preloadFonts() {
  try {
    if (!document.fonts || !document.fonts.load) return Promise.resolve();
    return Promise.all([
      document.fonts.load('400 40px Inter'),
      document.fonts.load('500 40px Inter'),
      document.fonts.load('600 40px Inter'),
    ]).then(() => undefined, () => undefined);
  } catch (_) { return Promise.resolve(); }
}
export function onFontsChange(cb) {
  try {
    if (!document.fonts) return;
    document.fonts.addEventListener('loadingdone', () => cb());
    document.fonts.ready.then(() => cb());
  } catch (_) { /* tidak fatal */ }
}

/* ---------- Utilitas ---------- */
export function yieldUi() {
  return new Promise((res) => { const c = new MessageChannel(); c.port1.onmessage = () => res(); c.port2.postMessage(0); });
}
export function hasWebCodecs() {
  return !!(W.VideoEncoder && W.AudioEncoder && W.VideoFrame && W.AudioData && W.Mp4Muxer && W.WebMMuxer);
}
export function hasRecorder() {
  return !!(W.MediaRecorder && HTMLCanvasElement.prototype.captureStream);
}
export function requestWakeLock() {
  try { if (navigator.wakeLock) return navigator.wakeLock.request('screen').catch(() => null); } catch (_) { /* abaikan */ }
  return Promise.resolve(null);
}
export function releaseWakeLock(l) { try { if (l) l.release(); } catch (_) { /* abaikan */ } }

/* ---------- Audio: dekode berkas ---------- */
export function decodeSong(file) {
  return file.arrayBuffer().then((ab) => {
    const OC = W.OfflineAudioContext || W.webkitOfflineAudioContext;
    const oc = new OC(2, 1, 48000);
    return new Promise((res, rej) => oc.decodeAudioData(ab, res, rej));
  });
}
export function songChannels(buf) { return buf.numberOfChannels; }
export function songLength(buf) { return buf.length; }
export function songRate(buf) { return buf.sampleRate; }
export function songChannel(buf, i) { return buf.getChannelData(Math.min(i, buf.numberOfChannels - 1)); }

/* ---------- Audio: grafik pemutaran + analyser + rekam ---------- */
export class AudioGraph {
  constructor(audio, vol) {
    const AC = W.AudioContext || W.webkitAudioContext;
    if (!AC) throw new Error('no AudioContext');
    const ctx = new AC();
    const src = ctx.createMediaElementSource(audio);
    const spk = ctx.createGain(); spk.gain.value = vol * 0.95;
    const rg = ctx.createGain(); rg.gain.value = 0.95;
    const dest = ctx.createMediaStreamDestination();
    const an = ctx.createAnalyser(); an.fftSize = 512; an.smoothingTimeConstant = 0.55;
    src.connect(spk); spk.connect(ctx.destination);
    src.connect(rg); rg.connect(dest);
    src.connect(an);
    audio.volume = 1;
    this.ctx = ctx; this.spk = spk; this.rg = rg; this.dest = dest; this.an = an;
  }
  resume() { return this.ctx.resume().catch(() => undefined); }
  setVolume(v) { this.spk.gain.value = v * 0.95; }
  setRecGain(v) { this.rg.gain.value = v; }
  bytes(out) { this.an.getByteFrequencyData(out); }
  audioTracks() { return this.dest.stream.getAudioTracks(); }
}

/* ---------- Ekspor cepat: WebCodecs + muxer ---------- */
const VLIST = [
  { cont: 'mp4', codec: 'avc1.640033', mux: 'avc' },
  { cont: 'mp4', codec: 'avc1.4d0033', mux: 'avc' },
  { cont: 'mp4', codec: 'avc1.42003e', mux: 'avc' },
  { cont: 'webm', codec: 'vp09.00.51.08', mux: 'V_VP9' },
  { cont: 'webm', codec: 'vp8', mux: 'V_VP8' },
];

export async function pickCodecs(width, height, fps, bitrate) {
  if (!hasWebCodecs()) return null;
  for (const v of VLIST) {
    let vr = null, vc = null;
    for (const hw of ['prefer-hardware', 'no-preference']) {
      vc = { codec: v.codec, width, height, bitrate, framerate: fps,
        latencyMode: v.cont === 'mp4' ? 'quality' : 'realtime', hardwareAcceleration: hw };
      if (v.cont === 'mp4') vc.avc = { format: 'avc' };
      try { vr = await VideoEncoder.isConfigSupported(vc); } catch (_) { vr = null; }
      if (vr && vr.supported) break;
    }
    if (!vr || !vr.supported) continue;
    const alist = v.cont === 'mp4'
      ? [{ codec: 'mp4a.40.2', mux: 'aac' }, { codec: 'opus', mux: 'opus' }]
      : [{ codec: 'opus', mux: 'A_OPUS' }];
    for (const a of alist) {
      const ac = { codec: a.codec, sampleRate: 48000, numberOfChannels: 2, bitrate: 192000 };
      let ar;
      try { ar = await AudioEncoder.isConfigSupported(ac); } catch (_) { continue; }
      if (ar && ar.supported) return { v, a, vcfg: vr.config || vc, acfg: ar.config || ac, width, height, fps };
    }
  }
  return null;
}

export class Encoder {
  constructor(pick, canvas) {
    this.canvas = canvas; this.err = null; this.ext = pick.v.cont === 'mp4' ? 'mp4' : 'webm';
    const { width, height, fps } = pick;
    if (pick.v.cont === 'mp4') {
      this.target = new W.Mp4Muxer.ArrayBufferTarget();
      this.muxer = new W.Mp4Muxer.Muxer({ target: this.target, fastStart: 'in-memory',
        firstTimestampBehavior: 'cross-track-offset',
        video: { codec: 'avc', width, height, frameRate: fps },
        audio: { codec: pick.a.mux, numberOfChannels: 2, sampleRate: 48000 } });
    } else {
      this.target = new W.WebMMuxer.ArrayBufferTarget();
      this.muxer = new W.WebMMuxer.Muxer({ target: this.target, firstTimestampBehavior: 'permissive',
        video: { codec: pick.v.mux, width, height, frameRate: fps },
        audio: { codec: 'A_OPUS', numberOfChannels: 2, sampleRate: 48000 } });
    }
    this.venc = new VideoEncoder({ output: (c, m) => this.muxer.addVideoChunk(c, m), error: (e) => { this.err = e; } });
    this.venc.configure(pick.vcfg);
    this.aenc = new AudioEncoder({ output: (c, m) => this.muxer.addAudioChunk(c, m), error: (e) => { this.err = e; } });
    this.aenc.configure(pick.acfg);
    this.frameDur = Math.round(1e6 / fps);
  }
  extension() { return this.ext; }
  error() { return this.err ? String(this.err.message || this.err) : ''; }
  queueSize() { return this.venc.encodeQueueSize; }
  addFrame(tsUs, key) {
    const vf = new VideoFrame(this.canvas, { timestamp: tsUs, duration: this.frameDur });
    this.venc.encode(vf, { keyFrame: key });
    vf.close();
  }
  // data: Float32Array planar (L lalu R), n = jumlah frame per kanal
  addAudio(data, n, tsUs) {
    const ad = new AudioData({ format: 'f32-planar', sampleRate: 48000, numberOfFrames: n, numberOfChannels: 2, timestamp: tsUs, data });
    this.aenc.encode(ad); ad.close();
  }
  async finish() {
    await this.venc.flush(); await this.aenc.flush();
    if (this.err) throw this.err;
    this.muxer.finalize();
    return new Blob([this.target.buffer], { type: this.ext === 'mp4' ? 'video/mp4' : 'video/webm' });
  }
  close() {
    try { if (this.venc && this.venc.state !== 'closed') this.venc.close(); } catch (_) { /* abaikan */ }
    try { if (this.aenc && this.aenc.state !== 'closed') this.aenc.close(); } catch (_) { /* abaikan */ }
  }
}

/* ---------- Cadangan: rekam real-time lewat MediaRecorder ---------- */
const MIMES = [
  'video/mp4;codecs=avc1.42E01E,mp4a.40.2', 'video/mp4;codecs=avc1,mp4a.40.2', 'video/mp4',
  'video/webm;codecs=vp9,opus', 'video/webm;codecs=vp8,opus', 'video/webm',
];
export function pickMime() {
  if (!hasRecorder()) return null;
  for (const m of MIMES) { try { if (MediaRecorder.isTypeSupported(m)) return m; } catch (_) { /* lanjut */ } }
  return '';
}

export class Recorder {
  constructor(canvas, fps, graph, bitrate, mime) {
    const stream = canvas.captureStream(fps);
    this.hasAudio = false;
    if (graph) { graph.audioTracks().forEach((t) => stream.addTrack(t)); this.hasAudio = true; }
    const opts = { videoBitsPerSecond: bitrate };
    if (mime) opts.mimeType = mime;
    try { this.mr = new MediaRecorder(stream, opts); } catch (_) { this.mr = new MediaRecorder(stream); }
    this.chunks = [];
    this.mr.ondataavailable = (e) => { if (e.data && e.data.size) this.chunks.push(e.data); };
    this.mime = mime;
    this.stopped = new Promise((res) => { this.mr.onstop = () => res(); });
    this.mr.onerror = () => { this.failed = true; };
  }
  start() { this.mr.start(1000); }
  stop() { try { this.mr.stop(); } catch (_) { /* sudah berhenti */ } return this.stopped; }
  failed_() { return !!this.failed; }
  blob() {
    const type = this.mr.mimeType || this.mime || 'video/webm';
    return new Blob(this.chunks, { type });
  }
}

/* Pemicu frame lewat Worker agar tetap jalan saat tab di latar; cadangan: requestAnimationFrame. */
export function makeTicker(fn, ms) {
  let w = null, off = false, r = 0;
  const useRaf = () => { const loop = () => { if (off) return; fn(); r = requestAnimationFrame(loop); }; loop(); };
  try {
    const u = URL.createObjectURL(new Blob(['var id=0;onmessage=function(e){clearInterval(id);if(e.data>0)id=setInterval(function(){postMessage(0)},e.data)}'], { type: 'text/javascript' }));
    w = new Worker(u);
    w.onmessage = () => { if (!off) fn(); };
    w.onerror = () => { if (w) { try { w.terminate(); } catch (_) { /* abaikan */ } w = null; if (!off) useRaf(); } };
    w.postMessage(ms);
  } catch (_) { w = null; useRaf(); }
  return () => { off = true; cancelAnimationFrame(r); if (w) { try { w.postMessage(0); w.terminate(); } catch (_) { /* abaikan */ } w = null; } };
}

/* ---------- Simpan / bagikan ---------- */
export function canShareFile(blob, name) {
  try { return !!(navigator.canShare && navigator.canShare({ files: [new File([blob], name, { type: blob.type })] })); } catch (_) { return false; }
}
export function shareFile(blob, name) {
  return navigator.share({ files: [new File([blob], name, { type: blob.type })] });
}
