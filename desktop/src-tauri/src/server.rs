use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Serialize;

use serde::Deserialize;

use tauri::AppHandle;

use crate::paths::{
    desktop_log_file, health_url, server_url, setup_creds_file, venv_python, DEFAULT_PORT,
};
use crate::runtime::materialize_native_server;

#[derive(Clone, Serialize, Default)]
pub struct StatusPayload {
    pub message: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_password: Option<String>,
}

#[derive(Clone)]
pub struct SetupCredentials {
    pub username: String,
    pub password: String,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub struct ServerManager {
    port: u16,
    data_root: PathBuf,
    bundled_server: Option<PathBuf>,
    venv_root: Option<PathBuf>,
    native_bundle_expected: bool,
    child: Arc<Mutex<Option<Child>>>,
    started_by_us: Arc<Mutex<bool>>,
}

impl ServerManager {
    pub fn new(
        data_root: PathBuf,
        bundled_server: Option<PathBuf>,
        venv_root: Option<PathBuf>,
        native_bundle_expected: bool,
    ) -> Self {
        Self {
            port: DEFAULT_PORT,
            data_root,
            bundled_server,
            venv_root,
            native_bundle_expected,
            child: Arc::new(Mutex::new(None)),
            started_by_us: Arc::new(Mutex::new(false)),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn app_url(&self) -> String {
        server_url(self.port)
    }

    pub fn is_healthy(&self) -> bool {
        ping_health(self.port)
    }

    pub fn ensure_running<F>(
        &self,
        app: &AppHandle,
        mut on_status: F,
    ) -> Result<Option<SetupCredentials>, String>
    where
        F: FnMut(StatusPayload),
    {
        let _ = fs::create_dir_all(&self.data_root);
        if let Some(parent) = desktop_log_file(&self.data_root).parent() {
            let _ = fs::create_dir_all(parent);
        }

        if self.is_healthy() {
            if self.needs_bootstrap() {
                return Err(
                    "First-time setup cannot continue while another server is using port 7000. \
                     Close launch-windows.ps1 or any other Odysseus instance, then restart \
                     Omschrift Odysseus."
                        .into(),
                );
            }
            on_status(StatusPayload {
                message: "Connecting to Omschrift Odysseus server…".into(),
                ..Default::default()
            });
            return Ok(None);
        }

        on_status(StatusPayload {
            message: if self.native_bundle_expected {
                "Preparing Omschrift Odysseus (native bundle, no Python install required)…".into()
            } else {
                "Checking Python environment…".into()
            },
            ..Default::default()
        });

        if self.native_bundle_expected || self.bundled_server.is_some() {
            let bundled = self.resolve_bundled_executable(app, &mut on_status)?;
            let creds = if self.needs_bootstrap() {
                on_status(StatusPayload {
                    message: "Running first-time setup…".into(),
                    ..Default::default()
                });
                self.run_bundled_setup(&bundled)?
            } else {
                None
            };
            self.finish_startup_bundled(&bundled, &mut on_status)?;
            return Ok(creds.or_else(|| read_setup_creds_file(&self.data_root)));
        }

        let repo_root = self
            .venv_root
            .clone()
            .ok_or_else(|| "Python runtime root was not configured.".to_string())?;

        let python = venv_python(&repo_root);
        if !python.is_file() {
            on_status(StatusPayload {
                message: "Creating virtual environment (first run)…".into(),
                ..Default::default()
            });
            self.create_venv(&repo_root, &mut on_status)?;
        }

        if !python.is_file() {
            return Err("Virtual environment was not created. Install Python 3.11+ and try again.".into());
        }

        let marker = repo_root.join("venv").join(".odysseus-desktop-ready");
        if !marker.is_file() {
            on_status(StatusPayload {
                message: "Installing Python dependencies (first run can take several minutes)…".into(),
                ..Default::default()
            });
            self.install_dependencies(&python, &repo_root, &mut on_status)?;
            on_status(StatusPayload {
                message: "Running first-time setup…".into(),
                ..Default::default()
            });
            let creds = self.run_setup(&python, &repo_root, &mut on_status)?;
            if let Some(parent) = marker.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&marker, b"ready");
            self.finish_startup(&python, &repo_root, &mut on_status)?;
            return Ok(creds);
        } else if self.needs_data_setup() {
            on_status(StatusPayload {
                message: "Running setup…".into(),
                ..Default::default()
            });
            let creds = self.run_setup(&python, &repo_root, &mut on_status)?;
            self.finish_startup(&python, &repo_root, &mut on_status)?;
            return Ok(creds);
        }

        self.finish_startup(&python, &repo_root, &mut on_status)?;
        Ok(None)
    }

    fn needs_bootstrap(&self) -> bool {
        if self.native_bundle_expected || self.bundled_server.is_some() {
            return self.needs_data_setup();
        }
        let repo_root = match &self.venv_root {
            Some(path) => path,
            None => return true,
        };
        let python = venv_python(repo_root);
        let marker = repo_root.join("venv").join(".odysseus-desktop-ready");
        !python.is_file() || !marker.is_file() || self.needs_data_setup()
    }

    fn needs_data_setup(&self) -> bool {
        let db = self.data_root.join("app.db");
        let auth = self.data_root.join("auth.json");
        !db.is_file() && !auth.is_file()
    }

    fn resolve_bundled_executable<F>(
        &self,
        app: &AppHandle,
        on_status: &mut F,
    ) -> Result<PathBuf, String>
    where
        F: FnMut(StatusPayload),
    {
        if let Some(path) = &self.bundled_server {
            if path.is_file() {
                return Ok(path.clone());
            }
        }

        if !self.native_bundle_expected {
            return Err(
                "Bundled Odysseus server was not found. Reinstall Omschrift Odysseus.".into(),
            );
        }

        on_status(StatusPayload {
            message: "Installing native server files (first run may take a minute)…".into(),
            ..Default::default()
        });
        materialize_native_server(app)
    }

    fn finish_startup_bundled<F>(&self, bundled: &Path, on_status: &mut F) -> Result<(), String>
    where
        F: FnMut(StatusPayload),
    {
        on_status(StatusPayload {
            message: "Starting Omschrift Odysseus server…".into(),
            ..Default::default()
        });
        self.spawn_bundled_server(bundled)?;

        on_status(StatusPayload {
            message: "Waiting for Omschrift Odysseus to become ready…".into(),
            ..Default::default()
        });
        self.wait_until_healthy(180, on_status)
    }

    fn run_bundled_setup(&self, bundled: &Path) -> Result<Option<SetupCredentials>, String> {
        let workdir = bundled.parent().unwrap_or_else(|| Path::new("."));
        let mut command = Command::new(bundled);
        command
            .current_dir(workdir)
            .arg("--setup-only")
            .env("ODYSSEUS_DATA_DIR", &self.data_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command
            .output()
            .map_err(|e| format!("Failed to run bundled setup: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let log = desktop_log_file(&self.data_root);
            return Err(format!(
                "Bundled setup failed ({}).\n{}\n{}\n\nLog file: {}",
                output.status, stderr, stdout, log.display()
            ));
        }

        Ok(read_setup_creds_file(&self.data_root).or_else(|| {
            parse_setup_credentials(&String::from_utf8_lossy(&output.stdout))
        }))
    }

    fn finish_startup<F>(&self, python: &Path, repo_root: &Path, on_status: &mut F) -> Result<(), String>
    where
        F: FnMut(StatusPayload),
    {
        on_status(StatusPayload {
            message: "Starting Omschrift Odysseus server…".into(),
            ..Default::default()
        });
        self.spawn_server(python, repo_root)?;

        on_status(StatusPayload {
            message: "Waiting for Omschrift Odysseus to become ready…".into(),
            ..Default::default()
        });
        self.wait_until_healthy(180, on_status)
    }

    pub fn shutdown(&self) {
        let mut guard = self.child.lock().expect("server child lock");
        if let Some(child) = guard.take() {
            kill_process_tree(child);
        }
        *self.started_by_us.lock().expect("started_by_us lock") = false;
    }

    fn create_venv<F>(&self, repo_root: &Path, on_status: &mut F) -> Result<(), String>
    where
        F: FnMut(StatusPayload),
    {
        fs::create_dir_all(repo_root).map_err(|e| {
            format!(
                "Could not prepare install folder {}: {e}",
                repo_root.display()
            )
        })?;

        let (launcher, args) = find_system_python().ok_or_else(|| {
            "Python 3.11+ was not found on this PC.\n\n\
             Use the native Omschrift Odysseus installer (bundles Python), or install Python from \
             https://www.python.org/downloads/ (check \"Add python.exe to PATH\"), then restart the app."
                .to_string()
        })?;

        on_status(StatusPayload {
            message: format!("Using {} to create virtual environment…", launcher.display()),
            ..Default::default()
        });

        let venv_path = repo_root.join("venv");
        let mut command = Command::new(&launcher);
        command
            .args(&args)
            .arg("-m")
            .arg("venv")
            .arg(&venv_path);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command
            .output()
            .map_err(|e| format!("Failed to create virtual environment: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let log = desktop_log_file(&self.data_root);
            return Err(format!(
                "Failed to create the virtual environment.\n{}\n{}\n\nLog file: {}",
                stderr, stdout, log.display()
            ));
        }
        Ok(())
    }

    fn install_dependencies<F>(&self, python: &Path, repo_root: &Path, on_status: &mut F) -> Result<(), String>
    where
        F: FnMut(StatusPayload),
    {
        run_python_command(
            python,
            repo_root,
            on_status,
            "Upgrading pip…",
            vec!["-m", "pip", "install", "--upgrade", "pip"],
        )?;

        on_status(StatusPayload {
            message: "Installing requirements.txt…".into(),
            ..Default::default()
        });

        let requirements = repo_root.join("requirements.txt");
        run_python_command(
            python,
            repo_root,
            on_status,
            "Installing Python packages…",
            vec![
                "-m",
                "pip",
                "install",
                "-r",
                requirements.to_string_lossy().as_ref(),
            ],
        )
    }

    fn run_setup<F>(
        &self,
        python: &Path,
        repo_root: &Path,
        on_status: &mut F,
    ) -> Result<Option<SetupCredentials>, String>
    where
        F: FnMut(StatusPayload),
    {
        on_status(StatusPayload {
            message: "Running setup.py…".into(),
            ..Default::default()
        });

        let output = Command::new(python)
            .current_dir(repo_root)
            .arg("setup.py")
            .output()
            .map_err(|e| format!("Failed to run setup.py: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "setup.py failed ({}).\n{}\n{}",
                output.status, stderr, stdout
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_setup_credentials(&stdout))
    }

    fn spawn_bundled_server(&self, bundled: &Path) -> Result<(), String> {
        if self.is_healthy() {
            return Ok(());
        }

        let log_path = desktop_log_file(&self.data_root);
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Could not open server log {}: {e}", log_path.display()))?;

        let workdir = bundled.parent().unwrap_or_else(|| Path::new("."));
        let mut command = Command::new(bundled);
        command
            .current_dir(workdir)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(self.port.to_string())
            .stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log_file))
            .env("ODYSSEUS_DATA_DIR", &self.data_root)
            .env("HF_HUB_DISABLE_SYMLINKS", "1")
            .env("HF_HUB_DISABLE_SYMLINKS_WARNING", "1");

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command
            .spawn()
            .map_err(|e| format!("Failed to start bundled Odysseus server: {e}"))?;

        *self.child.lock().expect("server child lock") = Some(child);
        *self.started_by_us.lock().expect("started_by_us lock") = true;
        Ok(())
    }

    fn spawn_server(&self, python: &Path, repo_root: &Path) -> Result<(), String> {
        if self.is_healthy() {
            return Ok(());
        }

        let log_path = desktop_log_file(&self.data_root);
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Could not open server log {}: {e}", log_path.display()))?;

        let mut command = Command::new(python);
        command
            .current_dir(repo_root)
            .arg("-m")
            .arg("uvicorn")
            .arg("app:app")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(self.port.to_string())
            .stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log_file))
            .env("ODYSSEUS_DATA_DIR", self.data_root.to_string_lossy().to_string())
            .env("HF_HUB_DISABLE_SYMLINKS", "1")
            .env("HF_HUB_DISABLE_SYMLINKS_WARNING", "1");

        apply_cuda_path(&mut command);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command
            .spawn()
            .map_err(|e| format!("Failed to start Odysseus server: {e}"))?;

        *self.child.lock().expect("server child lock") = Some(child);
        *self.started_by_us.lock().expect("started_by_us lock") = true;
        Ok(())
    }

    fn wait_until_healthy<F>(&self, timeout_secs: u64, on_status: &mut F) -> Result<(), String>
    where
        F: FnMut(StatusPayload),
    {
        let deadline = Duration::from_secs(timeout_secs);
        let started = std::time::Instant::now();

        while started.elapsed() < deadline {
            if ping_health(self.port) {
                return Ok(());
            }

            if let Some(code) = self.child_exit_status() {
                let log = desktop_log_file(&self.data_root);
                return Err(format!(
                    "Odysseus server exited early (code {code:?}). See {}",
                    log.display()
                ));
            }

            thread::sleep(Duration::from_millis(500));
        }

        on_status(StatusPayload {
            message: format!(
                "Server is still starting. You can open {} manually once ready.",
                self.app_url()
            ),
            ..Default::default()
        });
        Err("Timed out waiting for Odysseus to start.".into())
    }

    fn child_exit_status(&self) -> Option<std::process::ExitStatus> {
        let mut guard = self.child.lock().expect("server child lock");
        let child = guard.as_mut()?;
        match child.try_wait().ok()? {
            Some(status) => Some(status),
            None => None,
        }
    }
}

fn run_python_command<F>(
    python: &Path,
    repo_root: &Path,
    on_status: &mut F,
    label: &str,
    args: Vec<&str>,
) -> Result<(), String>
where
    F: FnMut(StatusPayload),
{
    on_status(StatusPayload {
        message: label.to_string(),
        ..Default::default()
    });

    let output = Command::new(python)
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {e}", python.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "Command failed ({}).\n{}\n{}",
        output.status, stderr, stdout
    ))
}

