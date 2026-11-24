//! Template "iOS Kaca": kartu kaca buram di atas latar, spektrum, kontrol ala Now Playing.

use super::common::{bar, draw_background, font, hit_box};
use super::{Action, BgPolicy, FieldSpec, Hit, Template, TemplateMeta};
use crate::geom::{fmt_time, Align, Orient, RRect};
use crate::icons::{draw_icon, Icon};
use crate::marquee;
use crate::paint::Painter;
use crate::spectrum::bar_frac;
use crate::state::{Frame, Scene};

pub struct Ios;

static META: TemplateMeta = TemplateMeta {
    id: "ios",
    name: "iOS Kaca",
    description: "Kartu kaca buram dengan spektrum, bilah volume, dan tombol perangkat.",
    tags: &["Kaca", "Spektrum"],
    fields: &[FieldSpec {
        id: "device",
        label: "Label tombol perangkat (kosongkan untuk menyembunyikan)",
        placeholder: "iPhone",
        default: "iPhone",
        max_len: 24,
        full: true,
        counter: false,
        orients: &[],
    }],
    bg: BgPolicy::Free,
    art_note: None,
    assets: &[],
};

struct Seg {
    x: f64,
    w: f64,
}
struct Ctl {
    y: f64,
    rw: Seg,
    pp: Seg,
    pp_play_w: f64,
    ff: Seg,
}
struct Lay {
    card: (f64, f64, f64, f64, f64),
    art: (f64, f64, f64, f64),
    tx: f64,
    ttl: (f64, f64),
    sub: (f64, f64),
    /// x, w, y, h, lebar batang, jumlah
    spec: (f64, f64, f64, f64, f64, usize),
    prog: (f64, f64, f64, f64, f64),
    time: (f64, f64),
    ctl: Ctl,
    vol: (f64, f64, f64, f64, (f64, f64), (f64, f64)),
    /// x, y, h, fs, pad, gap, icon
    pill: (f64, f64, f64, f64, f64, f64, f64),
    mq_w: f64,
}

fn lay(o: Orient) -> Lay {
    match o {
        Orient::Portrait => Lay {
            card: (10.0, 17.2, 80.0, 143.2, 10.5),
            art: (15.83, 23.2, 68.1, 3.9),
            tx: 15.83,
            ttl: (98.9, 3.9),
            sub: (103.8, 3.9),
            spec: (77.35, 6.6, 101.4, 4.6, 0.8, 5),
            prog: (15.83, 112.2, 68.12, 1.45, 1.9),
            time: (117.2, 3.2),
            ctl: Ctl { y: 127.9, rw: Seg { x: 29.7, w: 9.2 }, pp: Seg { x: 50.1, w: 6.4 }, pp_play_w: 7.2, ff: Seg { x: 70.3, w: 9.2 } },
            vol: (140.4, 22.2, 52.7, 1.45, (16.9, 2.4), (82.2, 4.4)),
            pill: (50.0, 150.5, 8.2, 3.6, 3.6, 1.7, 4.2),
            mq_w: 77.35 - 2.6 - 15.83,
        },
        Orient::Landscape => Lay {
            card: (5.0, 5.0, 90.0, 46.25, 4.2),
            art: (10.0, 10.0, 36.25, 2.2),
            tx: 52.5,
            ttl: (15.2, 3.1),
            sub: (19.6, 2.6),
            spec: (83.6, 6.4, 17.4, 3.6, 0.6, 5),
            prog: (52.5, 27.4, 37.5, 1.0, 1.4),
            time: (31.4, 2.0),
            ctl: Ctl { y: 36.6, rw: Seg { x: 59.0, w: 6.2 }, pp: Seg { x: 71.25, w: 3.4 }, pp_play_w: 3.9, ff: Seg { x: 83.5, w: 6.2 } },
            vol: (42.2, 56.6, 28.8, 1.0, (53.9, 1.6), (88.0, 3.0)),
            pill: (71.25, 47.2, 4.6, 2.1, 2.1, 1.0, 2.6),
            mq_w: 83.6 - 2.0 - 52.5,
        },
    }
}

impl Template for Ios {
    fn meta(&self) -> &TemplateMeta {
        &META
    }

