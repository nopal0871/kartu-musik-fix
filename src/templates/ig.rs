//! Template "IG": kartu catatan musik ala Instagram dengan latar tetap dan caption.

use std::cell::RefCell;

use super::common::{draw_background, font};
use super::{Action, Asset, BgPolicy, FieldSpec, Hit, Template, TemplateMeta};
use crate::geom::{Align, Orient, RRect, Rect};
use crate::marquee;
use crate::paint::Painter;
use crate::state::{Frame, Scene};

const CARD: &str = "#1b1e23";
const BLUE: &str = "#4f5bf3";
const LINK: &str = "#7f8dff";
const CAP_MAX: u32 = 50;
const META_TXT: &str = "Dibagikan kepada teman · Sekarang";
const BTN_TXT: &str = "Tinggalkan catatan baru";
const DEL_TXT: &str = "Hapus catatan";

static META: TemplateMeta = TemplateMeta {
    id: "ig",
    name: "Catatan IG",
    description: "Kartu catatan musik ala Instagram dengan foto profil dan caption.",
    tags: &["Latar tetap", "Caption"],
    fields: &[FieldSpec {
        id: "caption",
        label: "Caption catatan",
        placeholder: "Tulis caption (maks. 50 karakter)",
        default: "",
        max_len: CAP_MAX,
        full: true,
        counter: true,
        orients: &[],
    }],
    bg: BgPolicy::Fixed {
        portrait: "ig-bg-portrait",
        landscape: "ig-bg-landscape",
        blur: 90.0,
        dim: 12.0,
        note: "Gaya IG: blur latar dikunci di 90% dan latar digelapkan otomatis, jadi pengaturan latar tidak bisa diubah.",
    },
    art_note: Some("Gaya IG: foto ini dipakai sebagai foto profil. Sebelum kamu memilih foto, yang tampil adalah siluet bawaan."),
    assets: &[
        Asset { key: "ig-bg-portrait", url: "assets/ig-bg-portrait.jpg" },
        Asset { key: "ig-bg-landscape", url: "assets/ig-bg-landscape.jpg" },
    ],
};

/// Tata letak dihitung dari teks (caption berbaris-baris, judul), semua dalam satuan U.
#[derive(Clone, Debug)]
struct Geom {
    card: RRect,
    av: (f64, f64, f64),
    lines: Vec<String>,
    cap_fs: f64,
    cap_cy: Vec<f64>,
    meta_fs: f64,
    meta_cy: f64,
    div: (f64, f64, f64),
    btn: (f64, f64, f64, f64, f64, f64),
    del_fs: f64,
    del_cy: f64,
    ib: (f64, f64, f64, f64),
    tl: TitleLine,
}

#[derive(Clone, Debug)]
struct TitleLine {
    fs: f64,
    cy: f64,
    tw: f64,
    w1: f64,
    t1: String,
    t2: String,
    icon_x: f64,
    text_x: f64,
    box_w: f64,
}

pub struct Ig {
    cache: RefCell<Option<(String, Geom)>>,
}

impl Ig {
    pub fn new() -> Ig {
        Ig { cache: RefCell::new(None) }
    }

    fn em(p: &mut dyn Painter, text: &str, wt: u32) -> f64 {
        p.measure(text, &font(wt, 100.0)) / 100.0
    }

