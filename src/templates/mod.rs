//! Sistem template modular.
//!
//! Menambah template baru = 3 langkah (lihat docs/TEMPLATES.md):
//! 1. buat `src/templates/nama.rs` yang mengimplementasikan [`Template`],
//! 2. tambahkan satu baris di makro `register_templates!` di bawah,
//! 3. (opsional) taruh aset di `web/assets/` dan daftarkan di `TemplateMeta::assets`.
//!
//! Katalog, kolom isian, kontrol latar, pratinjau, player, dan ekspor video
//! semuanya dibangun otomatis dari `TemplateMeta` dan tidak perlu diubah.

use std::rc::Rc;

use crate::geom::{Orient, Rect};
use crate::paint::Painter;
use crate::state::{BgMode, Frame, Scene};

pub mod common;

/// Kolom isian tambahan yang dimunculkan di bagian "Info lagu".
pub struct FieldSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub default: &'static str,
    pub max_len: u32,
    /// Membentang selebar baris form.
    pub full: bool,
    /// Tampilkan penghitung `n/max`.
    pub counter: bool,
    /// Orientasi tempat kolom ini dipakai; kosong = semua.
    pub orients: &'static [Orient],
}

pub enum BgPolicy {
    /// Pengguna bebas memilih latar (album / gambar sendiri / warna) dan mengatur blur + gelap.
    Free,
    /// Template memakai latar bawaan dengan blur dan gelap tetap.
    Fixed {
        portrait: &'static str,
        landscape: &'static str,
        blur: f64,
        dim: f64,
        note: &'static str,
    },
}

pub struct Asset {
    pub key: &'static str,
    /// URL relatif terhadap index.html.
    pub url: &'static str,
}

pub struct TemplateMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Label kecil di kartu katalog.
    pub tags: &'static [&'static str],
    pub fields: &'static [FieldSpec],
    pub bg: BgPolicy,
    /// Catatan di bawah bagian "Foto album" (mis. foto dipakai sebagai foto profil).
    pub art_note: Option<&'static str>,
    pub assets: &'static [Asset],
}

impl TemplateMeta {
    pub fn fields_for(&self, o: Orient) -> impl Iterator<Item = &FieldSpec> {
        self.fields
            .iter()
            .filter(move |f| f.orients.is_empty() || f.orients.contains(&o))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Geser posisi lagu; pecahan diambil dari posisi x di dalam `rect`.
    Seek,
    /// Atur volume dengan cara yang sama.
    Volume,
    PlayPause,
    /// Maju/mundur sekian detik.
    Skip(f64),
}

/// Area sentuh/klik pada kartu, dalam satuan U.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
}

pub trait Template {
    fn meta(&self) -> &TemplateMeta;

    /// Lapisan yang tidak berubah antar-frame (latar, gambar, ikon tetap). Di-cache oleh renderer
    /// dan digambar ulang hanya ketika `Scene::rev`, ukuran, atau orientasi berubah.
    fn draw_static(&self, p: &mut dyn Painter, s: &Scene, o: Orient);

    /// Lapisan yang berubah tiap frame (progres, waktu, teks bergeser, spektrum, ikon putar/jeda).
    fn draw_dynamic(&self, p: &mut dyn Painter, s: &Scene, f: &Frame, o: Orient);

    /// Area interaktif untuk tampilan player.
    fn hits(&self, s: &Scene, o: Orient) -> Vec<Hit>;
}

/// Daftarkan template di sini. Satu baris per template: `nama_modul => ekspresi_konstruktor`.
macro_rules! register_templates {
    ($($m:ident => $ctor:expr),+ $(,)?) => {
        $(pub mod $m;)+

        /// Semua template, urutan = urutan di katalog (yang pertama jadi bawaan).
        pub fn registry() -> Vec<Rc<dyn Template>> {
            vec![$(Rc::new($ctor) as Rc<dyn Template>),+]
        }
    };
}

register_templates! {
    android => android::Android,
    ios => ios::Ios,
    ig => ig::Ig::new(),
}

pub fn find(reg: &[Rc<dyn Template>], id: &str) -> Option<Rc<dyn Template>> {
    reg.iter().find(|t| t.meta().id == id).cloned()
}

/// Apakah template ini membiarkan pengguna mengatur latar?
pub fn bg_is_free(m: &TemplateMeta) -> bool {
    matches!(m.bg, BgPolicy::Free)
}

/// Mode latar efektif; template dengan latar tetap selalu dianggap `Album` (tidak dipakai).
pub fn effective_bg_mode(m: &TemplateMeta, s: &Scene) -> BgMode {
    if bg_is_free(m) {
        s.bg_mode
    } else {
        BgMode::Album
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::mock::Mock;

    #[test]
    fn ids_are_unique_and_metadata_complete() {
        let reg = registry();
        assert!(reg.len() >= 3);
        let mut ids: Vec<_> = reg.iter().map(|t| t.meta().id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), reg.len(), "id template harus unik");
        for t in &reg {
            let m = t.meta();
            assert!(!m.name.is_empty() && !m.description.is_empty(), "{}", m.id);
            for f in m.fields {
                assert!(!f.id.is_empty() && f.max_len > 0, "{}.{}", m.id, f.id);
            }
        }
    }

    #[test]
    fn every_template_draws_in_both_orientations_without_panic() {
        let mut scene = Scene::new();
        for t in registry() {
            for f in t.meta().fields {
                scene.fields.insert(f.id.into(), f.default.into());
            }
        }
        scene.title = "Judul yang cukup panjang supaya marquee aktif di kartu ini".into();
        scene.artist = "Artis".into();
        scene
            .fields
            .insert("caption".into(), "Halo dunia dari kartu musik".into());
        for t in registry() {
            for o in Orient::ALL {
                let (w, h) = o.dims(540);
                let mut m = Mock::new(w as f64, h as f64);
                t.draw_static(&mut m, &scene, o);
                t.draw_dynamic(&mut m, &scene, &Frame::demo(), o);
                assert!(!m.log.is_empty(), "{} {:?}", t.meta().id, o);
                assert!(
                    m.has_text("Halo") || t.meta().id != "ig",
                    "caption IG harus tergambar"
                );
            }
        }
    }

    #[test]
    fn hits_stay_inside_the_card_and_include_play_pause() {
        let scene = Scene::new();
        for t in registry() {
            for o in Orient::ALL {
                let hits = t.hits(&scene, o);
                assert!(
                    hits.iter().any(|h| h.action == Action::PlayPause),
                    "{}",
                    t.meta().id
                );
                for h in &hits {
                    assert!(
                        h.rect.x >= -1.0 && h.rect.x + h.rect.w <= 101.0,
                        "{} x",
                        t.meta().id
                    );
                    assert!(
                        h.rect.y >= -1.0 && h.rect.y + h.rect.h <= o.hh() + 1.0,
                        "{} y {:?}",
                        t.meta().id,
                        h.rect
                    );
                }
            }
        }
    }

    #[test]
    fn fields_for_filters_by_orientation() {
        let reg = registry();
        let a = find(&reg, "android").unwrap();
        let p: Vec<_> = a
            .meta()
            .fields_for(Orient::Portrait)
            .map(|f| f.id)
            .collect();
        let l: Vec<_> = a
            .meta()
            .fields_for(Orient::Landscape)
            .map(|f| f.id)
            .collect();
        assert!(p.contains(&"h1") && !l.contains(&"h1"));
    }
}
