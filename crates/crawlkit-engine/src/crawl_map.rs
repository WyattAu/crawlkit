#![allow(
    clippy::unwrap_used,
    clippy::needless_return,
    clippy::manual_contains,
    clippy::unnecessary_to_owned,
    clippy::needless_range_loop
)]

//! Visual crawl map generator producing SVG output from link graphs.
//!
//! Generates a force-directed layout visualization of crawled pages
//! and their link relationships, color-coded by status, depth, or category.

use std::fmt::Write;

use crate::storage::PageData;

/// Configuration for the crawl map SVG output.
#[derive(Debug, Clone)]
pub struct CrawlMapConfig {
    /// SVG width in pixels.
    pub width: u32,
    /// SVG height in pixels.
    pub height: u32,
    /// Maximum number of nodes (pages) to display.
    pub max_nodes: usize,
    /// Color scheme for nodes.
    pub color_by: ColorScheme,
    /// Number of force-directed layout iterations.
    pub iterations: usize,
}

impl Default for CrawlMapConfig {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 900,
            max_nodes: 200,
            color_by: ColorScheme::Status,
            iterations: 100,
        }
    }
}

/// Color scheme for node visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Color by HTTP status code (2xx=green, 3xx=yellow, 4xx=orange, 5xx=red).
    Status,
    /// Color by crawl depth from seed URL.
    Depth,
    /// Color by link count (popularity).
    Popularity,
}

/// A positioned node in the force-directed layout.
#[derive(Debug, Clone)]
struct Node {
    x: f64,
    y: f64,
    label: String,
    color: String,
    radius: f64,
}

/// Escape a string for safe inclusion in SVG text/attribute content.
fn svg_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Truncate a URL for display, showing only the path portion.
fn truncate_url(url: &str) -> String {
    if let Some(pos) = url.find("://") {
        let path = &url[pos + 3..];
        if let Some(slash) = path.find('/') {
            let p = &path[slash..];
            if p.len() > 30 {
                format!("{}…", p.chars().take(29).collect::<String>())
            } else {
                p.to_string()
            }
        } else {
            "/".to_string()
        }
    } else {
        url.to_string()
    }
}

/// Map a status code to a fill color.
fn status_color(code: u16) -> &'static str {
    match code {
        200..=299 => "#4ade80",
        300..=399 => "#facc15",
        400..=499 => "#fb923c",
        500..=599 => "#f87171",
        _ => "#94a3b8",
    }
}

/// Map a depth to a fill color using a gradient from blue (shallow) to purple (deep).
fn depth_color(depth: usize) -> &'static str {
    match depth {
        0 => "#3b82f6",
        1 => "#6366f1",
        2 => "#8b5cf6",
        3 => "#a855f7",
        4 => "#d946ef",
        _ => "#ec4899",
    }
}

/// Map a normalized popularity score [0.0, 1.0] to a fill color.
fn popularity_color(score: f64) -> String {
    let r = (74.0 + score * 178.0) as u8;
    let g = (222.0 - score * 100.0) as u8;
    let b = (128.0 - score * 80.0) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Run a simple force-directed layout on the nodes.
fn force_directed_layout(nodes: &mut [Node], links: &[(usize, usize)], iterations: usize) {
    let repulsion_strength = 2000.0;
    let attraction_strength = 0.005;
    let ideal_distance = 120.0;
    let damping = 0.9;

    let mut vx = vec![0.0f64; nodes.len()];
    let mut vy = vec![0.0f64; nodes.len()];

    for _ in 0..iterations {
        // Repulsion between all node pairs
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let dx = nodes[i].x - nodes[j].x;
                let dy = nodes[i].y - nodes[j].y;
                let dist_sq = (dx * dx + dy * dy).max(1.0);
                let dist = dist_sq.sqrt();
                let force = repulsion_strength / dist_sq;
                let fx = dx / dist * force;
                let fy = dy / dist * force;
                vx[i] += fx;
                vy[i] += fy;
                vx[j] -= fx;
                vy[j] -= fy;
            }
        }

        // Attraction along links
        for &(src, tgt) in links {
            let dx = nodes[tgt].x - nodes[src].x;
            let dy = nodes[tgt].y - nodes[src].y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = (dist - ideal_distance) * attraction_strength;
            let fx = dx / dist * force;
            let fy = dy / dist * force;
            vx[src] += fx;
            vy[src] += fy;
            vx[tgt] -= fx;
            vy[tgt] -= fy;
        }

        // Apply velocities with damping and boundary constraints
        let w = nodes[0].x; // just need first node's coordinate system
        let _ = w;
        for (i, node) in nodes.iter_mut().enumerate() {
            vx[i] *= damping;
            vy[i] *= damping;
            node.x += vx[i];
            node.y += vy[i];
            node.x = node.x.clamp(60.0, 1140.0);
            node.y = node.y.clamp(60.0, 840.0);
        }
    }
}

