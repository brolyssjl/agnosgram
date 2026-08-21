//! Port of `src/core/writeFile.ts`: the shared write-if-changed helper every
//! agnosgram-owned or managed-block target routes through.

use std::fs;
use std::path::Path;

use crate::core::output::UserError;

/// `src/core/writeFile.ts`'s `WriteAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    Created,
    Updated,
    Unchanged,
}

impl WriteAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            WriteAction::Created => "created",
            WriteAction::Updated => "updated",
            WriteAction::Unchanged => "unchanged",
        }
    }
}

/// `src/core/writeFile.ts`'s `WriteResult`.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub path: String,
    pub action: WriteAction,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WriteOpts {
    pub executable: bool,
}

/// Port of `writeIfChanged`: write `content` to `root/relPath` only when it
/// differs from what is already on disk, creating parent directories as
/// needed, and report which of created/updated/unchanged happened.
pub fn write_if_changed(
    root: &Path,
    rel_path: &str,
    content: &str,
    opts: WriteOpts,
) -> Result<WriteResult, UserError> {
    let target = root.join(rel_path);
    let existed_before = target.exists();
    let before = if existed_before {
        Some(fs::read_to_string(&target).map_err(|e| UserError::new(e.to_string()))?)
    } else {
        None
    };
    let unchanged = before.as_deref() == Some(content);
    if !unchanged {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| UserError::new(e.to_string()))?;
        }
        fs::write(&target, content).map_err(|e| UserError::new(e.to_string()))?;
    }
    if opts.executable {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
                .map_err(|e| UserError::new(e.to_string()))?;
        }
    }
    let action = if unchanged {
        WriteAction::Unchanged
    } else if existed_before {
        WriteAction::Updated
    } else {
        WriteAction::Created
    };
    Ok(WriteResult {
        path: rel_path.to_string(),
        action,
    })
}
