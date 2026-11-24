//! Kartu Musik: pemutar musik (Android / iOS / Instagram) yang bisa diekspor sebagai video.
//! Seluruh logika berjalan di browser lewat WebAssembly.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::needless_range_loop
)]

pub mod app;
pub mod bridge;
pub mod dom;
pub mod export;
pub mod geom;
pub mod icons;
pub mod image;
pub mod marquee;
pub mod paint;
pub mod render;
pub mod spectrum;
pub mod state;
pub mod templates;
pub mod text;
