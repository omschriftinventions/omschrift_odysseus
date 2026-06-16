use std::path::{Path, PathBuf};

use tauri::Manager;

/// Default Odysseus HTTP port.
pub const DEFAULT_PORT: u16 = 7000;

/// Resolve the Odysseus repository root (directory containing `app.py`).
pub fn resolve_repo_root(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    if let Ok(root) = std::env::var("ODYSSEUS_ROOT") {
        let path = PathBuf::from(root);
        if is_repo_root(&path) {
            return Some(path);
        }
    }

    if let Some(app) = app {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let bundled = resource_dir.join("odysseus");
            if is_repo_root(&bundled) {
                return Some(bundled);
            }
            if is_repo_root(&resource_dir) {
                return Some(resource_dir);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(ref current) = dir {
            if is_repo_root(current) {
                return Some(current.clone());
            }
            let bundled = current.join("odysseus");
            if is_repo_root(&bundled) {
                return Some(bundled);
            }
            dir = current.parent().map(Path::to_path_buf);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = Some(cwd);
        while let Some(ref current) = dir {
            if is_repo_root(current) {
                return Some(current.clone());
            }
            dir = current.parent().map(Path::to_path_buf);
        }
    }

    None
}

pub fn is_repo_root(path: &Path) -> bool {
    path.join("app.py").is_file() && path.join("requirements.txt").is_file()
}

pub fn venv_python(repo_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        repo_root.join("venv").join("Scripts").join("python.exe")
    }
    #[cfg(not(windows))]
    {
        repo_root.join("venv").join("bin").join("python")
    }
}

pub fn server_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

pub fn health_url(port: u16) -> String {
    format!("{}/api/version", server_url(port))
}

pub fn desktop_log_file(data_root: &Path) -> PathBuf {
    data_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| data_root.to_path_buf())
        .join("logs")
        .join("odysseus-desktop-server.log")
}

pub fn setup_creds_file(data_root: &Path) -> PathBuf {
    data_root.join(".desktop-setup-creds.json")
}

/// PyInstaller onedir server shipped inside the desktop installer.
pub fn resolve_bundled_server(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    let rel = Path::new("omschrift-odysseus-server").join("omschrift-odysseus-server.exe");

    if let Some(app) = app {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let candidate = resource_dir.join(&rel);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(&rel);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn resolve_data_root(app: Option<&tauri::AppHandle>, native_bundle: bool) -> PathBuf {
    if let Ok(root) = std::env::var("ODYSSEUS_DATA_DIR") {
        return PathBuf::from(root);
    }

    if native_bundle {
        if let Some(app) = app {
            if let Ok(local) = app.path().app_local_data_dir() {
                return local.join("data");
            }
        }
        if let Ok(base) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(base)
                .join("com.odysseus.desktop")
                .join("data");
        }
    }

    resolve_repo_root(app).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }).join("data")
}
