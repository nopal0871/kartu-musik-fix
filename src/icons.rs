//! Ikon vektor (data path SVG). Satu tabel dipakai semua template.

use crate::paint::Painter;

pub struct Part {
    pub d: &'static str,
    pub fill: bool,
    pub sw: f64,
}

pub struct IconDef {
    pub w: f64,
    pub h: f64,
    pub parts: &'static [Part],
}

const fn p(d: &'static str, fill: bool, sw: f64) -> Part {
    Part { d, fill, sw }
}

const SPK: &str = "M3 11H9L18 4V28L9 21H3Z";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Rewind,
    Forward,
    Pause,
    Play,
    Airplay,
    VolLow,
    VolHigh,
    SkipPrev,
    SkipNext,
    PlayL,
    PauseL,
    Share,
    Heart,
    Queue,
    Moon,
    Shuffle,
    Lyrics,
    Repeat,
    Dots,
}

impl Icon {
    pub fn def(self) -> &'static IconDef {
        &DEFS[self as usize]
    }
}

static DEFS: [IconDef; 19] = [
    IconDef { w: 60.0, h: 36.0, parts: &[p("M27 4V32L3 18Z M55 4V32L31 18Z", true, 5.0)] },
    IconDef { w: 60.0, h: 36.0, parts: &[p("M5 4V32L29 18Z M33 4V32L57 18Z", true, 5.0)] },
    IconDef {
                w: 34.0,
                h: 48.0,
                parts: &[p(
                    "M4 1H9A3 3 0 0 1 12 4V44A3 3 0 0 1 9 47H4A3 3 0 0 1 1 44V4A3 3 0 0 1 4 1ZM25 1H30A3 3 0 0 1 33 4V44A3 3 0 0 1 30 47H25A3 3 0 0 1 22 44V4A3 3 0 0 1 25 1Z",
                    true,
                    0.0,
                )],
            },
    IconDef { w: 40.0, h: 48.0, parts: &[p("M9 6L34 24L9 42Z", true, 6.0)] },
    IconDef {
                w: 40.0,
                h: 40.0,
                parts: &[
                    p("M10.5 27A14 14 0 1 1 29.5 27", false, 2.8),
                    p("M14.5 22.5A8 8 0 1 1 25.5 22.5", false, 2.8),
                    p("M20 23L28 34H12Z", true, 2.0),
                ],
            },
    IconDef { w: 24.0, h: 32.0, parts: &[p(SPK, true, 3.0)] },
    IconDef {
                w: 40.0,
                h: 32.0,
                parts: &[
                    p(SPK, true, 3.0),
                    p("M22 11Q26 16 22 21", false, 2.8),
                    p("M26 7Q33 16 26 25", false, 2.8),
                    p("M30 4Q38 16 30 28", false, 2.8),
                ],
            },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M6 5.5V18.5", false, 2.4), p("M19 5.5V18.5L9.5 12Z", true, 2.0)] },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M18 5.5V18.5", false, 2.4), p("M5 5.5V18.5L14.5 12Z", true, 2.0)] },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M7 4.5V19.5L19.5 12Z", false, 2.2)] },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M8 5V19", false, 3.0), p("M16 5V19", false, 3.0)] },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M14 5L21 11.5L14 18V14C9 14 5.5 15.5 3.5 19C4.5 12.5 8 9.3 14 8.8Z", false, 1.8)] },
    IconDef {
                w: 24.0,
                h: 24.0,
                parts: &[p("M12 20.5C5 15.5 3 12 3 8.8C3 6.3 5 4.5 7.3 4.5C9.3 4.5 11 5.7 12 7.5C13 5.7 14.7 4.5 16.7 4.5C19 4.5 21 6.3 21 8.8C21 12 19 15.5 12 20.5Z", false, 1.9)],
            },
    IconDef {
                w: 24.0,
                h: 24.0,
                parts: &[
                    p("M3 6H16M3 11H16M3 16H9", false, 2.0),
                    p("M15.5 19.7A2.2 2.2 0 1 0 15.5 15.3A2.2 2.2 0 0 0 15.5 19.7Z", true, 0.0),
                    p("M17.7 17.5V9.5L21.5 10.5", false, 1.8),
                ],
            },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M20 14.5A8.5 8.5 0 1 1 9.5 4A6.8 6.8 0 0 0 20 14.5Z", false, 2.0)] },
    IconDef {
                w: 24.0,
                h: 24.0,
                parts: &[p("M3 7H7C13 7 11 17 17 17H20M17 14L20.5 17L17 20M3 17H7C8.5 17 9.5 16.3 10.3 15.2M13.7 8.8C14.5 7.7 15.5 7 17 7H20M17 4L20.5 7L17 10", false, 1.9)],
            },
    IconDef { w: 24.0, h: 24.0, parts: &[p("M4 5H20M7 9.5H17M4 14H20M7 18.5H17", false, 1.9)] },
    IconDef {
                w: 24.0,
                h: 24.0,
                parts: &[p("M4 11V9A3 3 0 0 1 7 6H19M16 3L19.5 6L16 9M20 13V15A3 3 0 0 1 17 18H5M8 15L4.5 18L8 21", false, 1.9)],
            },
    IconDef {
                w: 24.0,
                h: 24.0,
                parts: &[p("M12 3.7A1.8 1.8 0 1 1 12 7.3A1.8 1.8 0 1 1 12 3.7ZM12 10.2A1.8 1.8 0 1 1 12 13.8A1.8 1.8 0 1 1 12 10.2ZM12 16.7A1.8 1.8 0 1 1 12 20.3A1.8 1.8 0 1 1 12 16.7Z", true, 0.0)],
            },
];

/// Gambar ikon berpusat di (cx, cy) dengan lebar `w_px`.
pub fn draw_icon(pt: &mut dyn Painter, icon: Icon, cx: f64, cy: f64, w_px: f64, color: &str) {
    let d = icon.def();
    let k = w_px / d.w;
    let (tx, ty) = (cx - w_px / 2.0, cy - d.h * k / 2.0);
    for part in d.parts {
        let fill = part.fill.then_some(color);
        let stroke = (part.sw > 0.0).then_some((color, part.sw));
        pt.svg_path(part.d, tx, ty, k, fill, stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::mock::Mock;

    #[test]
    fn every_icon_has_parts_and_positive_size() {
        use Icon::*;
        for i in [
            Rewind, Forward, Pause, Play, Airplay, VolLow, VolHigh, SkipPrev, SkipNext, PlayL,
            PauseL, Share, Heart, Queue, Moon, Shuffle, Lyrics, Repeat, Dots,
        ] {
            let d = i.def();
            assert!(d.w > 0.0 && d.h > 0.0 && !d.parts.is_empty(), "{i:?}");
        }
    }

    #[test]
    fn draw_icon_emits_one_path_per_part() {
        let mut m = Mock::new(100.0, 100.0);
        draw_icon(&mut m, Icon::VolHigh, 50.0, 50.0, 20.0, "#fff");
        assert_eq!(m.count("path"), Icon::VolHigh.def().parts.len());
    }
}
