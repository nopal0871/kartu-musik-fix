//! Template "Pemutar Android": cover album besar, tombol Putar lebar, deretan tombol kecil.

use super::common::{draw_background, font, hit_box};
use super::{Action, FieldSpec, Hit, Template, TemplateMeta};
use crate::geom::{fmt_time, Align, Orient, RRect, Rect};
use crate::icons::{draw_icon, Icon};
use crate::marquee;
use crate::paint::Painter;
use crate::state::{Frame, Scene};

pub struct Android;

static META: TemplateMeta = TemplateMeta {
    id: "android",
    name: "Pemutar Android",
    description: "Cover album besar, tombol Putar lebar, dan deretan tombol kecil.",
    tags: &["Cover besar", "Latar bebas"],
    fields: &[
        FieldSpec { id: "h1", label: "Label atas", placeholder: "Sedang Diputar", default: "Sedang Diputar", max_len: 40, full: false, counter: false, orients: &[Orient::Portrait] },
        FieldSpec { id: "h2", label: "Nama playlist", placeholder: "Mix Ropang", default: "Mix Ropang", max_len: 40, full: false, counter: false, orients: &[Orient::Portrait] },
    ],
    bg: super::BgPolicy::Free,
    art_note: None,
    assets: &[],
};

struct Btn {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    ic: f64,
    rad: [f64; 4],
}
struct Prog {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    kw: f64,
    kh: f64,
    g1: f64,
    g2: f64,
    dd: f64,
    d_in: f64,
}
struct Seg {
    x: f64,
    w: f64,
}
struct Row {
    y: f64,
    h: f64,
    rw: Seg,
    pp: Seg,
    ff: Seg,
    ic: f64,
    pic: f64,
    pgap: f64,
    fs: f64,
}
struct Mini {
    y: f64,
    x: f64,
    w: f64,
    gap: f64,
    big: f64,
    small: f64,
    ic: f64,
    ow: f64,
}
struct Lay {
    /// (y, fs) label atas; hanya portrait.
    h1: Option<(f64, f64)>,
    h2: (f64, f64),
    art: (f64, f64, f64, f64),
    tx: f64,
    mq_w: f64,
    ttl: (f64, f64),
    sub: (f64, f64),
    share: Btn,
    heart: Btn,
    prog: Prog,
    time: (f64, f64, f64, f64),
    row: Row,
    mini: Mini,
    more: (f64, f64, f64, f64),
}

fn lay(o: Orient) -> Lay {
    match o {
        Orient::Portrait => Lay {
            h1: Some((9.2, 2.7)),
            h2: (13.4, 2.7),
            art: (4.72, 22.0, 90.56, 1.3),
            tx: 4.8,
            mq_w: 74.6,
            ttl: (122.4, 3.3),
            sub: (127.0, 2.7),
            share: Btn { x: 81.94, y: 124.2, w: 6.2, h: 6.2, ic: 4.2, rad: [3.1, 0.9, 0.9, 3.1] },
            heart: Btn { x: 88.5, y: 124.2, w: 6.2, h: 6.2, ic: 4.0, rad: [0.9; 4] },
            prog: Prog { x: 5.0, y: 135.6, w: 90.0, h: 2.4, kw: 0.6, kh: 6.4, g1: 1.1, g2: 1.2, dd: 0.7, d_in: 1.3 },
            time: (141.0, 2.0, 5.4, 94.6),
            row: Row { y: 150.7, h: 9.9, rw: Seg { x: 4.72, w: 18.0 }, pp: Seg { x: 23.9, w: 52.1 }, ff: Seg { x: 77.2, w: 18.1 }, ic: 5.2, pic: 4.4, pgap: 1.2, fs: 2.5 },
            mini: Mini { y: 165.0, x: 4.45, w: 6.1, gap: 1.0, big: 3.05, small: 0.45, ic: 3.6, ow: 0.16 },
            more: (92.4, 165.0, 6.2, 4.1),
        },
        Orient::Landscape => Lay {
            h1: None,
            h2: (0.0, 0.0),
            art: (7.3, 9.0, 36.35, 0.8),
            tx: 55.1,
            mq_w: 33.4,
            ttl: (18.5, 1.86),
            sub: (20.8, 1.52),
            share: Btn { x: 89.86, y: 19.5, w: 3.44, h: 3.44, ic: 2.36, rad: [1.72, 0.5, 0.5, 1.72] },
            heart: Btn { x: 93.86, y: 19.5, w: 3.44, h: 3.44, ic: 2.25, rad: [0.5; 4] },
            prog: Prog { x: 55.1, y: 25.7, w: 42.2, h: 1.35, kw: 0.34, kh: 3.6, g1: 0.62, g2: 0.68, dd: 0.4, d_in: 0.73 },
            time: (28.7, 1.13, 55.3, 97.1),
            row: Row { y: 34.2, h: 5.6, rw: Seg { x: 55.1, w: 8.45 }, pp: Seg { x: 64.28, w: 23.84 }, ff: Seg { x: 88.85, w: 8.45 }, ic: 2.92, pic: 2.48, pgap: 0.68, fs: 1.4 },
            mini: Mini { y: 49.1, x: 2.5, w: 3.43, gap: 0.56, big: 1.72, small: 0.25, ic: 2.02, ow: 0.09 },
            more: (95.8, 49.1, 3.5, 2.3),
        },
    }
}

