//! zapret (winws) motorunu yönetir.
//!
//! Motor dosyaları exe'nin içine gömülüdür ve ilk çalıştırmada
//! C:\ProgramData\Arunnel\engine klasörüne çıkarılır. winws, "Arunnel" adlı bir
//! Windows servisi olarak çalışır; böylece arayüz kapalıyken de bağlantı sürer.
//!
//! winws yalnızca discord.txt listesindeki alan adlarına ve Discord ses
//! portlarına müdahale eder. Diğer tüm trafik (oyunlar, ExitLag vb.) olduğu gibi geçer.

use crate::{logs, util};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SERVICE: &str = "Arunnel";

const FILES: &[(&str, &[u8])] = &[
    ("winws.exe", include_bytes!("../engine/winws.exe")),
    ("cygwin1.dll", include_bytes!("../engine/cygwin1.dll")),
    ("WinDivert.dll", include_bytes!("../engine/WinDivert.dll")),
    ("WinDivert64.sys", include_bytes!("../engine/WinDivert64.sys")),
    ("discord.txt", include_bytes!("../engine/discord.txt")),
    ("voice.txt", include_bytes!("../engine/voice.txt")),
];

/// Sırayla denenen TLS stratejileri. Operatörden operatöre hangisinin
/// çalıştığı değişir; ilk başarılı olan kaydedilir ve sonraki sefer önce denenir.
pub const PROFILES: &[&str] = &[
    "--dpi-desync=fake,multidisorder --dpi-desync-split-pos=midsld --dpi-desync-repeats=6 --dpi-desync-fooling=badseq,md5sig",
    "--dpi-desync=fake,multisplit --dpi-desync-split-pos=1,midsld --dpi-desync-repeats=8 --dpi-desync-fooling=md5sig --dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com",
    "--dpi-desync=multidisorder --dpi-desync-split-pos=1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1",
    "--dpi-desync=fake,fakedsplit --dpi-desync-autottl=2 --dpi-desync-fooling=md5sig",
    "--dpi-desync=syndata,multidisorder --dpi-desync-split-pos=2,midsld",
    "--dpi-desync=fake,multidisorder --dpi-desync-split-pos=1,midsld --dpi-desync-repeats=11 --dpi-desync-fooling=md5sig --dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com",
];

pub fn dir() -> PathBuf {
    util::data_dir().join("engine")
}

/// Windows yolu boşluk içeriyorsa servis komut satırı için tırnaklar.
fn q(p: PathBuf) -> String {
    let s = p.to_string_lossy().to_string();
    if s.contains(' ') { format!("\\\"{s}\\\"") } else { s }
}

