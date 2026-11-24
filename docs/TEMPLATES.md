# Menambah template baru

Template adalah satu berkas Rust di `src/templates/`. Katalog, kolom isian, kontrol latar,
pratinjau, tampilan player, dan ekspor video dibangun otomatis dari metadata template,
jadi **tidak ada file lain yang perlu diubah** selain satu baris pendaftaran.

## 3 langkah

1. Buat `src/templates/nama.rs` (salin kerangka di bawah).
2. Daftarkan di `src/templates/mod.rs`, di dalam `register_templates!`:
   ```rust
   register_templates! {
       android => android::Android,
       ios     => ios::Ios,
       ig      => ig::Ig::new(),
       nama    => nama::Nama,        // <- satu baris ini
   }
   ```
3. `cargo test`. Tes bawaan otomatis memeriksa template baru: id unik, metadata lengkap,
   bisa digambar di portrait dan landscape, area sentuh berada di dalam kartu, dan ada tombol putar/jeda.

## Konsep

| Bagian | Fungsi |
|---|---|
| `TemplateMeta` | id, nama, deskripsi, tag katalog, kolom isian, kebijakan latar, catatan foto, aset |
| `draw_static` | Lapisan yang tidak berubah antar-frame. Di-cache; digambar ulang hanya bila pengaturan berubah |
| `draw_dynamic` | Lapisan per-frame: progres, waktu, teks bergeser, spektrum, ikon putar/jeda |
| `hits` | Area klik/sentuh di player (`Seek`, `Volume`, `PlayPause`, `Skip`) |

Satuan koordinat: **U = 1% lebar kartu**. Lebar selalu 100 U, tinggi `o.hh()` (177,8 U portrait, 56,25 U landscape).
Kalikan dengan `p.unit()` untuk piksel, sehingga tampilan sama di resolusi mana pun.

Template hanya berbicara dengan trait `Painter` (`src/paint/`), bukan Canvas langsung. Itu sebabnya tampilan bisa diuji
tanpa browser memakai `paint::mock::Mock`.

## Kerangka

```rust
use super::common::{draw_background, font, hit_box};
use super::{Action, BgPolicy, FieldSpec, Hit, Template, TemplateMeta};
use crate::geom::{Align, Orient};
use crate::paint::Painter;
use crate::state::{Frame, Scene};

pub struct Nama;

static META: TemplateMeta = TemplateMeta {
    id: "nama",                       // unik, huruf kecil
    name: "Nama Template",
    description: "Satu kalimat tentang tampilannya.",
    tags: &["Minimal"],
    fields: &[FieldSpec {             // kolom tambahan di "Info lagu" (boleh kosong: &[])
        id: "label", label: "Label", placeholder: "Contoh", default: "Halo",
        max_len: 30, full: false, counter: false,
        orients: &[],                 // kosong = semua orientasi
    }],
    bg: BgPolicy::Free,               // atau BgPolicy::Fixed { .. } untuk latar bawaan template
    art_note: None,
    assets: &[],                      // aset di web/assets/, mis. Asset { key: "bg", url: "assets/bg.jpg" }
};

impl Template for Nama {
    fn meta(&self) -> &TemplateMeta { &META }

    fn draw_static(&self, p: &mut dyn Painter, s: &Scene, o: Orient) {
        let u = p.unit();
        draw_background(p, s, &META, o, 0.0);
        p.text(s.field("label"), 50.0 * u, 10.0 * u, &font(600, 4.0 * u), "#fff", Align::Center);
    }

    fn draw_dynamic(&self, p: &mut dyn Painter, s: &Scene, f: &Frame, o: Orient) {
        let u = p.unit();
        let _ = (s, o);
        // f.progress() 0..1, f.cur / f.dur (detik), f.playing, f.spec (5 batang), f.t (animasi)
        p.fill_rect(10.0 * u, 50.0 * u, 80.0 * u * f.progress(), 1.0 * u, "#fff");
    }

    fn hits(&self, _s: &Scene, _o: Orient) -> Vec<Hit> {
        vec![Hit { rect: hit_box(50.0, 50.0, 100.0, 100.0), action: Action::PlayPause }]
    }
}
```

## Tips

* **Teks panjang:** pakai `marquee::draw` (lihat `android.rs`) agar bergeser seperti di iOS.
* **Ikon:** tambahkan path SVG di `src/icons.rs` (enum `Icon` + satu entri tabel `DEFS`, urutannya sama), lalu `draw_icon`.
* **Latar sendiri:** taruh gambar di `web/assets/`, daftarkan di `assets`, pakai `BgPolicy::Fixed { portrait, landscape, blur, dim, note }`.
* **Nilai kolom:** `s.field("id")`. Judul/artis: `s.disp_title()` dan `s.disp_artist()`.
* Jangan menyimpan keadaan per-frame di struct template; semua datang dari `Scene` dan `Frame`. Cache perhitungan mahal
  boleh (lihat `ig.rs`), asalkan kuncinya mencakup `s.rev`.
