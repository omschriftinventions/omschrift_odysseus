use std::fs;
use std::path::{Path, PathBuf};

use tauri::AppHandle;
use tauri::Manager;

use crate::paths::is_repo_root;

const BUNDLE_VERSION_FILE: &str = ".desktop-bundle-version";
const NATIVE_SERVER_BUNDLE_ID: &str = "native-server-2";
const NATIVE_SERVER_DIR: &str = "omschrift-odysseus-server";
const NATIVE_SERVER_EXE: &str = "omschrift-odysseus-server.exe";

/// True when the installer shipped the PyInstaller server folder.
pub fn has_native_server_bundle(app: &AppHandle) -> bool {
    bundled_server_resource(app)
        .map(|path| path.is_file())
        .unwrap_or(false)
}

fn bundled_server_resource(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    Some(
        resource_dir
            .join(NATIVE_SERVER_DIR)
            .join(NATIVE_SERVER_EXE),
    )
}

pub fn materialize_native_server(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled_src = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Could not resolve resource directory: {e}"))?
        .join(NATIVE_SERVER_DIR);

    let bundled_exe = bundled_src.join(NATIVE_SERVER_EXE);
    if !bundled_exe.is_file() {
        return Err(
            "Native server bundle was not found in the install. Reinstall Omschrift Odysseus.".into(),
        );
    }

    let runtime_root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {e}"))?
        .join("server");

    let target_exe = runtime_root.join(NATIVE_SERVER_EXE);
    let version = format!("{}:{}", env!("CARGO_PKG_VERSION"), NATIVE_SERVER_BUNDLE_ID);
    let marker = runtime_root.join(BUNDLE_VERSION_FILE);
    let installed_version = fs::read_to_string(&marker).unwrap_or_default();

    if !target_exe.is_file() || installed_version.trim() != version {
        if runtime_root.exists() {
            fs::remove_dir_all(&runtime_root).map_err(|e| {
                format!(
                    "Failed to refresh native server at {}: {e}",
                    runtime_root.display()
                )
            })?;
        }
        copy_dir_all(&bundled_src, &runtime_root).map_err(|e| {
            format!(
                "Failed to install native server to {}: {e}",
                runtime_root.display()
            )
        })?;
        fs::write(&marker, version).map_err(|e| {
            format!(
                "Failed to write server version marker at {}: {e}",
                marker.display()
            )
        })?;
    }

    if !target_exe.is_file() {
        return Err(format!(
            "Native server executable is missing at {}. Reinstall Omschrift Odysseus.",
            target_exe.display()
        ));
    }

    Ok(target_exe)
}

/// Resolve the writable Odysseus backend root used to run Python.
pub fn resolve_runtime_root(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("ODYSSEUS_ROOT") {
        let path = PathBuf::from(root);
        if is_repo_root(&path) {
            return Ok(path);
        }
        return Err(format!(
            "ODYSSEUS_ROOT does not look like an Odysseus install: {}",
            path.display()
        ));
    }

    if let Some(dev_root) = find_development_root(app) {
        return Ok(dev_root);
    }

    materialize_installed_runtime(app)
}

fn find_development_root(app: &AppHandle) -> Option<PathBuf> {
    let candidates = development_candidates(app);
    for candidate in candidates {
        if is_development_tree(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn development_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd);
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(current) = dir {
            out.push(current.clone());
            dir = current.parent().map(Path::to_path_buf);
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let mut dir = Some(resource_dir);
        while let Some(current) = dir {
            out.push(current.clone());
            dir = current.parent().map(Path::to_path_buf);
        }
    }

    out
}

fn is_development_tree(path: &Path) -> bool {
    is_repo_root(path) && path.join("desktop").is_dir()
}

fn materialize_installed_runtime(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Could not resolve resource directory: {e}"))?
        .join("odysseus");

    if !is_repo_root(&bundled) {
        return Err(
            "Installed Odysseus backend bundle was not found. Reinstall the desktop app.".into(),
        );
    }

    let runtime_root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {e}"))?
        .join("backend");

    let version = env!("CARGO_PKG_VERSION");
    let marker = runtime_root.join(BUNDLE_VERSION_FILE);
    let installed_version = fs::read_to_string(&marker).unwrap_or_default();

    if !is_repo_root(&runtime_root) || installed_version.trim() != version {
        if runtime_root.exists() {
            fs::remove_dir_all(&runtime_root).map_err(|e| {
                format!(
                    "Failed to refresh installed backend at {}: {e}",
                    runtime_root.display()
                )
            })?;
        }
        copy_dir_all(&bundled, &runtime_root).map_err(|e| {
            format!(
                "Failed to materialize Odysseus backend to {}: {e}",
                runtime_root.display()
            )
        })?;
        fs::write(&marker, version).map_err(|e| {
            format!(
                "Failed to write backend version marker at {}: {e}",
                marker.display()
            )
        })?;
    }

    Ok(runtime_root)
}

fn copy_dir_all(source: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_tree_requires_desktop_folder() {
        let temp = std::env::temp_dir().join("odysseus-runtime-test");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("app.py"), "print('ok')\n").unwrap();
        fs::write(temp.join("requirements.txt"), "fastapi\n").unwrap();
        assert!(!is_development_tree(&temp));
        fs::create_dir_all(temp.join("desktop")).unwrap();
        assert!(is_development_tree(&temp));
        let _ = fs::remove_dir_all(&temp);
    }
}
