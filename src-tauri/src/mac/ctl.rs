use crate::{dns, logs, util};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const ROOT: &str = "/Library/Application Support/Arunnel";

const FILES: &[(&str, &[u8])] = &[
    ("tpws", include_bytes!("../../engine/tpws")),
    ("apply.sh", include_bytes!("../../engine/macos/apply.sh")),
    ("install.sh", include_bytes!("../../engine/macos/install.sh")),
    ("uninstall.sh", include_bytes!("../../engine/macos/uninstall.sh")),
    ("app.arunnel.apply.plist", include_bytes!("../../engine/macos/app.arunnel.apply.plist")),
    ("discord.txt", include_bytes!("../../engine/discord.txt")),
    ("roblox.txt", include_bytes!("../../engine/roblox.txt")),
];

#[derive(Clone, Default)]
pub struct Request {
    pub seq: u64,
    pub engine: bool,
    pub profile: usize,
    pub roblox: bool,
    pub autostart: bool,
    pub hosts: Vec<String>,
}

static REQ: Mutex<Option<Request>> = Mutex::new(None);

fn root() -> PathBuf {
    PathBuf::from(ROOT)
}

fn request_path() -> PathBuf {
    root().join("ctl").join("request")
}

fn names() -> String {
    let mut s: Vec<&str> = dns::DOMAINS.iter().chain(dns::ROBLOX_DOMAINS).copied().collect();
    s.sort_unstable();
    s.dedup();
    s.join("\n") + "\n"
}

fn version() -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, data) in FILES {
        for b in name.bytes().chain(data.iter().copied()) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    for b in names().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}\n")
}

pub fn installed() -> bool {
    root().join("ctl").is_dir() && root().join("engine").join("apply.sh").is_file()
}

pub fn up_to_date() -> bool {
    std::fs::read_to_string(root().join("engine").join("version"))
        .map(|v| v == version())
        .unwrap_or(false)
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn admin(script: &str) -> Result<(), String> {
    let escaped = script.replace('\\', "\\\\").replace('"', "\\\"");
    let prompt = "Arunnel bağlantı bileşenini kurmak için izin istiyor.";
    let source = format!(r#"do shell script "{escaped}" with prompt "{prompt}" with administrator privileges"#);
    let out = util::run("/usr/bin/osascript", &["-e", &source]).map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else if String::from_utf8_lossy(&out.stderr).contains("-128") {
        Err("Yönetici izni verilmedi".into())
    } else {
        Err("Bileşen kurulamadı".into())
    }
}

fn stage() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("arunnel-setup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, data) in FILES {
        let bytes = if name.ends_with(".txt") {
            String::from_utf8_lossy(data).replace("\r\n", "\n").into_bytes()
        } else {
            data.to_vec()
        };
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    std::fs::write(dir.join("names.txt"), names()).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("version"), version()).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn install() -> Result<(), String> {
    let dir = stage()?;
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() {
        return Err("Kullanıcı adı alınamadı".into());
    }
    let script = format!(
        "/bin/sh {} {} {}",
        shell_quote(&dir.join("install.sh").to_string_lossy()),
        shell_quote(&dir.to_string_lossy()),
        shell_quote(&user)
    );
    let res = admin(&script);
    let _ = std::fs::remove_dir_all(&dir);
    res?;
    *REQ.lock().unwrap() = None;
    logs::info("Bağlantı bileşeni kuruldu");
    Ok(())
}

pub fn uninstall() {
    if !installed() {
        return;
    }
    let Ok(dir) = stage() else { return };
    let script = format!("/bin/sh {}", shell_quote(&dir.join("uninstall.sh").to_string_lossy()));
    if let Err(e) = admin(&script) {
        logs::warn(e);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn load() -> Request {
    let mut r = Request::default();
    let Ok(text) = std::fs::read_to_string(request_path()) else { return r };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match k {
            "seq" => r.seq = v.parse().unwrap_or(0),
            "engine" => r.engine = v == "1",
            "profile" => r.profile = v.parse().unwrap_or(0),
            "roblox" => r.roblox = v == "1",
            "autostart" => r.autostart = v == "1",
            "host" => r.hosts.push(v.to_string()),
            _ => {}
        }
    }
    r
}

fn render(r: &Request) -> String {
    let b = |v: bool| if v { "1" } else { "0" };
    let mut s = format!(
        "seq={}\nengine={}\nprofile={}\nroblox={}\nautostart={}\n",
        r.seq,
        b(r.engine),
        r.profile,
        b(r.roblox),
        b(r.autostart)
    );
    for h in &r.hosts {
        s.push_str(&format!("host={h}\n"));
    }
    s
}

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)
}

fn wait_ack(seq: u64) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(12) {
        if let Ok(text) = std::fs::read_to_string(root().join("ack")) {
            let text = text.trim();
            let (num, rest) = text.split_once(' ').unwrap_or((text, ""));
            if num.parse::<u64>().ok() == Some(seq) {
                return match rest.split_once(' ') {
                    Some(("err", msg)) => Err(msg.to_string()),
                    _ if rest == "ok" => Ok(()),
                    _ => Err("Motor yanıt vermedi".into()),
                };
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("Motor yanıt vermedi".into())
}

pub fn current() -> Request {
    let mut g = REQ.lock().unwrap();
    g.get_or_insert_with(load).clone()
}

pub fn update(wait: bool, f: impl FnOnce(&mut Request)) -> Result<(), String> {
    if !installed() {
        return Ok(());
    }
    let seq = {
        let mut g = REQ.lock().unwrap();
        let r = g.get_or_insert_with(load);
        f(r);
        r.seq += 1;
        write_atomic(&request_path(), &render(r)).map_err(|e| format!("Motor ayarı yazılamadı: {e}"))?;
        r.seq
    };
    if wait {
        wait_ack(seq)
    } else {
        Ok(())
    }
}
