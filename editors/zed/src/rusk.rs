use zed_extension_api::{self as zed, Result, settings::LspSettings};

const RUSK_LSP: &str = "rusk-lsp";

struct RuskExtension;

impl zed::Extension for RuskExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != RUSK_LSP {
            return Err(format!("unknown language server: {language_server_id}"));
        }

        let binary = LspSettings::for_worktree(RUSK_LSP, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let args = binary
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();

        if let Some(path) = binary.and_then(|binary| binary.path) {
            Ok(zed::Command {
                command: path,
                args,
                env: Default::default(),
            })
        } else if let Some(path) = worktree.which(RUSK_LSP) {
            Ok(zed::Command {
                command: path,
                args,
                env: Default::default(),
            })
        } else {
            Err("rusk-lsp not found in PATH; install it with `cargo install --path crates/rusk --bin rusk-lsp` or configure lsp.rusk-lsp.binary.path".to_string())
        }
    }
}

zed::register_extension!(RuskExtension);