    fn wrap(p: &mut dyn Painter, text: &str, wt: u32, max_em: f64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut line = String::new();
        let norm: Vec<&str> = text.split_whitespace().collect();
        for tk in norm {
            let test = if line.is_empty() { tk.to_string() } else { format!("{line} {tk}") };
            if Self::em(p, &test, wt) <= max_em {
                line = test;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            let mut part = String::new();
            for ch in tk.chars() {
                let mut cand = part.clone();
                cand.push(ch);
                if Self::em(p, &cand, wt) <= max_em {
                    part = cand;
                } else {
                    if !part.is_empty() {
                        out.push(std::mem::take(&mut part));
                    }
                    part.push(ch);
                }
            }
            line = part;
        }
        if !line.is_empty() {
            out.push(line);
        }
        out.truncate(3);
        out
    }

    fn geom(&self, p: &mut dyn Painter, s: &Scene, o: Orient) -> Geom {
        let cap = s.field("caption").trim().to_string();
        let (ti, ar) = (s.disp_title().to_string(), s.disp_artist().to_string());
        let key = format!("{:?}\u{1}{}\u{1}{cap}\u{1}{ti}\u{1}{ar}", o, s.rev);
        if let Some((k, g)) = &*self.cache.borrow() {
            if *k == key {
                return g.clone();
            }
        }
        let g = Self::compute(p, o, &cap, &ti, &ar);
        *self.cache.borrow_mut() = Some((key, g.clone()));
        g
    }

    fn compute(p: &mut dyn Painter, o: Orient, cap: &str, ti: &str, ar: &str) -> Geom {
        let cw = if o.is_landscape() { 36.0 } else { 62.0 };
        let hh = o.hh();
        let cap_fs = 0.039;
        let lines = if cap.is_empty() { Vec::new() } else { Self::wrap(p, cap, 600, 0.84 / cap_fs) };
        let n = lines.len();
        let title_c = 0.240;
        let mut caps = Vec::new();
        let mut c = title_c;
        for i in 0..n {
            c += if i == 0 { 0.069 } else { 0.057 };
            caps.push(c);
        }
        let meta_c = if n > 0 { caps[n - 1] + 0.055 } else { title_c + 0.066 };
        let dv = meta_c + 0.043;
        let bt = dv + 0.034;
        let bh = 0.118;
        let del_c = bt + bh + 0.079;
        let total_h = del_c + 0.141;
        let x0 = (100.0 - cw) / 2.0;
        let y0 = (hh - total_h * cw) / 2.0;
        let yy = |v: f64| y0 + v * cw;
        let ib_w = 0.026 * cw;
        let ib_h = 0.027 * cw;
        let ib_bw = 0.0062 * cw;
        let ib_gap = (ib_w - 3.0 * ib_bw) / 2.0;

        let fs = 0.039 * cw;
        let gap_i = 0.014 * cw;
        let avail = 0.844 * cw - ib_w - gap_i;
        let w1 = Self::em(p, ti, 600) * fs;
        let t2 = format!(" · {ar}");
        let w2 = Self::em(p, &t2, 400) * fs;
        let tw = w1 + w2;
        let over = tw > avail + 0.5;
        let cy = yy(title_c);
        let (icon_x, text_x, box_w) = if over {
            let ix = 50.0 - 0.422 * cw;
            (ix, ix + ib_w + gap_i, avail)
        } else {
            let gw = ib_w + gap_i + tw;
            let ix = 50.0 - gw / 2.0;
            (ix, ix + ib_w + gap_i, tw + 1.2)
        };
        Geom {
            card: RRect::new(x0, y0, cw, total_h * cw, 0.093 * cw),
            av: (50.0, yy(0.0315 + 0.075), 0.15 * cw),
            cap_fs: cap_fs * cw,
            cap_cy: caps.iter().map(|v| yy(*v)).collect(),
            lines,
            meta_fs: 0.033 * cw,
            meta_cy: yy(meta_c),
            div: (x0 + 0.034 * cw, cw * (1.0 - 0.068), yy(dv)),
            btn: (x0 + 0.078 * cw, yy(bt), 0.844 * cw, bh * cw, 0.022 * cw, 0.039 * cw),
            del_fs: 0.039 * cw,
            del_cy: yy(del_c),
            ib: (ib_w, ib_h, ib_bw, ib_gap),
            tl: TitleLine { fs, cy, tw, w1, t1: ti.to_string(), t2, icon_x, text_x, box_w },
        }
    }
}

impl Default for Ig {
    fn default() -> Self {
        Self::new()
    }
}

fn centered(p: &mut dyn Painter, text: &str, f: &str, color: &str, y_u: f64) {
    let u = p.unit();
    p.text(text, 50.0 * u, y_u * u, f, color, Align::Center);
}

impl Template for Ig {
    fn meta(&self) -> &TemplateMeta {
        &META
    }

