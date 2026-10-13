//! Arunnel ile çakışan eski kurulumları temizler.
//!
//! - GoodbyeDPI, zapret, ByeDPI servisleri durdurulup silinir (program dosyalarına dokunulmaz).
//! - Cloudflare WARP ve WireSock bağlantısı kesilir, servisleri devre dışı bırakılır
//!   (programlar kaldırılmaz; istenirse servis yeniden etkinleştirilebilir).
//! - Başka programlardan kalan WinDivert sürücüleri kaldırılır.
//! - hosts dosyasında başka araçların eklediği eski Discord kayıtları silinir.
//! - Ölü bir yerel proxy'ye işaret eden sistem proxy ayarı kapatılır.
//!
//! ExitLag gibi oyun hızlandırıcılara dokunulmaz; Arunnel onlarla çakışmaz.

use crate::{dns, engine, logs, util};
use std::time::Duration;

/// Silinecek servis adı parçaları (küçük harf).
const DPI_SERVICES: &[&str] = &["goodbyedpi", "zapret", "winws", "byedpi", "ciadpi", "spoofdpi"];
/// Devre dışı bırakılacak tünel servisleri.
const TUNNEL_SERVICES: &[&str] = &["cloudflarewarp", "wiresock"];
/// Kapatılacak süreçler.
const DPI_PROCESSES: &[&str] = &["goodbyedpi.exe", "ciadpi.exe", "byedpi.exe", "spoofdpi.exe"];

fn services(kind: &str) -> Vec<String> {
    let Ok(out) = util::run("sc.exe", &format!("query type= {kind} state= all")) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().strip_prefix("SERVICE_NAME:"))
        .map(|s| s.trim().to_string())
        .collect()
}

fn is_running(name: &str) -> bool {
    util::run("sc.exe", &format!("query \"{name}\""))
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
        .unwrap_or(false)
}

fn stop_service(name: &str) {
    let _ = util::run("sc.exe", &format!("stop \"{name}\""));
    for _ in 0..20 {
        if !is_running(name) {
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

fn warp_disconnect() {
    for base in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(pf) = std::env::var(base) {
            let cli = std::path::Path::new(&pf).join(r"Cloudflare\Cloudflare WARP\warp-cli.exe");
            if cli.exists() {
                let _ = util::run(&cli.to_string_lossy(), "--accept-tos disconnect");
            }
        }
    }
}

/// Ölü bir 127.0.0.1 proxy'si Discord'un bağlanmasını engeller.
fn fix_dead_proxy() -> bool {
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let read = |v: &str| {
        util::run("reg.exe", &format!(r#"query "{KEY}" /v {v}"#))
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    };
    if !read("ProxyEnable").contains("0x1") {
        return false;
    }
    let server = read("ProxyServer");
    let Some(addr) = server
        .split_whitespace()
        .last()
        .map(|s| s.rsplit('=').next().unwrap_or(s).to_string())
    else {
        return false;
    };
    let local = addr.starts_with("127.") || addr.starts_with("localhost");
    if !local {
        return false;
    }
    let alive = addr
        .replace("localhost", "127.0.0.1")
        .parse::<std::net::SocketAddr>()
        .ok()
        .is_some_and(|a| std::net::TcpStream::connect_timeout(&a, Duration::from_millis(500)).is_ok());
    if alive {
        return false;
    }
    let _ = util::run("reg.exe", &format!(r#"add "{KEY}" /v ProxyEnable /t REG_DWORD /d 0 /f"#));
    true
}

/// Tüm çakışmaları temizler. Bir şey yapıldıysa kayıtlara yazar.
pub fn clean() {
    let ours_running = engine::state() == engine::ServiceState::Running;
    let mut did = Vec::new();

    // 1. DPI aşma servisleri: durdur ve sil.
    for name in services("service") {
        let lower = name.to_lowercase();
        if lower == "arunnel" || !DPI_SERVICES.iter().any(|d| lower.contains(d)) {
            continue;
        }
        stop_service(&name);
        let _ = util::run("sc.exe", &format!("delete \"{name}\""));
        did.push(format!("{name} servisi kaldırıldı"));
    }

    // 2. Tünel/VPN servisleri: bağlantıyı kes, devre dışı bırak.
    for name in services("service") {
        let lower = name.to_lowercase();
        if !TUNNEL_SERVICES.iter().any(|t| lower.contains(t)) {
            continue;
        }
        let disabled = util::run("sc.exe", &format!("qc \"{name}\""))
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("DISABLED"))
            .unwrap_or(false);
        if disabled && !is_running(&name) {
            continue;
        }
        if lower.contains("warp") {
            warp_disconnect();
        }
        stop_service(&name);
        let _ = util::run("sc.exe", &format!("config \"{name}\" start= disabled"));
        did.push(format!("{name} devre dışı bırakıldı"));
    }

    // 3. Elle çalıştırılmış DPI araçları.
    let procs = util::process_names();
    let mut kill: Vec<&str> = DPI_PROCESSES.iter().copied().filter(|p| procs.iter().any(|n| n == p)).collect();
    if !ours_running {
        kill.extend(["winws.exe", "winws2.exe"].into_iter().filter(|p| procs.iter().any(|n| n == p)));
    }
    for p in kill {
        let _ = util::run("taskkill.exe", &format!("/F /IM {p}"));
        did.push(format!("{p} kapatıldı"));
    }

    // 4. Başka programlardan kalan WinDivert sürücüleri.
    for name in services("driver") {
        let lower = name.to_lowercase();
        if !lower.starts_with("windivert") {
            continue;
        }
        if lower == "windivert" && !engine::foreign_driver_loaded() {
            continue; // Arunnel'in kendi sürücüsü
        }
        if lower == "windivert" && ours_running {
            continue;
        }
        stop_service(&name);
        let _ = util::run("sc.exe", &format!("delete \"{name}\""));
        did.push(format!("Eski {name} sürücüsü kaldırıldı"));
    }

    // 5. hosts dosyasındaki eski Discord kayıtları.
    let removed = dns::remove_foreign_entries();
    if removed > 0 {
        did.push(format!("hosts dosyasından {removed} eski Discord kaydı silindi"));
    }

    // 6. Ölü yerel proxy.
    if fix_dead_proxy() {
        did.push("Çalışmayan sistem proxy ayarı kapatıldı".into());
    }

    for d in did {
        logs::warn(d);
    }
}
