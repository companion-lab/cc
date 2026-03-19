/// cc-lsp: LSP client stub — Phase 4 implementation
/// For now, exposes empty stubs so cc-tools can compile.

pub struct LspClient;

impl LspClient {
    pub fn new() -> Self { Self }

    /// Diagnostics for a given file path (empty until Phase 4)
    pub fn diagnostics_for(&self, _path: &std::path::Path) -> Vec<String> {
        vec![]
    }
}

impl Default for LspClient {
    fn default() -> Self { Self::new() }
}
