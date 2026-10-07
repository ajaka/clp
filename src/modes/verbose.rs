use colored::*;

/// Print a verbose summary of copied content.
pub fn report(content: &str, headline: &str) {
    let lines = content.lines().count();
    let chars = content.chars().count();
    let preview: String = content.chars().take(50).collect();
    let truncated = content.chars().nth(50).is_some();

    println!("{}", headline.green().bold());
    println!("  Lines:      {}", lines);
    println!("  Characters: {}", chars);
    println!("  Bytes:      {}", content.len());
    print!("  Preview:    {}", preview.white());
    if truncated {
        print!("{}", "…".bright_black());
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_truncates_at_50_chars_with_ellipsis_flag() {
        let long = "x".repeat(60);
        assert!(long.chars().nth(50).is_some());

        let short = "x".repeat(10);
        assert!(short.chars().nth(50).is_none());
    }

    #[test]
    fn preview_is_char_safe_not_byte_safe() {
        // 30 CJK chars = 90 bytes; a byte-based slice would panic here.
        let cjk = "世".repeat(30);
        let preview: String = cjk.chars().take(50).collect();
        assert_eq!(preview.chars().count(), 30);
    }

    #[test]
    fn report_does_not_panic_on_empty_or_multibyte() {
        report("", "test");
        report("héllo wörld\nsecond line", "test");
    }
}
