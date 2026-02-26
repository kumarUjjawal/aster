/// UTF-8-safe character-based ellipsizing helper for dynamic UI labels.
///
/// This avoids byte-index slicing in rendering paths that can panic when text
/// contains multi-byte characters (for example typographic apostrophes).
pub fn ellipsize_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let mut char_count = 0usize;
    let mut cutoff = text.len();

    for (idx, _) in text.char_indices() {
        if char_count == max_chars {
            cutoff = idx;
            break;
        }
        char_count += 1;
    }

    if cutoff == text.len() {
        return text.to_string();
    }

    let mut output = String::with_capacity(cutoff + 3);
    output.push_str(&text[..cutoff]);
    output.push('…');
    output
}

#[cfg(test)]
mod tests {
    use super::ellipsize_chars;

    #[test]
    fn ellipsize_preserves_utf8_boundaries() {
        let text = "dn’t require";
        let result = ellipsize_chars(text, 3);
        assert_eq!(result, "dn’…");
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn ellipsize_returns_original_when_short_enough() {
        let text = "short";
        assert_eq!(ellipsize_chars(text, 10), text);
    }
}
