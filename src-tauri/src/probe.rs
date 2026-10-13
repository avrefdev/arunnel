//! Discord servislerine erişim testleri ve gecikme ölçümü.

use std::time::{Duration, Instant};

pub const SERVICES: &[(&str, &str)] = &[
    ("Giriş", "https://discord.com/api/v9/gateway"),
    ("Mesajlaşma", "https://gateway.discord.gg/"),
    ("Medya", "https://cdn.discordapp.com/embed/avatars/0.png"),
    ("Sesli sohbet", "https://latency.discord.media/"),
];

fn client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .no_proxy()
        .build()
        .expect("http istemcisi")
}

/// TLS el sıkışması tamamlanıp herhangi bir HTTP yanıtı geldiyse erişilebilir sayılır.
pub async fn reachable(url: &str, timeout: Duration) -> bool {
    client(timeout).get(url).send().await.is_ok()
}

/// Bağlantının gerçekten çalıştığını doğrular.
pub async fn discord_ok() -> bool {
    reachable(SERVICES[0].1, Duration::from_secs(4)).await
        && reachable(SERVICES[1].1, Duration::from_secs(4)).await
}

pub async fn check_all() -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    for (name, url) in SERVICES {
        out.push((*name, reachable(url, Duration::from_secs(5)).await));
    }
    out
}

/// discord.com:443'e TCP bağlantı süresi (ms).
pub async fn ping() -> Option<u32> {
    let start = Instant::now();
    let conn = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect("discord.com:443"),
    )
    .await;
    match conn {
        Ok(Ok(_)) => Some(start.elapsed().as_millis() as u32),
        _ => None,
    }
}