    fn draw_static(&self, p: &mut dyn Painter, s: &Scene, o: Orient) {
        let g = self.geom(p, s, o);
        let u = p.unit();
        draw_background(p, s, &META, o, 0.0);

        let c = g.card;
        p.fill_rrect(RRect::new(c.x * u, c.y * u, c.w * u, c.h * u, c.r[0] * u), CARD);

        let (acx, acy, ad) = g.av;
        let (ax, ay, adp) = ((acx - ad / 2.0) * u, (acy - ad / 2.0) * u, ad * u);
        p.save();
        p.clip_rrect(RRect::new(ax, ay, adp, adp, adp / 2.0));
        p.fill_rect(ax, ay, adp, adp, "#eceef2");
        match (&s.art, s.art_custom) {
            (Some(img), true) => p.image_cover(img, ax, ay, adp, adp),
            _ => {
                p.fill_circle(ax + adp * 0.5, ay + adp * 0.40, adp * 0.18, "#8e9199");
                p.fill_ellipse(ax + adp * 0.5, ay + adp, adp * 0.36, adp * 0.30, "#8e9199");
            }
        }
        p.restore();

        let cf = font(600, g.cap_fs * u);
        for (ln, cy) in g.lines.iter().zip(&g.cap_cy) {
            centered(p, ln, &cf, "#fff", *cy);
        }
        centered(p, META_TXT, &font(600, g.meta_fs * u), "rgba(255,255,255,.78)", g.meta_cy);
        let (dx, dw, dy) = g.div;
        p.fill_rect(dx * u, dy * u, dw * u, (0.12 * u).max(1.0), "rgba(255,255,255,.11)");
        let (bx, by, bw, bh, br, bfs) = g.btn;
        p.fill_rrect(RRect::new(bx * u, by * u, bw * u, bh * u, br * u), BLUE);
        centered(p, BTN_TXT, &font(500, bfs * u), "#fff", by + bh / 2.0);
        centered(p, DEL_TXT, &font(500, g.del_fs * u), LINK, g.del_cy);
    }

    fn draw_dynamic(&self, p: &mut dyn Painter, s: &Scene, f: &Frame, o: Orient) {
        let g = self.geom(p, s, o);
        let u = p.unit();
        let t = &g.tl;
        // Tiga batang berdenyut di sebelah judul
        let ph = 0.5 + 0.5 * (f.t * 6.0).sin();
        let vals = [ph, 1.0 - ph, ph];
        let (ib_w, ib_h, ib_bw, ib_gap) = g.ib;
        let _ = ib_w;
        for (i, v) in vals.iter().enumerate() {
            let bh = ib_h * u * (0.38 + 0.62 * v);
            p.fill_rrect(
                RRect::new((t.icon_x + i as f64 * (ib_bw + ib_gap)) * u, t.cy * u - bh / 2.0, ib_bw * u, bh, ib_bw * u / 2.0),
                "#fff",
            );
        }
        // Judul + artis dalam satu baris yang bisa bergeser
        let (f1, f2) = (font(600, t.fs * u), font(400, t.fs * u));
        let (w1_px, tw_px) = (t.w1 * u, t.tw * u);
        let (t1, t2) = (t.t1.clone(), t.t2.clone());
        marquee::draw(p, t.text_x * u, t.cy * u, t.box_w * u, (t.fs * u * 1.7).ceil(), tw_px, f.t, &mut |lp, ox, cy| {
            lp.text(&t1, ox, cy, &f1, "#fff", Align::Left);
            lp.text(&t2, ox + w1_px, cy, &f2, "rgba(255,255,255,.66)", Align::Left);
        });
    }

    fn hits(&self, _s: &Scene, o: Orient) -> Vec<Hit> {
        // Seluruh kartu berfungsi sebagai tombol putar/jeda.
        let cw = if o.is_landscape() { 36.0 } else { 62.0 };
        let r = Rect::new((100.0 - cw) / 2.0, 0.0, cw, o.hh());
        vec![Hit { rect: r, action: Action::PlayPause }]
    }
}
