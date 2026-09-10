//! CI drift-check entry point (ROADMAP Phase 0.1).
//!
//! Compares the `[counts]` table in the committed `docs/capabilities.toml`
//! against values generated from the live analyzer registry. Exit code 0
//! means no drift; exit code 1 prints each drifted key.
//!
//! CI usage (see `.github/workflows/ci.yml`, `manifest-drift` job):
//!
//! ```bash
//! cargo run --quiet -p crawlkit-engine --example manifest_drift_check
//! ```
//!
//! Regenerate the committed table after intentional analyzer changes with:
//!
//! ```bash
//! cargo run --quiet -p crawlkit-engine --example manifest_drift_check -- --print
//! ```

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--print") {
        print!("{}", crawlkit_engine::manifest::render_toml());
        return;
    }

    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/capabilities.toml"
    );
    let committed = match std::fs::read_to_string(manifest_path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("manifest drift check: cannot read {manifest_path}: {e}");
            std::process::exit(1);
        }
    };

    match crawlkit_engine::manifest::check_drift(&committed) {
        Ok(()) => {
            println!("manifest drift check: OK (counts match registry)");
        }
        Err(drift) => {
            eprintln!("manifest drift check FAILED — documentation counts are stale:");
            for line in drift {
                eprintln!("  {line}");
            }
            eprintln!();
            eprintln!("Regenerate with:");
            eprintln!("  cargo run -p crawlkit-engine --example manifest_drift_check -- --print");
            std::process::exit(1);
        }
    }
}
