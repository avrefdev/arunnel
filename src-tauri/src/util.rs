use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Konsol penceresi açmadan bir sistem komutu çalıştırır.
pub fn run(program: &str, raw_args: &str) -> std::io::Result<Output> {
    Command::new(program)
        .raw_arg(raw_args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// Uygulama verilerinin tutulduğu klasör: C:\ProgramData\Arunnel
pub fn data_dir() -> PathBuf {
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = PathBuf::from(base).join("Arunnel");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Çalışan süreçlerin adlarını (küçük harf) döndürür.
pub fn process_names() -> Vec<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut names = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return names;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
                names.push(String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase());
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    names
}
