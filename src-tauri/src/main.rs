#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(target_os = "linux")]
  if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
      .args(std::env::args_os().skip(1))
      .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
      .status()
      .expect("failed to restart with the WebKitGTK Wayland workaround");
    std::process::exit(status.code().unwrap_or(1));
  }

  pd2_x64_converter_tauri::run();
}