/// Generate an SVG crawl map from page data and link relationships.
///
/// # Arguments
///
/// * `pages` - Slice of crawled page data.
/// * `links` - Slice of `(source_url, target_urls)` pairs representing discovered links.
/// * `config` - Layout and rendering configuration.
///
/// # Returns
///
/// A complete SVG document as a `String`.
pub fn generate_svg(
    pages: &[PageData],
    links: &[(String, Vec<String>)],
    config: &CrawlMapConfig,
) -> String {
    let n = pages.len().min(config.max_nodes);
    if n == 0 {
        return empty_svg(config);
    }

    // Build URL-to-index map for internal links only
    let mut url_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, page) in pages.iter().take(n).enumerate() {
        url_index.insert(page.url.to_string(), i);
    }

    // Count inbound links per page
    let mut inbound_counts = vec![0usize; n];
    let mut outbound_links: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (src_url, targets) in links {
        if let Some(&src_idx) = url_index.get(src_url) {
            if src_idx < n {
                for tgt_url in targets {
                    if let Some(&tgt_idx) = url_index.get(tgt_url) {
                        if tgt_idx < n {
                            outbound_links[src_idx].push(tgt_idx);
                            inbound_counts[tgt_idx] += 1;
                        }
                    }
                }
            }
        }
    }

    // Compute link counts for radius scaling
    let link_counts: Vec<usize> = (0..n)
        .map(|i| inbound_counts[i] + outbound_links[i].len())
        .collect();
    let max_links = link_counts.iter().copied().max().unwrap_or(1).max(1);

    // Compute popularity score for Popularity color scheme
    let popularity_scores: Vec<f64> = link_counts
        .iter()
        .map(|&c| c as f64 / max_links as f64)
        .collect();

    // Assign initial positions in a circle
    let cx = config.width as f64 / 2.0;
    let cy = config.height as f64 / 2.0;
    let radius = (config.width as f64).min(config.height as f64) * 0.38;

    let mut nodes: Vec<Node> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            let page = &pages[i];
            let color = match config.color_by {
                ColorScheme::Status => status_color(page.status_code),
                ColorScheme::Depth => depth_color(0), // depth unknown from PageData alone
                ColorScheme::Popularity => "placeholder",
            };
            let r = 6.0 + (link_counts[i] as f64 / max_links as f64) * 14.0;
            let color_str = match config.color_by {
                ColorScheme::Status => color.to_string(),
                ColorScheme::Depth => depth_color(0).to_string(),
                ColorScheme::Popularity => popularity_color(popularity_scores[i]),
            };
            Node {
                x: cx + radius * angle.cos(),
                y: cy + radius * angle.sin(),
                label: truncate_url(&page.url.to_string()),
                color: color_str,
                radius: r,
            }
        })
        .collect();

    // Build edge list (only between distinct nodes in our set)
    let mut edge_list: Vec<(usize, usize)> = Vec::new();
    for src in 0..n {
        for &tgt in &outbound_links[src] {
            if src != tgt {
                edge_list.push((src, tgt));
            }
        }
    }

    // Run force-directed layout
    force_directed_layout(&mut nodes, &edge_list, config.iterations);

    // Build SVG
    let mut svg = String::with_capacity(8192);

    // Header
    writeln!(
        svg,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
  .edge {{ stroke: #cbd5e1; stroke-width: 1; stroke-opacity: 0.5; }}
  .node-label {{ font-family: sans-serif; font-size: 10px; fill: #334155; text-anchor: middle; pointer-events: none; }}
  .title {{ font-family: sans-serif; font-size: 18px; fill: #1e293b; font-weight: bold; }}
  .legend-text {{ font-family: sans-serif; font-size: 12px; fill: #475569; }}
</style>
<rect width="100%" height="100%" fill="white"/>"#,
        config.width,
        config.height,
        config.width,
        config.height,
    )
    .unwrap();

    // Title
    let seed = pages
        .first()
        .map(|p| svg_escape(&p.url.to_string()))
        .unwrap_or_default();
    writeln!(
        svg,
        r#"<text x="20" y="30" class="title">Crawl Map — {}</text>"#,
        seed
    )
    .unwrap();

    // Stats subtitle
    writeln!(
        svg,
        r#"<text x="20" y="50" class="legend-text">{} pages · {} edges</text>"#,
        n,
        edge_list.len()
    )
    .unwrap();

    // Edges
    for &(src, tgt) in &edge_list {
        writeln!(
            svg,
            r#"<line class="edge" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
            nodes[src].x, nodes[src].y, nodes[tgt].x, nodes[tgt].y,
        )
        .unwrap();
    }

    // Nodes
    for (i, node) in nodes.iter().enumerate() {
        let page = &pages[i];
        let fill = svg_escape(&node.color);
        let id = svg_escape(&page.url.to_string());
        writeln!(
            svg,
            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" stroke="white" stroke-width="1.5"><title>{}</title></circle>"#,
            node.x, node.y, node.radius, fill, id,
        )
        .unwrap();
    }

    // Labels for top pages by link count (top 25)
    let mut ranked: Vec<(usize, usize)> = (0..n).map(|i| (i, link_counts[i])).collect();
    ranked.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let label_limit = 25.min(n);
    for &(idx, _) in ranked.iter().take(label_limit) {
        let node = &nodes[idx];
        let label = svg_escape(&node.label);
        let ty = node.y - node.radius - 4.0;
        writeln!(
            svg,
            r#"<text x="{:.1}" y="{:.1}" class="node-label">{}</text>"#,
            node.x,
            if ty < 20.0 {
                node.y + node.radius + 14.0
            } else {
                ty
            },
            label,
        )
        .unwrap();
    }

    // Legend
    let legend_y = config.height as f64 - 30.0;
    writeln!(
        svg,
        r#"<text x="20" y="{:.0}" class="legend-text">Legend:</text>"#,
        legend_y
    )
    .unwrap();
    let (entries, labels): (Vec<&str>, Vec<&str>) = match config.color_by {
        ColorScheme::Status => (
            vec!["#4ade80", "#facc15", "#fb923c", "#f87171"],
            vec!["2xx", "3xx", "4xx", "5xx"],
        ),
        ColorScheme::Depth => (
            vec!["#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef"],
            vec!["Depth 0", "Depth 1", "Depth 2", "Depth 3", "Depth 4+"],
        ),
        ColorScheme::Popularity => (vec!["#4ade80", "#facc15"], vec!["High links", "Low links"]),
    };
    for (j, (c, l)) in entries.iter().zip(labels.iter()).enumerate() {
        let lx = 80.0 + j as f64 * 120.0;
        writeln!(
            svg,
            r#"<circle cx="{:.0}" cy="{:.0}" r="6" fill="{}"/><text x="{:.0}" y="{:.0}" class="legend-text">{}</text>"#,
            lx, legend_y - 3.0, c, lx + 12.0, legend_y, l,
        )
        .unwrap();
    }

    svg.push_str("</svg>\n");
    svg
}

/// Generate an empty state SVG when no pages are provided.
fn empty_svg(config: &CrawlMapConfig) -> String {
    let w = config.width;
    let h = config.height;
    let hw = w / 2;
    let hh = h / 2;
    [
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"",
        &format!(" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"),
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>",
        &format!(
            "<text x=\"{hw}\" y=\"{hh}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"18\" fill=\"#94a3b8\">No pages to display</text>"
        ),
        "</svg>",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use url::Url;

    fn make_page(url: &str, status: u16, title: Option<&str>) -> PageData {
        PageData {
            id: url.to_string(),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: status,
            title: title.map(String::from),
            description: None,
            canonical_url: None,
            word_count: None,
            load_time_ms: None,
            body_size: None,
            fetched_at: Utc::now(),
            links: Vec::new(),
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: None,
            schema_types: None,
            viewport_ok: None,
            has_csp: None,
            has_hsts: None,
            images_total: None,
            images_missing_alt: None,
            h1_count: None,
            heading_count: None,
            extractions: None,
        }
    }

    #[test]
    fn test_empty_pages_produces_valid_svg() {
        let config = CrawlMapConfig::default();
        let svg = generate_svg(&[], &[], &config);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("No pages to display"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_single_page_svg() {
        let pages = vec![make_page("https://example.com/", 200, Some("Home"))];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &[], &config);
        assert!(svg.contains("Crawl Map"));
        assert!(svg.contains("1 pages"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_svg_with_links() {
        let pages = vec![
            make_page("https://example.com/", 200, Some("Home")),
            make_page("https://example.com/about", 200, Some("About")),
            make_page("https://example.com/contact", 200, Some("Contact")),
        ];
        let links = vec![(
            "https://example.com/".to_string(),
            vec![
                "https://example.com/about".to_string(),
                "https://example.com/contact".to_string(),
            ],
        )];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &links, &config);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("2 edges"));
    }

    #[test]
    fn test_max_nodes_limits_output() {
        let pages: Vec<PageData> = (0..50)
            .map(|i| make_page(&format!("https://example.com/page{i}"), 200, None))
            .collect();
        let config = CrawlMapConfig {
            max_nodes: 5,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &[], &config);
        assert!(svg.contains("5 pages"));
    }

    #[test]
    fn test_status_color_scheme() {
        let pages = vec![
            make_page("https://example.com/ok", 200, None),
            make_page("https://example.com/redir", 301, None),
            make_page("https://example.com/notfound", 404, None),
            make_page("https://example.com/error", 500, None),
        ];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            color_by: ColorScheme::Status,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &[], &config);
        assert!(svg.contains("#4ade80")); // 2xx green
        assert!(svg.contains("#facc15")); // 3xx yellow
        assert!(svg.contains("#fb923c")); // 4xx orange
        assert!(svg.contains("#f87171")); // 5xx red
    }

    #[test]
    fn test_depth_color_scheme() {
        let pages = vec![
            make_page("https://example.com/", 200, None),
            make_page("https://example.com/a", 200, None),
        ];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            color_by: ColorScheme::Depth,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &[], &config);
        assert!(svg.contains("#3b82f6")); // depth 0
    }

    #[test]
    fn test_popularity_color_scheme() {
        let pages = vec![
            make_page("https://example.com/popular", 200, None),
            make_page("https://example.com/lonely", 200, None),
        ];
        let links = vec![(
            "https://example.com/popular".to_string(),
            vec!["https://example.com/lonely".to_string()],
        )];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            color_by: ColorScheme::Popularity,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &links, &config);
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn test_svg_escapes_special_chars_in_url() {
        let pages = vec![make_page("https://example.com/?q=1&b=2", 200, None)];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &[], &config);
        assert!(svg.contains("&amp;"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_force_layout_convergence() {
        let mut nodes: Vec<Node> = (0..5)
            .map(|i| Node {
                x: (i as f64) * 10.0,
                y: (i as f64) * 10.0,
                label: format!("n{i}"),
                color: "#000".into(),
                radius: 8.0,
            })
            .collect();
        let links = vec![(0, 1), (1, 2), (2, 3), (3, 4)];

        // Record initial state
        let initial_positions: Vec<(f64, f64)> = nodes.iter().map(|n| (n.x, n.y)).collect();

        force_directed_layout(&mut nodes, &links, 100);

        // After layout, positions should have changed (non-degenerate)
        let final_positions: Vec<(f64, f64)> = nodes.iter().map(|n| (n.x, n.y)).collect();
        assert_ne!(initial_positions, final_positions);

        // All nodes should be within bounds
        for node in &nodes {
            assert!(
                node.x >= 60.0 && node.x <= 1140.0,
                "x out of bounds: {}",
                node.x
            );
            assert!(
                node.y >= 60.0 && node.y <= 840.0,
                "y out of bounds: {}",
                node.y
            );
        }
    }

    #[test]
    fn test_force_layout_boundary_constraints() {
        let mut nodes: Vec<Node> = (0..3)
            .map(|i| Node {
                x: -1000.0 + (i as f64) * 2000.0,
                y: -1000.0 + (i as f64) * 2000.0,
                label: format!("n{i}"),
                color: "#000".into(),
                radius: 8.0,
            })
            .collect();
        let links = vec![(0, 1)];

        force_directed_layout(&mut nodes, &links, 50);

        for node in &nodes {
            assert!(
                node.x >= 60.0 && node.x <= 1140.0,
                "x out of bounds: {}",
                node.x
            );
            assert!(
                node.y >= 60.0 && node.y <= 840.0,
                "y out of bounds: {}",
                node.y
            );
        }
    }

    #[test]
    fn test_truncate_url() {
        assert_eq!(truncate_url("https://example.com/"), "/");
        assert_eq!(
            truncate_url("https://example.com/very/long/path/that/exceeds/thirty/chars"),
            "/very/long/path/that/exceeds/…"
        );
    }

    #[test]
    fn test_status_color_mapping() {
        assert_eq!(status_color(200), "#4ade80");
        assert_eq!(status_color(201), "#4ade80");
        assert_eq!(status_color(301), "#facc15");
        assert_eq!(status_color(404), "#fb923c");
        assert_eq!(status_color(500), "#f87171");
        assert_eq!(status_color(599), "#f87171");
        assert_eq!(status_color(0), "#94a3b8");
    }

    #[test]
    fn test_depth_color_mapping() {
        assert_eq!(depth_color(0), "#3b82f6");
        assert_eq!(depth_color(4), "#d946ef");
        assert_eq!(depth_color(10), "#ec4899");
    }

    #[test]
    fn test_popularity_color_is_hex() {
        let c = popularity_color(0.5);
        assert!(c.starts_with('#'));
        assert_eq!(c.len(), 7);
    }

    #[test]
    fn test_generate_svg_is_valid_xml_start() {
        let config = CrawlMapConfig::default();
        let svg = generate_svg(&[], &[], &config);
        assert!(svg.starts_with("<?xml version=\"1.0\""));
    }

    #[test]
    fn test_svg_with_mixed_internal_external_links() {
        let pages = vec![
            make_page("https://example.com/", 200, None),
            make_page("https://example.com/about", 200, None),
        ];
        let links = vec![(
            "https://example.com/".to_string(),
            vec![
                "https://example.com/about".to_string(),
                "https://external.com/page".to_string(), // external, should be ignored
            ],
        )];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &links, &config);
        // Only 1 edge (internal), not 2
        assert!(svg.contains("1 edges"));
    }

    #[test]
    fn test_self_links_ignored() {
        let pages = vec![make_page("https://example.com/", 200, None)];
        let links = vec![(
            "https://example.com/".to_string(),
            vec!["https://example.com/".to_string()], // self-link
        )];
        let config = CrawlMapConfig {
            max_nodes: 10,
            iterations: 10,
            ..Default::default()
        };
        let svg = generate_svg(&pages, &links, &config);
        assert!(svg.contains("0 edges"));
    }

    #[test]
    fn test_default_config_values() {
        let config = CrawlMapConfig::default();
        assert_eq!(config.width, 1200);
        assert_eq!(config.height, 900);
        assert_eq!(config.max_nodes, 200);
        assert_eq!(config.color_by, ColorScheme::Status);
        assert_eq!(config.iterations, 100);
    }
}
