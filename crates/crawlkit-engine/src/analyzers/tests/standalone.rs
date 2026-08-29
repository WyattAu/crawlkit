use crate::analyzers::*;

/// Direct fixtures for KeywordAnalyzer::compute_tfidf — hand-computed from
/// idf = ln(N/df) + 1, tfidf = tf * idf. Added after the mutants pass on
/// seo_analyzers.rs showed arithmetic mutants surviving here (same class
/// as the flesch gaps fixed in 4.0.0).
#[test]
fn test_compute_tfidf_fixtures() {
    use std::collections::HashMap;

    let tf: HashMap<String, f64> = [("alpha".to_string(), 0.5)].into();
    // Two "documents" in the corpus; alpha appears in one with tf 0.5.
    let corpus: HashMap<String, f64> =
        [("alpha".to_string(), 0.5), ("beta".to_string(), 0.25)].into();

    let out = KeywordAnalyzer::compute_tfidf(&tf, &corpus);
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    // idf = ln(2/0.5)+1 = ln(4)+1 = 2.386294361...; tfidf = 0.5 * that.
    close(out["alpha"], 0.5 * ((2.0f64 / 0.5).ln() + 1.0));

    // Term absent from the corpus: idf falls back to 1.0 -> tfidf = tf.
    let tf2: HashMap<String, f64> = [("gamma".to_string(), 0.25)].into();
    let out2 = KeywordAnalyzer::compute_tfidf(&tf2, &corpus);
    close(out2["gamma"], 0.25);

    // Empty corpus (total_docs clamps to 1): df 0 -> idf 1.0 -> tfidf = tf.
    let out3 = KeywordAnalyzer::compute_tfidf(&tf2, &HashMap::new());
    close(out3["gamma"], 0.25);

    // Higher df (more common term) must yield a strictly smaller idf.
    let common: HashMap<String, f64> = [("alpha".to_string(), 2.0)].into();
    let out_common = KeywordAnalyzer::compute_tfidf(&tf, &common);
    // idf = ln(1/2)+1 < 1 -> score below plain tf.
    assert!(out_common["alpha"] < 0.5);
}

/// Direct fixtures for KeywordAnalyzer::keyword_density (percent = count /
/// total * 100) and its zero/empty guards.
#[test]
fn test_keyword_density_fixtures() {
    let tokens: Vec<String> = ["a", "a", "b"].iter().map(|s| s.to_string()).collect();
    let d = KeywordAnalyzer::keyword_density(&tokens, 4);
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    close(d["a"], 50.0); // 2/4*100
    close(d["b"], 25.0); // 1/4*100

    // Zero total words: empty map, never a division by zero.
    assert!(KeywordAnalyzer::keyword_density(&tokens, 0).is_empty());

    // Density threshold handoff: 1.5% boundary is inclusive.
    let d2 = KeywordAnalyzer::keyword_density(&["x".to_string()], 66);
    close(d2["x"], 100.0 / 66.0);
}

/// Direct fixtures for HreflangValidator::is_valid_locale — every branch
/// (x-default, language-only length/charset bounds, language-region with
/// alpha-2 and numeric-4 regions, multi-segment rejection).
#[test]
fn test_is_valid_locale_fixtures() {
    use crate::analyzers::seo_analyzers::HreflangValidator;
    let _v = HreflangValidator::new();

    // x-default passes unconditionally.
    assert!(HreflangValidator::is_valid_locale("x-default"));

    // Language-only: 2-3 ascii letters.
    assert!(HreflangValidator::is_valid_locale("en"));
    assert!(HreflangValidator::is_valid_locale("dan"));
    assert!(!HreflangValidator::is_valid_locale("e")); // too short
    assert!(!HreflangValidator::is_valid_locale("engs")); // too long
    assert!(!HreflangValidator::is_valid_locale("e1")); // non-alpha

    // Language-Region: alpha-2 or numeric-4 (UN M49).
    assert!(HreflangValidator::is_valid_locale("en-US"));
    assert!(HreflangValidator::is_valid_locale("pt-BR"));
    assert!(HreflangValidator::is_valid_locale("es-419")); // UN M49 Latin America
    assert!(HreflangValidator::is_valid_locale("en-001")); // UN M49 world
    assert!(!HreflangValidator::is_valid_locale("en-USA")); // 3-letter region
    assert!(!HreflangValidator::is_valid_locale("en-1")); // 1-digit region
    assert!(!HreflangValidator::is_valid_locale("en-4199")); // 4-digit region (not M49)
    assert!(!HreflangValidator::is_valid_locale("e1-US")); // bad language half
    assert!(!HreflangValidator::is_valid_locale("en-US-variant")); // 3 segments
}

/// Direct fixtures for SitemapAnalyzer::is_valid_lastmod — full-year scan
/// window, separator/digit requirements, and the short-input guard.
#[test]
fn test_is_valid_lastmod_fixtures() {
    use crate::analyzers::seo_analyzers::SitemapAnalyzer;

    // Canonical forms.
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01"));
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15"));
    assert!(SitemapAnalyzer::is_valid_lastmod(
        "2024-01-15T10:30:00+00:00"
    ));

    // Too short for the YYYY-MM scan window.
    assert!(!SitemapAnalyzer::is_valid_lastmod(""));
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024"));
    assert!(!SitemapAnalyzer::is_valid_lastmod("202-01"));

    // Window-internal separator/digit defects.
    assert!(!SitemapAnalyzer::is_valid_lastmod("202x-01")); // year digit
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024x01")); // separator
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024-xy")); // month digits

    // Valid date appearing after a prefix (scan finds it anywhere).
    assert!(SitemapAnalyzer::is_valid_lastmod("modified:2024-06"));
}