fn read_setup_creds_file(data_root: &Path) -> Option<SetupCredentials> {
    #[derive(Deserialize)]
    struct CredsFile {
        username: String,
        password: String,
    }

    let path = setup_creds_file(data_root);
    let text = fs::read_to_string(&path).ok()?;
    let parsed: CredsFile = serde_json::from_str(&text).ok()?;
    Some(SetupCredentials {
        username: parsed.username,
        password: parsed.password,
    })
}

fn parse_setup_credentials(stdout: &str) -> Option<SetupCredentials> {
    let mut username = None;
    let mut password = None;

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("  [ok] Initial admin user created (") {
            username = rest.strip_suffix(')').map(str::to_string);
        }
        if let Some(rest) = line.strip_prefix("        Temporary password: ") {
            password = Some(rest.trim().to_string());
        }
    }

    match (username, password) {
        (Some(username), Some(password)) => Some(SetupCredentials { username, password }),
        _ => None,
    }
}

fn ping_health(port: u16) -> bool {
    let client = match Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    client
        .get(health_url(port))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn find_system_python() -> Option<(PathBuf, Vec<String>)> {
    #[cfg(windows)]
    {
        let windows_py = PathBuf::from(r"C:\Windows\py.exe");
        if windows_py.is_file() {
            if let Some(found) = try_py_launcher_at(&windows_py, &["-3.13", "-3.12", "-3.11"]) {
                return Some(found);
            }
        }
        if let Some(found) = try_py_launcher_at(&PathBuf::from("py"), &["-3.13", "-3.12", "-3.11"]) {
            return Some(found);
        }
        if let Some(found) = try_common_windows_python() {
            return Some(found);
        }
    }

    try_python_cmd(&[])
        .or_else(|| try_python_cmd(&["python3"]))
        .or_else(|| try_python_cmd(&["python"]))
}

#[cfg(windows)]
fn try_common_windows_python() -> Option<(PathBuf, Vec<String>)> {
    let mut candidates = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(local).join("Programs").join("Python");
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("python.exe"));
            }
        }
    }
    if let Ok(prog) = std::env::var("ProgramFiles") {
        let base = PathBuf::from(prog);
        candidates.push(base.join("Python311").join("python.exe"));
        candidates.push(base.join("Python312").join("python.exe"));
        candidates.push(base.join("Python313").join("python.exe"));
    }

    for python in candidates {
        if !python.is_file() {
            continue;
        }
        if python_version_ok_output(&python) {
            return Some((python, Vec::new()));
        }
    }
    None
}

