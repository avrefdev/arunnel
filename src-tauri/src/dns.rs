//! Discord alan adlarının DNS engelini aşar.
//!
//! Operatör DNS'i Discord için sahte adres döndürdüğünden, bu alan adlarını
//! şifreli DNS (DoH) ile çözüp hosts dosyasına işaretli bir blok olarak yazarız.
//! Sistem DNS ayarlarına dokunulmaz; bağlantı kesilince blok silinir.

use crate::{logs, util};
use serde::Deserialize;
use std::time::Duration;

const BEGIN: &str = "# >>> Arunnel (otomatik eklendi, elle düzenlemeyin)";
const END: &str = "# <<< Arunnel";

pub const DOMAINS: &[&str] = &[
    "discord.com",
    "www.discord.com",
    "canary.discord.com",
    "ptb.discord.com",
    "status.discord.com",
    "support.discord.com",
    "updates.discord.com",
    "cdn.discord.com",
    "discord.gg",
    "gateway.discord.gg",
    "discord.media",
    "latency.discord.media",
    "discordapp.com",
    "www.discordapp.com",
    "cdn.discordapp.com",
    "dl.discordapp.net",
    "media.discordapp.net",
    "images-ext-1.discordapp.net",
    "images-ext-2.discordapp.net",
    "router.discordapp.net",
    "discordcdn.com",
    "discordstatus.com",
    "dis.gd",
];

pub const ROBLOX_DOMAINS: &[&str] = &[
    "roblox.com",
    "www.roblox.com",
    "web.roblox.com",
    "auth.roblox.com",
    "apis.roblox.com",
    "users.roblox.com",
    "accountsettings.roblox.com",
    "accountinformation.roblox.com",
    "friends.roblox.com",
    "presence.roblox.com",
    "games.roblox.com",
    "gamejoin.roblox.com",
    "develop.roblox.com",
    "thumbnails.roblox.com",
    "avatar.roblox.com",
    "catalog.roblox.com",
    "economy.roblox.com",
    "inventory.roblox.com",
    "groups.roblox.com",
    "badges.roblox.com",
    "chat.roblox.com",
    "notifications.roblox.com",
    "realtime-signalr.roblox.com",
    "assetdelivery.roblox.com",
    "clientsettings.roblox.com",
    "clientsettingscdn.roblox.com",
    "ecsv2.roblox.com",
    "ephemeralcounters.api.roblox.com",
    "setup.rbxcdn.com",
    "tr.rbxcdn.com",
    "js.rbxcdn.com",
    "css.rbxcdn.com",
    "images.rbxcdn.com",
    "static.rbxcdn.com",
];

const RESOLVERS: &[&str] = &["https://1.1.1.1/dns-query", "https://8.8.8.8/resolve"];

#[derive(Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer", default)]
    answer: Vec<DohAnswer>,
}

#[derive(Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .expect("http istemcisi")
}

async fn resolve(client: &reqwest::Client, name: &str) -> Option<String> {
    for url in RESOLVERS {
        let res = client
            .get(*url)
            .query(&[("name", name), ("type", "A")])
            .header("accept", "application/dns-json")
            .send()
            .await;
        if let Ok(r) = res {
            if let Ok(body) = r.json::<DohResponse>().await {
                if let Some(a) = body.answer.into_iter().find(|a| a.kind == 1) {
                    return Some(a.data);
                }
            }
        }
    }
    None
}

fn hosts_path() -> std::path::PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    std::path::PathBuf::from(root).join(r"System32\drivers\etc\hosts")
}

/// Hosts içeriğinden Arunnel bloğunu çıkarır.
fn strip_block(content: &str) -> String {
    let mut out = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if line.trim() == BEGIN {
            inside = true;
        } else if line.trim() == END {
            inside = false;
        } else if !inside {
            out.push(line);
        }
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.join("\r\n")
}

fn write_hosts(content: &str) -> std::io::Result<()> {
    let path = hosts_path();
    // Bazı sistemlerde hosts salt okunur işaretli olur.
    if let Ok(meta) = std::fs::metadata(&path) {
        let mut perms = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(&path, perms);
    }
    std::fs::write(&path, content)?;
    let _ = util::run("ipconfig.exe", "/flushdns");
    Ok(())
}

/// Discord alan adlarını DoH ile çözüp hosts dosyasına yazar.
pub async fn apply(roblox: bool) -> Result<usize, String> {
    let client = client();
    let mut lines = Vec::new();
    let extra: &[&str] = if roblox { ROBLOX_DOMAINS } else { &[] };
    for name in DOMAINS.iter().chain(extra) {
        if let Some(ip) = resolve(&client, name).await {
            lines.push(format!("{ip} {name}"));
        }
    }
    if lines.is_empty() {
        return Err("Güvenli DNS'e ulaşılamadı".into());
    }
    let current = std::fs::read_to_string(hosts_path()).unwrap_or_default();
    let mut content = strip_block(&current);
    content.push_str(&format!("\r\n\r\n{BEGIN}\r\n{}\r\n{END}\r\n", lines.join("\r\n")));
    write_hosts(&content).map_err(|e| format!("hosts yazılamadı: {e}"))?;
    Ok(lines.len())
}

const DISCORD_ZONES: &[&str] = &[
    "discord.com", "discord.gg", "discord.media", "discordapp.com", "discordapp.net",
    "discordcdn.com", "discordstatus.com", "dis.gd",
];

fn is_discord_host(host: &str) -> bool {
    let h = host.to_lowercase();
    DISCORD_ZONES.iter().any(|z| h == *z || h.ends_with(&format!(".{z}")))
}

/// Başka araçların hosts dosyasına eklediği (Arunnel bloğu dışındaki) Discord
/// kayıtlarını siler. Eski IP'ler Discord'un bağlanmasını bozar. Silinen satır sayısını döndürür.
pub fn remove_foreign_entries() -> usize {
    let Ok(current) = std::fs::read_to_string(hosts_path()) else { return 0 };
    let mut out = Vec::new();
    let mut inside = false;
    let mut removed = 0;
    for line in current.lines() {
        let t = line.trim();
        if t == BEGIN {
            inside = true;
        } else if t == END {
            inside = false;
        } else if !inside && !t.starts_with('#') {
            let mut parts = t.split('#').next().unwrap_or("").split_whitespace();
            let _ip = parts.next();
            let hosts: Vec<&str> = parts.collect();
            if !hosts.is_empty() && hosts.iter().all(|h| is_discord_host(h)) {
                removed += 1;
                continue;
            }
        }
        out.push(line);
    }
    if removed > 0 {
        let mut content = out.join("\r\n");
        content.push_str("\r\n");
        if let Err(e) = write_hosts(&content) {
            logs::warn(format!("hosts temizlenemedi: {e}"));
            return 0;
        }
    }
    removed
}

/// Arunnel'in hosts dosyasına eklediği satırları kaldırır.
pub fn clear() {
    let Ok(current) = std::fs::read_to_string(hosts_path()) else { return };
    if !current.contains(BEGIN) {
        return;
    }
    let mut content = strip_block(&current);
    content.push_str("\r\n");
    if let Err(e) = write_hosts(&content) {
        logs::warn(format!("hosts temizlenemedi: {e}"));
    }
}
