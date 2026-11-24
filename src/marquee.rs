//! Teks bergeser ala iOS: jeda, geser sampai habis, lalu ulang dengan celah.

use crate::paint::Painter;

pub const PAUSE: f64 = 2.5;
pub const SPEED: f64 = 8.0;
pub const GAP: f64 = 8.0;
pub const FADE: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mq {
    pub over: bool,
    pub off: f64,
}

/// Semua dalam satuan U. `tw` = lebar teks, `bw` = lebar kotak.
pub fn calc(t: f64, tw: f64, bw: f64) -> Mq {
    if tw <= bw + 0.5 {
        return Mq {
            over: false,
            off: 0.0,
        };
    }
    let dist = tw + GAP;
    let cyc = PAUSE + dist / SPEED;
    let ph = t.rem_euclid(cyc);
    Mq {
        over: true,
        off: if ph < PAUSE {
            0.0
        } else {
            (ph - PAUSE) * SPEED
        },
    }
}

/// Gambar teks dalam kotak (x, y_mid, w, h) piksel. `draw(painter, offset_x, tengah_y)` dipanggil
/// satu kali (teks muat) atau dua kali (teks panjang, untuk efek melingkar) di dalam lapisan.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    p: &mut dyn Painter,
    x: f64,
    y_mid: f64,
    w: f64,
    h: f64,
    text_w_px: f64,
    t: f64,
    draw: &mut dyn FnMut(&mut dyn Painter, f64, f64),
) {
    let u = p.unit();
    let r = calc(t, text_w_px / u, w / u);
    let fade_l = FADE.min(r.off) * u;
    let fade_r = if r.over { FADE * u } else { 0.0 };
    p.layer(x, y_mid - h / 2.0, w, h, fade_l, fade_r, &mut |lp| {
        draw(lp, -r.off * u, h / 2.0);
        if r.over {
            draw(lp, -r.off * u + text_w_px + GAP * u, h / 2.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::mock::Mock;

    #[test]
    fn short_text_never_moves() {
        assert_eq!(
            calc(0.0, 10.0, 50.0),
            Mq {
                over: false,
                off: 0.0
            }
        );
        assert!(!calc(99.0, 50.4, 50.0).over);
    }

    #[test]
    fn long_text_pauses_then_scrolls_then_loops() {
        let (tw, bw) = (100.0, 50.0);
        assert_eq!(calc(0.0, tw, bw).off, 0.0);
        assert_eq!(calc(PAUSE - 0.01, tw, bw).off, 0.0);
        let mid = calc(PAUSE + 2.0, tw, bw);
        assert!(mid.over && (mid.off - 2.0 * SPEED).abs() < 1e-9);
        let cyc = PAUSE + (tw + GAP) / SPEED;
        assert!(calc(cyc + 0.1, tw, bw).off == 0.0);
    }

    #[test]
    fn draw_calls_closure_twice_when_overflowing() {
        let mut m = Mock::new(100.0, 100.0);
        let mut n = 0;
        draw(&mut m, 0.0, 10.0, 20.0, 4.0, 80.0, 5.0, &mut |_, _, _| {
            n += 1
        });
        assert_eq!(n, 2);
        let mut k = 0;
        draw(&mut m, 0.0, 10.0, 20.0, 4.0, 5.0, 5.0, &mut |_, _, _| {
            k += 1
        });
        assert_eq!(k, 1);
    }
}