fn command_line(profile: usize) -> String {
    let d = dir();
    let hosts = q(d.join("discord.txt"));
    let voice = q(d.join("voice.txt"));
    let tls = PROFILES[profile % PROFILES.len()];
    [
        q(d.join("winws.exe")),
        "--wf-tcp=443 --wf-udp=443".into(),
        format!("--wf-raw-part=@{voice}"),
        format!("--filter-tcp=443 --hostlist={hosts} {tls} --new"),
        format!("--filter-udp=443 --hostlist={hosts} --dpi-desync=fake --dpi-desync-repeats=6 --new"),
        "--filter-udp=50000-50099,19294-19344 --filter-l7=discord,stun --dpi-desync=fake --dpi-desync-repeats=6".into(),
    ]
    .join(" ")
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ServiceState {
    Missing,
    Stopped,
    Pending,
    Running,
}

pub fn state() -> ServiceState {
    let Ok(out) = util::run("sc.exe", &format!("query {SERVICE}")) else {
        return ServiceState::Missing;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    if out.status.code() == Some(1060) {
        ServiceState::Missing
    } else if text.contains("RUNNING") {
        ServiceState::Running
    } else if text.contains("STOPPED") {
        ServiceState::Stopped
    } else {
        ServiceState::Pending
    }
}

fn wait_for(target: ServiceState, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if state() == target {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

/// Gömülü motor dosyalarını diske yazar; dosya zaten aynıysa dokunmaz.
pub fn extract() {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    let outdated: Vec<_> = FILES
        .iter()
        .filter(|(name, data)| std::fs::read(d.join(name)).map(|b| b != *data).unwrap_or(true))
        .collect();
    if outdated.is_empty() {
        return;
    }
    let was_running = state() == ServiceState::Running;
    if was_running {
        stop();
    }
    for (name, data) in outdated {
        if let Err(e) = std::fs::write(d.join(name), data) {
            logs::warn(format!("{name} güncellenemedi: {e}"));
        }
    }
    if was_running {
        let _ = util::run("sc.exe", &format!("start {SERVICE}"));
    }
}

/// Hafızada yüklü WinDivert sürücüsü başka bir programa (ör. GoodbyeDPI) ait mi?
/// Farklı sürüm bir sürücü yüklüyken winws açılamaz.
pub fn foreign_driver_loaded() -> bool {
    let Ok(out) = util::run("sc.exe", "qc WinDivert") else { return false };
    if !out.status.success() {
        return false;
    }
    let ours = dir().to_string_lossy().to_lowercase();
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    let running = util::run("sc.exe", "query WinDivert")
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
        .unwrap_or(false);
    running && !text.contains(&ours)
}

/// Başka programın bıraktığı WinDivert sürücüsünü kaldırır. O program hâlâ
/// çalışıyorsa dokunmaz ve hata döndürür.
pub fn release_foreign_driver() -> Result<(), String> {
    if !foreign_driver_loaded() {
        return Ok(());
    }
    let procs = util::process_names();
    if procs.iter().any(|n| n == "goodbyedpi.exe") {
        return Err("GoodbyeDPI çalışıyor. Kapatıp tekrar deneyin".into());
    }
    if procs.iter().any(|n| n == "winws.exe" || n == "winws2.exe") {
        return Err("Başka bir zapret çalışıyor. Kapatıp tekrar deneyin".into());
    }
    logs::warn("Başka bir programdan kalan WinDivert sürücüsü kaldırılıyor");
    let _ = util::run("sc.exe", "stop WinDivert");
    std::thread::sleep(Duration::from_millis(500));
    let _ = util::run("sc.exe", "delete WinDivert");
    if foreign_driver_loaded() {
        return Err("Eski WinDivert sürücüsü kaldırılamadı. Bilgisayarı yeniden başlatın".into());
    }
    Ok(())
}

/// Servisi verilen strateji ile (yeniden) başlatır.
pub fn start(profile: usize, autostart: bool) -> Result<(), String> {
    release_foreign_driver()?;
    let bin = command_line(profile);
    let start_type = if autostart { "auto" } else { "demand" };
    if state() == ServiceState::Missing {
        let out = util::run(
            "sc.exe",
            &format!(r#"create {SERVICE} binPath= "{bin}" DisplayName= "Arunnel" start= {start_type}"#),
        )
        .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("Servis oluşturulamadı ({})", out.status.code().unwrap_or(-1)));
        }
        let _ = util::run(
            "sc.exe",
            &format!(r#"description {SERVICE} "Arunnel - yalnızca Discord trafiği için DPI aşma""#),
        );
    } else {
        stop();
        let _ = util::run("sc.exe", &format!(r#"config {SERVICE} binPath= "{bin}" start= {start_type}"#));
    }
    let _ = util::run("sc.exe", &format!("start {SERVICE}"));
    if wait_for(ServiceState::Running, Duration::from_secs(5)) {
        // winws hatalı parametrede hemen kapanır; kısa bir süre sonra tekrar kontrol et.
        std::thread::sleep(Duration::from_millis(400));
        if state() == ServiceState::Running {
            return Ok(());
        }
    }
    // 0x8032000A (FWP_E_IN_USE): başka bir programın WinDivert sürücüsü hâlâ çekirdekte
    // yüklü. Windows sürücüyü ancak yeniden başlatınca bırakır.
    let exit = util::run("sc.exe", &format!("query {SERVICE}"))
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();
    if exit.contains("8032000a") {
        return Err("Eski sürücü hafızada kaldı. Bilgisayarı yeniden başlatın".into());
    }
    Err("Motor başlatılamadı".into())
}

pub fn stop() {
    if matches!(state(), ServiceState::Running | ServiceState::Pending) {
        let _ = util::run("sc.exe", &format!("stop {SERVICE}"));
        wait_for(ServiceState::Stopped, Duration::from_secs(5));
    }
}

pub fn set_autostart(on: bool) {
    if state() != ServiceState::Missing {
        let t = if on { "auto" } else { "demand" };
        let _ = util::run("sc.exe", &format!("config {SERVICE} start= {t}"));
    }
}

/// Servisi, WinDivert sürücüsünü ve motor dosyalarını sistemden kaldırır.
pub fn uninstall() {
    stop();
    let _ = util::run("sc.exe", &format!("delete {SERVICE}"));
    let _ = util::run("sc.exe", "stop WinDivert");
    let _ = util::run("sc.exe", "delete WinDivert");
    let _ = std::fs::remove_dir_all(dir());
}

/// Arunnel dışında çalışan başka bir winws/zapret servisi var mı?
pub fn foreign_winws_running() -> bool {
    let ours = state() == ServiceState::Running;
    let count = util::process_names().iter().filter(|n| *n == "winws.exe" || *n == "winws2.exe").count();
    count > usize::from(ours)
}
