# Video Tutorial Scripts

This document contains scripts for video tutorials about crawlkit.

## Tutorial 1: Getting Started with crawlkit (10 minutes)

### Introduction (0:00 - 1:00)

"Welcome to crawlkit, a high-performance Rust web crawler for SEO analysis.
In this tutorial, we'll cover installation, basic usage, and advanced features."

### Installation (1:00 - 2:00)

"First, install crawlkit using cargo:

```bash
cargo install crawlkit-engine
```

Or download pre-built binaries from GitHub Releases."

### Basic Crawl (2:00 - 4:00)

"Let's crawl a website:

```bash
crawlkit crawl https://example.com --max-pages 50
```

This will crawl 50 pages and analyze them for SEO issues."

### Analyzing Results (4:00 - 6:00)

"The crawl results are stored in a SQLite database. Let's generate a report:

```bash
crawlkit report crawl-data/ --format html --output report.html
```

Open the report in your browser to see the analysis."

### Advanced Features (6:00 - 8:00)

"Crawlkit supports advanced features like:

- Deterministic crawls with `--seed`
- Encrypted storage with `--encrypt`
- Metrics export with `--metrics-json`
- JavaScript rendering with `--javascript`"

### Conclusion (8:00 - 10:00)

"That's it for getting started with crawlkit. Check out the documentation for more advanced features."

## Tutorial 2: Building Custom Plugins (15 minutes)

### Introduction (0:00 - 1:00)

"In this tutorial, we'll build a custom SEO analyzer plugin for crawlkit."

### Plugin Setup (1:00 - 3:00)

"First, create a new Rust project:

```bash
cargo new my-seo-plugin
cd my-seo-plugin
```

Add the plugin SDK to Cargo.toml:

```toml
[dependencies]
crawlkit-plugin-sdk = "1.0.0"
```

### Implementing the Analyzer (3:00 - 8:00)

"Now let's implement the analyzer:

```rust
use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};

pub struct MyAnalyzer;

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str { "my-analyzer" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        // Your analysis logic here
        findings
    }
}

crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
```

### Building for WASM (8:00 - 10:00)

"Build the plugin for WASM:

```bash
rustup target add wasm32-wasi
cargo build --target wasm32-wasi --release
```

### Installing the Plugin (10:00 - 12:00)

"Create a plugin manifest and install:

```bash
mkdir -p ~/.crawlkit/plugins/my-plugin
cp target/wasm32-wasi/release/my_plugin.wasm ~/.crawlkit/plugins/my-plugin/
cp crawlkit-plugin.toml ~/.crawlkit/plugins/my-plugin/
```

### Testing the Plugin (12:00 - 14:00)

"Test the plugin:

```bash
crawlkit plugin test ~/.crawlkit/plugins/my-plugin/ --html '<html>...</html>'
```

### Conclusion (14:00 - 15:00)

"You've built your first crawlkit plugin. Publish it to the marketplace to share with the community."

## Tutorial 3: Enterprise Deployment (20 minutes)

### Introduction (0:00 - 2:00)

"In this tutorial, we'll deploy crawlkit in an enterprise environment with SSO, RBAC, and multi-tenancy."

### Architecture Overview (2:00 - 4:00)

"The enterprise deployment includes:

- JWT authentication with RBAC
- OIDC integration for SSO
- Multi-tenant data isolation
- Audit logging"

### Setting Up Authentication (4:00 - 8:00)

"Configure JWT authentication:

```bash
export JWT_SECRET=$(openssl rand -hex 32)
crawlkit-api --port 8080
```

Create users and roles:

```bash
curl -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer <jwt>" \
  -d '{"email": "admin@example.com", "name": "Admin", "password": "secure-password", "roles": ["admin"]}'
```

### Configuring SSO (8:00 - 12:00)

"Set up OIDC with Google:

```bash
export OIDC_PROVIDER=google
export OIDC_CLIENT_ID=your-client-id
export OIDC_CLIENT_SECRET=your-client-secret
export OIDC_DISCOVERY_URL=https://accounts.google.com
export OIDC_REDIRECT_URI=https://your-domain.com/api/v1/auth/oidc/callback
```

### Multi-Tenancy (12:00 - 16:00)

"Create tenants and assign users:

```bash
curl -X POST http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer <jwt>" \
  -d '{"name": "Acme Corp", "plan": "professional"}'
```

### Monitoring and Compliance (16:00 - 19:00)

"Monitor with Prometheus metrics:

```bash
curl http://localhost:8080/metrics
```

View audit logs:

```bash
curl http://localhost:8080/api/v1/audit \
  -H "Authorization: Bearer <jwt>"
```

### Conclusion (19:00 - 20:00)

"You've deployed crawlkit in an enterprise environment. See the documentation for more details."
