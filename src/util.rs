//! Utility functions for `ZeroClaw`.
//!
//! This module contains reusable helper functions used across the codebase.

use unicode_width::UnicodeWidthStr;

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
///
/// This function safely handles multi-byte UTF-8 characters (emoji, CJK, accented characters)
/// by using character boundaries instead of byte indices.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep (excluding "...")
///
/// # Returns
/// * Original string if length <= `max_chars`
/// * Truncated string with "..." appended if length > `max_chars`
///
/// # Examples
/// ```ignore
/// use zeroclaw::util::truncate_with_ellipsis;
///
/// // ASCII string - no truncation needed
/// assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
///
/// // ASCII string - truncation needed
/// assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
///
/// // Multi-byte UTF-8 (emoji) - safe truncation
/// assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
/// assert_eq!(truncate_with_ellipsis("😀😀😀😀", 2), "😀😀...");
///
/// // Empty string
/// assert_eq!(truncate_with_ellipsis("", 10), "");
/// ```
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            // Trim trailing whitespace for cleaner output
            format!("{}...", truncated.trim_end())
        }
        None => s.to_string(),
    }
}

/// Truncate a string to fit within a terminal display width, appending "..." if truncated.
///
/// This function correctly handles:
/// - ASCII characters (width 1)
/// - CJK characters (Chinese, Japanese, Korean - width 2)
/// - Emoji and other wide characters (width 2)
/// - Combining characters (width 0)
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_width` - Maximum terminal display width
/// * `ellipsis` - The ellipsis string to append when truncated (required)
///
/// # Returns
/// * Original string if width <= `max_width`
/// * Truncated string with ellipsis appended if width > `max_width`
/// * Empty string if `max_width` is 0
/// * Truncated ellipsis if ellipsis itself exceeds `max_width`
///
/// # Examples
/// ```ignore
/// use zeroclaw::util::truncate_with_width;
///
/// // ASCII string - no truncation needed
/// assert_eq!(truncate_with_width("hello", 10, "..."), "hello");
///
/// // ASCII string - truncation needed
/// assert_eq!(truncate_with_width("hello world", 8, "..."), "hello...");
/// assert_eq!(truncate_with_width("hello world", 8, "→"), "hello w→");
///
/// // CJK characters (width 2 each)
/// assert_eq!(truncate_with_width("你好世界", 8, "..."), "你好世界");
/// assert_eq!(truncate_with_width("你好世界", 7, "..."), "你好...");
/// assert_eq!(truncate_with_width("你好世界", 2, "..."), "..");
/// assert_eq!(truncate_with_width("你好世界", 1, "..."), ".");  // ellipsis truncated to 1 char
/// assert_eq!(truncate_with_width("你好世界", 0, "..."), "");
///
/// // Mixed ASCII and CJK
/// assert_eq!(truncate_with_width("Hello 世界", 9, "..."), "Hello 世界");
/// assert_eq!(truncate_with_width("Hello 世界", 8, "..."), "Hello 世...");
/// ```
pub fn truncate_with_width(s: &str, max_width: usize, ellipsis: &str) -> String {
    // Handle edge case: max_width == 0
    if max_width == 0 {
        return String::new();
    }

    let current_width = UnicodeWidthStr::width(s);

    // No truncation needed
    if current_width <= max_width {
        return s.to_string();
    }

    // Calculate width of ellipsis
    let ellipsis_width = UnicodeWidthStr::width(ellipsis);

    // If ellipsis itself exceeds or equals max_width, truncate ellipsis to fit
    let effective_ellipsis: String = if ellipsis_width >= max_width {
        // Find the maximum ellipsis that fits within max_width
        let mut truncated_ellipsis = String::new();
        let mut width_so_far = 0;
        let mut buf = [0u8; 4];
        for c in ellipsis.chars() {
            let encoded = c.encode_utf8(&mut buf);
            let char_width = UnicodeWidthStr::width(encoded);
            if width_so_far + char_width > max_width {
                break;
            }
            width_so_far += char_width;
            truncated_ellipsis.push(c);
        }
        // If we couldn't fit any character, return empty
        if truncated_ellipsis.is_empty() {
            return String::new();
        }
        truncated_ellipsis
    } else {
        ellipsis.to_string()
    };

    let effective_ellipsis_width = UnicodeWidthStr::width(effective_ellipsis.as_str());
    let available_width = max_width.saturating_sub(effective_ellipsis_width);

    // Buffer for encoding characters to UTF-8
    let mut buf = [0u8; 4];

    // Find the truncation point
    let mut width_so_far = 0;
    let mut truncate_at = 0;

    for (idx, c) in s.char_indices() {
        let encoded = c.encode_utf8(&mut buf);
        let char_width = UnicodeWidthStr::width(encoded);

        if width_so_far + char_width > available_width {
            break;
        }

        width_so_far += char_width;
        truncate_at = idx + c.len_utf8();
    }

    if truncate_at == 0 {
        effective_ellipsis
    } else {
        format!("{}{}", &s[..truncate_at].trim_end(), effective_ellipsis)
    }
}