    fn draw_static(&self, p: &mut dyn Painter, s: &Scene, o: Orient) {
        let l = lay(o);
        let u = p.unit();
        draw_background(p, s, &META, o, 0.0);

        // Kartu kaca: latar diburamkan lagi di dalam kartu, lalu gradien putih tipis.
        let (cx, cy, cw, ch, cr) = l.card;
        p.save();
        p.clip_rrect(RRect::new(cx * u, cy * u, cw * u, ch * u, cr * u));
        draw_background(p, s, &META, o, 3.0);
        p.fill_rect_vgrad(
            cx * u,
            cy * u,
            cw * u,
            ch * u,
            &[(0.0, "rgba(255,255,255,.34)"), (0.55, "rgba(255,255,255,.13)"), (1.0, "rgba(255,255,255,.08)")],
        );
        p.restore();
        p.stroke_rrect(
            RRect::new((cx + 0.125) * u, (cy + 0.125) * u, (cw - 0.25) * u, (ch - 0.25) * u, (cr - 0.125) * u),
            0.25 * u,
            "rgba(255,255,255,.14)",
        );

        let (ax, ay, asz, ar) = l.art;
        p.save();
        p.clip_rrect(RRect::new(ax * u, ay * u, asz * u, asz * u, ar * u));
        p.fill_rect(ax * u, ay * u, asz * u, asz * u, "#444");
        if let Some(img) = &s.art {
            p.image_cover(img, ax * u, ay * u, asz * u, asz * u);
        }
        p.restore();

        let t = &l.ctl;
        draw_icon(p, Icon::Rewind, t.rw.x * u, t.y * u, t.rw.w * u, "rgba(255,255,255,.78)");
        draw_icon(p, Icon::Forward, t.ff.x * u, t.y * u, t.ff.w * u, "rgba(255,255,255,.78)");
        let (vy, _, _, _, (lx, lw), (hx, hw)) = l.vol;
        draw_icon(p, Icon::VolLow, lx * u, vy * u, lw * u, "rgba(255,255,255,.6)");
        draw_icon(p, Icon::VolHigh, hx * u, vy * u, hw * u, "rgba(255,255,255,.6)");

        let device = s.field("device").trim();
        if !device.is_empty() {
            let (px, py, ph, fs, pad, gap, icon) = l.pill;
            let f = font(600, fs * u);
            let tw = p.measure(device, &f);
            let total = pad * u * 2.0 + icon * u + gap * u + tw;
            let x0 = px * u - total / 2.0;
            p.fill_rrect(RRect::new(x0, (py - ph / 2.0) * u, total, ph * u, ph * u / 2.0), "rgba(255,255,255,.14)");
            draw_icon(p, Icon::Airplay, x0 + pad * u + icon * u / 2.0, py * u, icon * u, "rgba(255,255,255,.95)");
            p.text(device, x0 + pad * u + icon * u + gap * u, py * u, &f, "rgba(255,255,255,.95)", Align::Left);
        }
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
            lp.text(artist, ox, cy, &sf, "rgba(255,255,255,.56)", Align::Left)
        });

        // Spektrum 5 batang
        let (sx, sw_, sy, sh, bw, n) = l.spec;
        let gap = (sw_ - n as f64 * bw) / (n as f64 - 1.0);
        for i in 0..n {
            let hh = bar_frac(f.spec[i]) * sh * u;
            let bx = sx * u + i as f64 * (bw + gap) * u;
            p.fill_rrect(RRect::new(bx, sy * u - hh / 2.0, bw * u, hh, bw * u / 2.0), "rgba(255,255,255,.64)");
        }

        let (px, py, pw, ph, knob) = l.prog;
        let frac = f.progress();
        bar(p, px * u, py * u, pw * u, ph * u, frac, "rgba(255,255,255,.88)", "rgba(255,255,255,.3)");
        p.fill_circle((px + pw * frac) * u, py * u, knob * u / 2.0, "#fff");

        let (ty, tfs) = l.time;
        let timef = font(500, tfs * u);
        let cu = f.cur.min(if f.dur > 0.0 { f.dur } else { f.cur });
        p.text(&fmt_time(cu), px * u, ty * u, &timef, "rgba(255,255,255,.6)", Align::Left);
        let rem = format!("-{}", fmt_time((f.dur - cu).max(0.0)));
        p.text(&rem, (px + pw) * u, ty * u, &timef, "rgba(255,255,255,.6)", Align::Right);

        let t = &l.ctl;
        let (icon, w) = if f.playing { (Icon::Pause, t.pp.w) } else { (Icon::Play, t.pp_play_w) };
        draw_icon(p, icon, t.pp.x * u, t.y * u, w * u, "#fff");

        let (vy, vx, vw, vh, _, _) = l.vol;
        bar(p, vx * u, vy * u, vw * u, vh * u, s.vol, "rgba(255,255,255,.88)", "rgba(255,255,255,.3)");
    }

    fn hits(&self, _s: &Scene, o: Orient) -> Vec<Hit> {
        let l = lay(o);
        let (px, py, pw, _, _) = l.prog;
        let (vy, vx, vw, _, _, _) = l.vol;
        let t = &l.ctl;
        let pad = 4.4;
        let seg = |s: &Seg, h: f64| hit_box(s.x, t.y, s.w + pad, h + pad);
        vec![
            Hit { rect: hit_box(px + pw / 2.0, py, pw, 6.0), action: Action::Seek },
            Hit { rect: hit_box(vx + vw / 2.0, vy, vw, 6.0), action: Action::Volume },
            Hit { rect: seg(&t.pp, t.pp.w * 48.0 / 34.0), action: Action::PlayPause },
            Hit { rect: seg(&t.rw, t.rw.w * 0.6), action: Action::Skip(-10.0) },
            Hit { rect: seg(&t.ff, t.ff.w * 0.6), action: Action::Skip(10.0) },
        ]
    }
}
