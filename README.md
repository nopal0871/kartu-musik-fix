# Kartu Musik iOS

Buat kartu pemutar musik bergaya **Android**, **iOS**, atau **Instagram**, lalu simpan sebagai video sepanjang lagu
(MP4/WebM, lengkap dengan suara, fade, dan spektrum). Semuanya berjalan di browser; tidak ada server dan tidak ada
berkas yang diunggah.

Ditulis ulang dari satu berkas HTML menjadi **Rust + WebAssembly** dengan tampilan yang sama.

## Fitur

* Katalog template dengan pratinjau langsung memakai foto dan teks Anda.
* Portrait 9:16 dan landscape 16:9.
* Ekspor cepat: video dirender offline lewat WebCodecs, jauh lebih cepat dari durasi lagu. Cadangan otomatis ke
  perekaman real-time (MediaRecorder) di browser yang belum mendukung.
* Resolusi 540p sampai 2K, 24/30/60 fps, tiga preset kualitas, fade in/out.
* Crop foto album dan latar secara manual.

## Menjalankan

Prasyarat: Rust stable + target `wasm32-unknown-unknown`, dan `wasm-bindgen-cli` (versi harus sama dengan `Cargo.lock`).

```bash
rustup target add wasm32-unknown-unknown
INSTALL_TOOLS=1 ./scripts/build.sh --release   # memasang wasm-bindgen-cli yang cocok bila belum ada
./scripts/serve.sh                             # http://localhost:8080
```

Tes logika (tanpa browser): `cargo test`.

## Deploy ke GitHub Pages

1. Push ke branch `main`.
2. Di repo: **Settings → Pages → Build and deployment → Source: GitHub Actions**.
3. Workflow `Deploy ke GitHub Pages` membangun dan menerbitkan otomatis pada setiap push.

Semua path bersifat relatif, jadi berfungsi di `https://<user>.github.io/<repo>/`.

## Struktur

```
src/templates/   satu berkas per template + registry (lihat docs/TEMPLATES.md)
src/paint/       trait Painter + implementasi Canvas2D + Mock untuk tes
src/render.rs    renderer tunggal (pratinjau, katalog, player, ekspor) dengan cache lapisan statis
src/export/      ekspor cepat (WebCodecs) dan cadangan real-time
src/app/         perekat DOM, audio, gambar/crop
web/             index.html, style.css, bridge.js (WebCodecs/MediaRecorder/WebAudio), vendor muxer, aset
```

`web/bridge.js` adalah satu-satunya JavaScript aplikasi: jembatan ke API browser yang belum punya binding Rust stabil.
Render frame, FFT, fade, dan penjadwalan tetap di Rust.

## Browser

Ekspor cepat: Chrome/Edge/Safari terbaru (WebCodecs). Lainnya memakai perekaman real-time dan halaman harus tetap terbuka.

## Lisensi

MIT. Lihat `LICENSE` dan `THIRD_PARTY_NOTICES.md`.
