fn main() {
    // Motor (winws + WinDivert) yönetici izni gerektirir; manifest UAC ile yükseltme ister.
    let manifest = include_str!("arunnel.exe.manifest");
    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest));
    tauri_build::try_build(attrs).expect("tauri build başarısız");
}
