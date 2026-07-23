use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Internal link graph for PageRank and orphan detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraph {
    /// Adjacency list: URL -> set of URLs it links to.
    pub adjacency: HashMap<String, HashSet<String>>,
    /// Reverse adjacency: URL -> set of URLs linking to it.
    pub reverse: HashMap<String, HashSet<String>>,
    /// PageRank scores.
    pub pagerank: HashMap<String, f64>,
}

impl LinkGraph {
    /// Create empty link graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            reverse: HashMap::new(),
            pagerank: HashMap::new(),
        }
    }

    /// Add a link from source to target.
    pub fn add_link(&mut self, source: &str, target: &str) {
        self.adjacency
            .entry(source.to_string())
            .or_default()
            .insert(target.to_string());
        self.reverse
            .entry(target.to_string())
            .or_default()
            .insert(source.to_string());
    }

    /// Compute PageRank scores.
    pub fn compute_pagerank(&mut self, damping: f64, iterations: usize) {
        let urls: Vec<String> = {
            let mut all_urls: HashSet<String> = HashSet::new();
            for (k, v) in &self.adjacency {
                all_urls.insert(k.clone());
                all_urls.extend(v.iter().cloned());
            }
            all_urls.into_iter().collect()
        };

        let n = urls.len() as f64;
        if n == 0.0 {
            return;
        }

        let mut scores: HashMap<String, f64> = urls.iter().map(|u| (u.clone(), 1.0 / n)).collect();

        for _ in 0..iterations {
            let mut new_scores: HashMap<String, f64> = HashMap::new();

            for url in &urls {
                let mut rank = (1.0 - damping) / n;

                if let Some(linkers) = self.reverse.get(url) {
                    for linker in linkers {
                        if let Some(outbound) = self.adjacency.get(linker) {
                            let out_degree = outbound.len() as f64;
                            if out_degree > 0.0 {
                                rank += damping * scores.get(linker).unwrap_or(&(1.0 / n)) / out_degree;
                            }
                        }
                    }
                }

                new_scores.insert(url.clone(), rank);
            }

            scores = new_scores;
        }

        self.pagerank = scores;
    }

    /// Find orphan pages (no inbound links).
    #[must_use]
    pub fn orphan_pages(&self) -> Vec<String> {
        self.adjacency
            .keys()
            .filter(|url| !self.reverse.contains_key(*url))
            .cloned()
            .collect()
    }

    /// Get all URLs in the graph.
    #[must_use]
    pub fn all_urls(&self) -> HashSet<String> {
        let mut urls = HashSet::new();
        for k in self.adjacency.keys() {
            urls.insert(k.clone());
        }
        for v in self.adjacency.values() {
            urls.extend(v.iter().cloned());
        }
        urls
    }

    /// Get outbound links for a URL.
    #[must_use]
    pub fn outbound(&self, url: &str) -> Vec<String> {
        self.adjacency
            .get(url)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get inbound links for a URL.
    #[must_use]
    pub fn inbound(&self, url: &str) -> Vec<String> {
        self.reverse
            .get(url)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Export as DOT format for Graphviz.
    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph linkgraph {\n");
        for (source, targets) in &self.adjacency {
            for target in targets {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", source, target));
            }
        }
        dot.push('}');
        dot
    }

    /// Export as CSV adjacency list.
    #[must_use]
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("source,target,pagerank_source,pagerank_target\n");
        for (source, targets) in &self.adjacency {
            let pr_source = self.pagerank.get(source).unwrap_or(&0.0);
            for target in targets {
                let pr_target = self.pagerank.get(target).unwrap_or(&0.0);
                csv.push_str(&format!(
                    "{},{},{:.6},{:.6}\n",
                    source, target, pr_source, pr_target
                ));
            }
        }
        csv
    }
}

impl Default for LinkGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_graph_add_link() {
        let mut graph = LinkGraph::new();
        graph.add_link("A", "B");
        graph.add_link("B", "C");
        graph.add_link("C", "A");

        assert_eq!(graph.outbound("A").len(), 1);
        assert_eq!(graph.inbound("A").len(), 1);
    }

    #[test]
    fn test_link_graph_pagerank() {
        let mut graph = LinkGraph::new();
        graph.add_link("A", "B");
        graph.add_link("B", "C");
        graph.add_link("C", "A");

        graph.compute_pagerank(0.85, 20);

        // All nodes should have similar PageRank in a cycle
        let pr_a = graph.pagerank.get("A").unwrap();
        let pr_b = graph.pagerank.get("B").unwrap();
        let pr_c = graph.pagerank.get("C").unwrap();
        assert!((pr_a - pr_b).abs() < 0.01);
        assert!((pr_b - pr_c).abs() < 0.01);
    }

    #[test]
    fn test_link_graph_orphans() {
        let mut graph = LinkGraph::new();
        graph.add_link("A", "B");
        graph.add_link("B", "C");
        graph.add_link("D", "E");

        let orphans = graph.orphan_pages();
        // D has no inbound links, E has no outbound links
        assert!(orphans.contains(&"D".to_string()));
    }

    #[test]
    fn test_link_graph_to_dot() {
        let mut graph = LinkGraph::new();
        graph.add_link("A", "B");
        let dot = graph.to_dot();
        assert!(dot.contains("\"A\" -> \"B\""));
    }
}