#[cfg(not(windows))]
fn try_common_windows_python() -> Option<(PathBuf, Vec<String>)> {
    None
}

#[cfg(windows)]
fn try_py_launcher_at(launcher: &Path, args: &[&str]) -> Option<(PathBuf, Vec<String>)> {
    use std::os::windows::process::CommandExt;

    for version in args {
        let mut command = Command::new(launcher);
        command
            .arg(version)
            .arg("-c")
            .arg("import sys; print('.'.join(map(str, sys.version_info[:3])))");
        command.creation_flags(0x0800_0000);
        let output = command.output().ok()?;
        if !output.status.success() {
            continue;
        }
        let ver = String::from_utf8_lossy(&output.stdout);
        if python_version_ok(&ver) {
            return Some((launcher.to_path_buf(), vec![version.to_string()]));
        }
    }
    None
}

fn python_version_ok_output(python: &Path) -> bool {
    let mut command = Command::new(python);
    command.arg("-c").arg("import sys; print('.'.join(map(str, sys.version_info[:3])))");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(_) => return false,
    };
    if !output.status.success() {
        return false;
    }
    python_version_ok(&String::from_utf8_lossy(&output.stdout))
}

fn try_python_cmd(launcher_args: &[&str]) -> Option<(PathBuf, Vec<String>)> {
    let launcher = if launcher_args.is_empty() {
        PathBuf::from("python")
    } else {
        PathBuf::from(launcher_args[0])
    };

    let mut command = Command::new(&launcher);
    command
        .arg("-c")
        .arg("import sys; print('.'.join(map(str, sys.version_info[:3])))");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let output = command.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let ver = String::from_utf8_lossy(&output.stdout);
    if python_version_ok(&ver) {
        Some((launcher, launcher_args[1..].iter().map(|s| (*s).to_string()).collect()))
    } else {
        None
    }
}

fn python_version_ok(version_text: &str) -> bool {
    let trimmed = version_text.trim();
    let mut parts = trimmed.split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    major > 3 || (major == 3 && minor >= 11)
}

#[cfg(windows)]
fn apply_cuda_path(command: &mut Command) {
    let cuda_base = PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA");
    if !cuda_base.is_dir() {
        return;
    }

    let mut best: Option<(PathBuf, String)> = None;
    if let Ok(entries) = fs::read_dir(&cuda_base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("bin").is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if best.as_ref().map(|(_, n)| &name > n).unwrap_or(true) {
                    best = Some((path, name));
                }
            }
        }
    }

    if let Some((path, _)) = best {
        command.env("CUDA_PATH", path);
    }
}

#[cfg(not(windows))]
fn apply_cuda_path(_command: &mut Command) {}

fn kill_process_tree(child: Child) {
    #[cfg(windows)]
    {
        let pid = child.id();
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }

    let mut child = child;
    let _ = child.kill();
    let _ = child.wait();
}