const MINI_ICONS: [Icon; 5] = [Icon::Queue, Icon::Moon, Icon::Shuffle, Icon::Lyrics, Icon::Repeat];

impl Template for Android {
    fn meta(&self) -> &TemplateMeta {
        &META
    }

    fn draw_static(&self, p: &mut dyn Painter, s: &Scene, o: Orient) {
        let l = lay(o);
        let u = p.unit();
        let (w, _) = p.size();
        draw_background(p, s, &META, o, 0.0);

        if let Some((y, fs)) = l.h1 {
            p.text(s.field("h1").trim(), w / 2.0, y * u, &font(400, fs * u), "rgba(255,255,255,.92)", Align::Center);
            p.text(s.field("h2").trim(), w / 2.0, l.h2.0 * u, &font(400, l.h2.1 * u), "rgba(255,255,255,.78)", Align::Center);
        }

        let (ax, ay, asz, ar) = l.art;
        p.save();
        p.clip_rrect(RRect::new(ax * u, ay * u, asz * u, asz * u, ar * u));
        p.fill_rect(ax * u, ay * u, asz * u, asz * u, "#444");
        if let Some(img) = &s.art {
            p.image_cover(img, ax * u, ay * u, asz * u, asz * u);
        }
        p.restore();

        for (b, ic) in [(&l.share, Icon::Share), (&l.heart, Icon::Heart)] {
            let r = RRect::with_radii(b.x * u, (b.y - b.h / 2.0) * u, b.w * u, b.h * u, b.rad.map(|v| v * u));
            p.fill_rrect(r, "#fff");
            draw_icon(p, ic, (b.x + b.w / 2.0) * u, b.y * u, b.ic * u, "#15181d");
        }

        let r = &l.row;
        for (seg, ic) in [(&r.rw, Icon::SkipPrev), (&r.ff, Icon::SkipNext)] {
            p.fill_rrect(RRect::new(seg.x * u, (r.y - r.h / 2.0) * u, seg.w * u, r.h * u, r.h * u / 2.0), "rgba(255,255,255,.2)");
            draw_icon(p, ic, (seg.x + seg.w / 2.0) * u, r.y * u, r.ic * u, "#fff");
        }

        let m = &l.mini;
        for (i, ic) in MINI_ICONS.iter().enumerate() {
            let rad = match i {
                0 => [m.big, m.small, m.small, m.big],
                4 => [m.small, m.big, m.big, m.small],
                _ => [m.small; 4],
            };
            let (x, y, side, ow) = ((m.x + i as f64 * (m.w + m.gap)) * u, (m.y - m.w / 2.0) * u, m.w * u, m.ow * u);
            let rr = RRect::with_radii(x + ow / 2.0, y + ow / 2.0, side - ow, side - ow, rad.map(|v| (v * u - ow / 2.0).max(0.0)));
            p.stroke_rrect(rr, ow, "rgba(255,255,255,.6)");
            draw_icon(p, *ic, x + side / 2.0, m.y * u, m.ic * u, "#fff");
        }

        let (mx, my, md, mic) = l.more;
        p.fill_circle(mx * u, my * u, md * u / 2.0, "#fff");
        draw_icon(p, Icon::Dots, mx * u, my * u, mic * u, "#15181d");
    }

