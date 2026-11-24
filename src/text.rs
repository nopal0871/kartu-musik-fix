//! Fungsi teks murni (mudah diuji).

use crate::geom::Orient;

/// Tebak judul lagu dari nama berkas: buang ekstensi, ubah `_` jadi spasi, rapikan spasi.
pub fn guess_title(file_name: &str) -> String {
    let stem = match file_name.rfind('.') {
        Some(i) if i > 0 && !file_name[i..].contains(['/', '\\']) => &file_name[..i],
        _ => file_name,
    };
    stem.replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Nama berkas hasil unduhan, mis. `Judul-Lagu-9x16`.
pub fn file_base(title: &str, orient: Orient) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .collect();
    let slug = cleaned.split_whitespace().collect::<Vec<_>>().join("-");
    let slug: String = slug.chars().take(40).collect();
    let base = if slug.is_empty() {
        "kartu-musik".to_string()
    } else {
        slug
    };
    format!(
        "{base}{}",
        if orient.is_landscape() {
            "-16x9"
        } else {
            "-9x16"
        }
    )
}

/// Batasi jumlah karakter (bukan byte).
pub fn limit_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guesses_title_from_filename() {
        assert_eq!(guess_title("Artis_-_Judul  Lagu.mp3"), "Artis - Judul Lagu");
        assert_eq!(guess_title("tanpa-ekstensi"), "tanpa-ekstensi");
        assert_eq!(guess_title(".hidden"), ".hidden");
        assert_eq!(guess_title(""), "");
    }

    #[test]
    fn file_base_is_safe_and_short() {
        assert_eq!(
            file_base("Halo, Dunia!", Orient::Portrait),
            "Halo-Dunia-9x16"
        );
        assert_eq!(file_base("   ", Orient::Landscape), "kartu-musik-16x9");
        assert!(file_base(&"a".repeat(100), Orient::Portrait).len() <= 40 + 5);
        assert_eq!(
            file_base("../../etc/passwd", Orient::Portrait),
            "etcpasswd-9x16"
        );
    }

    #[test]
    fn limit_counts_chars_not_bytes() {
        assert_eq!(limit_chars("ééééé", 3), "ééé");
    }
}