/// Utility enum for handling optional values.
pub enum MaybeSet<T> {
    Set(T),
    Unset,
    Null,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii_no_truncation() {
        // ASCII string shorter than limit - no change
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 50), "hello world");
    }

    #[test]
    fn test_truncate_ascii_with_truncation() {
        // ASCII string longer than limit - truncates
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(
            truncate_with_ellipsis("This is a long message", 10),
            "This is a..."
        );
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn test_truncate_at_exact_boundary() {
        // String exactly at boundary - no truncation
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_emoji_single() {
        // Single emoji (4 bytes) - should not panic
        let s = "🦀";
        assert_eq!(truncate_with_ellipsis(s, 10), s);
        assert_eq!(truncate_with_ellipsis(s, 1), s);
    }

    #[test]
    fn test_truncate_emoji_multiple() {
        // Multiple emoji - safe truncation at character boundary
        let s = "😀😀😀😀"; // 4 emoji, each 4 bytes = 16 bytes total
        assert_eq!(truncate_with_ellipsis(s, 2), "😀😀...");
        assert_eq!(truncate_with_ellipsis(s, 3), "😀😀😀...");
    }

    #[test]
    fn test_truncate_mixed_ascii_emoji() {
        // Mixed ASCII and emoji
        assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
        assert_eq!(truncate_with_ellipsis("Hi 😊", 10), "Hi 😊");
    }

    #[test]
    fn test_truncate_cjk_characters() {
        // CJK characters (Chinese - each is 3 bytes)
        let s = "这是一个测试消息用来触发崩溃的中文"; // 21 characters
        let result = truncate_with_ellipsis(s, 16);
        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len() - 1));
    }

    #[test]
    fn test_truncate_accented_characters() {
        // Accented characters (2 bytes each in UTF-8)
        let s = "café résumé naïve";
        assert_eq!(truncate_with_ellipsis(s, 10), "café résum...");
    }

    #[test]
    fn test_truncate_unicode_edge_case() {
        // Mix of 1-byte, 2-byte, 3-byte, and 4-byte characters
        let s = "aé你好🦀"; // 1 + 1 + 2 + 2 + 4 bytes = 10 bytes, 5 chars
        assert_eq!(truncate_with_ellipsis(s, 3), "aé你...");
    }

    #[test]
    fn test_truncate_long_string() {
        // Long ASCII string
        let s = "a".repeat(200);
        let result = truncate_with_ellipsis(&s, 50);
        assert_eq!(result.len(), 53); // 50 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_zero_max_chars() {
        // Edge case: max_chars = 0
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }

    // Tests for truncate_with_width

    #[test]
    fn test_truncate_width_ascii_no_truncation() {
        assert_eq!(truncate_with_width("hello", 10, "..."), "hello");
        assert_eq!(truncate_with_width("hello world", 50, "..."), "hello world");
    }

    #[test]
    fn test_truncate_width_ascii_with_truncation() {
        // max_width=8, ellipsis=3, available=5: "hello" = 5 width, fits exactly
        assert_eq!(truncate_with_width("hello world", 8, "..."), "hello...");
        // max_width=5, ellipsis=3, available=2: "he" = 2 width, fits exactly
        assert_eq!(truncate_with_width("hello world", 5, "..."), "he...");
        // max_width=4, ellipsis=3, available=1: "h" = 1 width
        assert_eq!(truncate_with_width("hello world", 4, "..."), "h...");
    }

    #[test]
    fn test_truncate_width_cjk_characters() {
        // Basic test: function should not panic with CJK characters
        let _ = truncate_with_width("你好", 4, "...");
        let _ = truncate_with_width("你好世界", 10, "...");
        // String shorter than max should return as-is
        assert_eq!(truncate_with_width("你好", 10, "..."), "你好");
    }

    #[test]
    fn test_truncate_width_mixed_ascii_cjk() {
        // Basic test: function should not panic with mixed content
        let _ = truncate_with_width("Hello 世界", 20, "...");
        assert_eq!(truncate_with_width("Hello", 10, "..."), "Hello");
    }

    #[test]
    fn test_truncate_width_emoji() {
        // max_width=8, ellipsis=3, available=5: 👋(2) + H(1) + e(1) + l(1) = 5, fits exactly
        assert_eq!(truncate_with_width("👋Hello", 8, "..."), "👋Hello");
        // max_width=7, available=4: 👋(2) + H(1) + e(1) = 4, fits exactly
        assert_eq!(truncate_with_width("👋Hello", 7, "..."), "👋Hello");
        // max_width=6, available=3: 👋(2) + H(1) = 3, fits exactly
        assert_eq!(truncate_with_width("👋Hello", 6, "..."), "👋H...");
        // max_width=5, available=2: 👋(2) fits exactly
        assert_eq!(truncate_with_width("👋Hello", 5, "..."), "👋...");
    }

    #[test]
    fn test_truncate_width_custom_ellipsis() {
        assert_eq!(truncate_with_width("hello world", 8, "→"), "hello w→");
        assert_eq!(truncate_with_width("hello world", 8, "…"), "hello w…");
    }

    #[test]
    fn test_truncate_width_empty_string() {
        assert_eq!(truncate_with_width("", 10, "..."), "");
        assert_eq!(truncate_with_width("", 0, "..."), "");
    }

    #[test]
    fn test_truncate_width_exact_boundary() {
        // String exactly at boundary - no truncation
        assert_eq!(truncate_with_width("hello", 5, "..."), "hello");
        // CJK exact boundary
        assert_eq!(truncate_with_width("你好", 4, "..."), "你好");
    }

    #[test]
    fn test_truncate_width_accented_characters() {
        // Basic test: function should not panic with accented characters
        let _ = truncate_with_width("café résumé naïve", 20, "...");
    }

    #[test]
    fn test_truncate_width_emoji_multiple() {
        let _ = truncate_with_width("👋👋👋👋", 10, "...");
    }
}
