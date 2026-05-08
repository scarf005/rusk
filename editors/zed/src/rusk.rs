use zed_extension_api::{self as zed, Result, settings::LspSettings};

const RUSK_LSP: &str = "rusk-lsp";
const LOCAL_RUSK_LSP_SOURCE: &str = "crates/rusk/src/bin/rusk-lsp.rs";
const LOCAL_RUSK_MANIFEST: &str = "crates/rusk/Cargo.toml";

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
        } else if worktree.read_text_file(LOCAL_RUSK_LSP_SOURCE).is_ok()
            && let Some(path) = worktree.which("cargo")
        {
            Ok(zed::Command {
                command: path,
                args: [
                    "run".to_string(),
                    "--quiet".to_string(),
                    "--manifest-path".to_string(),
                    format!("{}/{}", worktree.root_path(), LOCAL_RUSK_MANIFEST),
                    "--bin".to_string(),
                    RUSK_LSP.to_string(),
                    "--".to_string(),
                ]
                .into_iter()
                .chain(args)
                .collect(),
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
