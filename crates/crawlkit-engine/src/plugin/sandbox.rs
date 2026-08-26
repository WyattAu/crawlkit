use super::PluginVerification;

/// Security configuration for WASM plugin execution.
#[derive(Clone)]
pub struct WasmConfig {
    /// Maximum fuel (instructions) a WASM plugin may consume before being
    /// killed. Prevents infinite loops / CPU exhaustion.
    pub max_fuel: u64,
    /// Maximum bytes the WASM linear memory may grow to.
    pub max_memory_bytes: usize,
    /// Wall-clock timeout for a single `analyze` call in milliseconds,
    /// enforced via wasmtime epoch interruption. A plugin that exceeds it
    /// traps with a deadline error instead of running indefinitely.
    pub max_analysis_timeout_ms: u64,
    /// Trust-chain policy applied to the manifest's hash/signature fields.
    pub plugin_verification: PluginVerification,
    /// When true, plugins whose manifest declares `permissions.network = true`
    /// may call the host `crawlkit_host.fetch` function (SSRF-validated,
    /// redirect-free, 1 MiB cap, 10 s timeout). The default is false:
    /// network access is deny-by-default even if the manifest requests it.
    pub allow_plugin_network: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            // ~10 billion instructions – generous for legitimate analysis
            // but prevents runaway loops.
            max_fuel: 10_000_000_000,
            // 64 MiB – sufficient for HTML processing without allowing
            // memory-bomb attacks.
            max_memory_bytes: 64 * 1024 * 1024,
            // 30 seconds per analysis call.
            max_analysis_timeout_ms: 30_000,
            // Fail-closed by default: unsigned/untrusted plugins are
            // rejected unless the embedder explicitly opts out.
            plugin_verification: PluginVerification::Required,
            // Network access deny-by-default; must be explicitly enabled
            // by the embedder in addition to the manifest declaring it.
            allow_plugin_network: false,
        }
    }
}