    fn draw_dynamic(&self, p: &mut dyn Painter, s: &Scene, f: &Frame, o: Orient) {
        let l = lay(o);
        let u = p.unit();

        let (title, artist) = (s.disp_title(), s.disp_artist());
        let (tf, sf) = (font(600, l.ttl.1 * u), font(400, l.sub.1 * u));
        let (tw, sw) = (p.measure(title, &tf), p.measure(artist, &sf));
        marquee::draw(p, l.tx * u, l.ttl.0 * u, l.mq_w * u, l.ttl.1 * u * 1.6, tw, f.t, &mut |lp, ox, cy| {
            lp.text(title, ox, cy, &tf, "#fff", Align::Left)
        });
        marquee::draw(p, l.tx * u, l.sub.0 * u, l.mq_w * u, l.sub.1 * u * 1.6, sw, f.t, &mut |lp, ox, cy| {
            lp.text(artist, ox, cy, &sf, "rgba(255,255,255,.82)", Align::Left)
        });

        // Progres: bagian terisi, celah, trek sisa, kenop, titik akhir.
        let pr = &l.prog;
        let frac = f.progress();
        let x = frac * pr.w;
        let tl = pr.w.min(x + pr.g2);
        let fw = (x - pr.g1).max(0.0);
        if fw > 0.0 {
            p.fill_rrect(RRect::new(pr.x * u, (pr.y - pr.h / 2.0) * u, fw * u, pr.h * u, (pr.h * u / 2.0).min(fw * u / 2.0)), "rgba(255,255,255,.97)");
        }
        if pr.w - tl > 0.0 {
            let tw_ = (pr.w - tl) * u;
            p.fill_rrect(RRect::new((pr.x + tl) * u, (pr.y - pr.h / 2.0) * u, tw_, pr.h * u, (pr.h * u / 2.0).min(tw_ / 2.0)), "rgba(255,255,255,.45)");
        }
        p.fill_rrect(RRect::new((pr.x + x - pr.kw / 2.0) * u, (pr.y - pr.kh / 2.0) * u, pr.kw * u, pr.kh * u, pr.kw * u / 2.0), "#fff");
        p.fill_circle((pr.x + pr.w - pr.d_in) * u, pr.y * u, pr.dd * u / 2.0, "#fff");

        let (ty, tfs, lx, rx) = l.time;
        let timef = font(500, tfs * u);
        p.text(&fmt_time(f.cur.min(f.dur.max(f.cur))), lx * u, ty * u, &timef, "rgba(255,255,255,.9)", Align::Left);
        p.text(&fmt_time(f.dur), rx * u, ty * u, &timef, "rgba(255,255,255,.9)", Align::Right);

        // Tombol Putar/Jeda lebar
        let r = &l.row;
        p.fill_rrect(RRect::new(r.pp.x * u, (r.y - r.h / 2.0) * u, r.pp.w * u, r.h * u, r.h * u / 2.0), "rgba(255,255,255,.97)");
        let label = if f.playing { "Jeda" } else { "Putar" };
        let lf = font(500, r.fs * u);
        let lw = p.measure(label, &lf);
        let iw = r.pic * u * 13.0 / 24.0;
        let total = iw + r.pgap * u + lw;
        let x0 = (r.pp.x + r.pp.w / 2.0) * u - total / 2.0;
        draw_icon(p, if f.playing { Icon::PauseL } else { Icon::PlayL }, x0 + iw / 2.0, r.y * u, r.pic * u, "#14171c");
        p.text(label, x0 + iw + r.pgap * u, r.y * u, &lf, "#14171c", Align::Left);
    }

    fn hits(&self, _s: &Scene, o: Orient) -> Vec<Hit> {
        let l = lay(o);
        let (pr, r) = (&l.prog, &l.row);
        let hh = pr.kh + 1.5;
        let seg = |s: &Seg| Rect::new(s.x, r.y - r.h / 2.0, s.w, r.h);
        vec![
            Hit { rect: hit_box(pr.x + pr.w / 2.0, pr.y, pr.w, hh), action: Action::Seek },
            Hit { rect: seg(&r.pp), action: Action::PlayPause },
            Hit { rect: seg(&r.rw), action: Action::Skip(-10.0) },
            Hit { rect: seg(&r.ff), action: Action::Skip(10.0) },
        ]
    }
}
