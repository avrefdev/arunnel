#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod conflicts;
mod dns;
mod engine;
mod logs;
mod probe;
mod settings;
mod util;

use serde::Serialize;
use settings::Settings;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const HISTORY: usize = 30;
const TRAY_OFF: &[u8] = include_bytes!("../icons/tray-off.png");
const TRAY_ON: &[u8] = include_bytes!("../icons/tray-on.png");
const TRAY_ERR: &[u8] = include_bytes!("../icons/tray-error.png");

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Idle,
    Connecting,
    Connected,
    Error,
}

struct Core {
    status: Status,
    profile: Option<usize>,
    since: Option<Instant>,
    pings: VecDeque<Option<u32>>,
    settings: Settings,
}

type Shared = Arc<Mutex<Core>>;

/// Tepsi menüsündeki "Bağlan / Bağlantıyı kes" öğesi.
struct TrayToggle(MenuItem<Wry>);

fn core(app: &AppHandle) -> Shared {
    app.state::<Shared>().inner().clone()
}

fn set_status(app: &AppHandle, status: Status) {
    {
        let c = core(app);
        let mut c = c.lock().unwrap();
        c.status = status;
        if status == Status::Connected {
            c.since = Some(Instant::now());
            c.pings.clear();
        } else if status != Status::Connecting {
            c.since = None;
        }
    }
    let (icon, tip, label) = match status {
        Status::Connected => (TRAY_ON, "Arunnel · Bağlı", "Bağlantıyı kes"),
        Status::Connecting => (TRAY_OFF, "Arunnel · Bağlanıyor", "Bağlanıyor…"),
        Status::Error => (TRAY_ERR, "Arunnel · Bağlanılamadı", "Bağlan"),
        Status::Idle => (TRAY_OFF, "Arunnel · Bağlı değil", "Bağlan"),
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Image::from_bytes(icon).ok());
        let _ = tray.set_tooltip(Some(tip));
    }
    if let Some(t) = app.try_state::<TrayToggle>() {
        let _ = t.0.set_text(label);
        let _ = t.0.set_enabled(status != Status::Connecting);
    }
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn set_logon_task(on: bool) {
    if on {
        let exe = std::env::current_exe().unwrap_or_default();
        let _ = util::run(
            "schtasks.exe",
            &format!(
                r#"/Create /TN "Arunnel" /TR "\"{}\" --tray" /SC ONLOGON /RL HIGHEST /F"#,
                exe.display()
            ),
        );
    } else {
        let _ = util::run("schtasks.exe", r#"/Delete /TN "Arunnel" /F"#);
    }
}

async fn do_connect(app: AppHandle) -> Result<(), String> {
    let (last, autostart) = {
        let c = core(&app);
        let c = c.lock().unwrap();
        if matches!(c.status, Status::Connecting | Status::Connected) {
            return Ok(());
        }
        (c.settings.last_profile, c.settings.start_with_windows)
    };
    set_status(&app, Status::Connecting);
    logs::info("Bağlantı başlatılıyor");

    tauri::async_runtime::spawn_blocking(conflicts::clean).await.ok();
    tauri::async_runtime::spawn_blocking(engine::extract).await.ok();

    match dns::apply().await {
        Ok(n) => logs::info(format!("{n} Discord adresi güvenli DNS ile çözüldü")),
        Err(e) => {
            logs::error(e.clone());
            set_status(&app, Status::Error);
            return Err(e);
        }
    }

    let n = engine::PROFILES.len();
    let order: Vec<usize> = std::iter::once(last % n).chain((0..n).filter(|&i| i != last % n)).collect();
    for p in order {
        logs::info(format!("Profil {} deneniyor", p + 1));
        let started = tauri::async_runtime::spawn_blocking(move || engine::start(p, autostart))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        if let Err(e) = started {
            // Motor hiç açılamıyorsa sorun profilde değildir; diğerlerini denemek boşuna.
            logs::error(e.clone());
            tauri::async_runtime::spawn_blocking(|| {
                engine::stop();
                dns::clear();
            })
            .await
            .ok();
            set_status(&app, Status::Error);
            return Err(e);
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
        if probe::discord_ok().await {
            {
                let c = core(&app);
                let mut c = c.lock().unwrap();
                c.profile = Some(p);
                c.settings.last_profile = p;
                settings::save(&c.settings);
            }
            set_status(&app, Status::Connected);
            logs::ok(format!("Bağlandı · Profil {}", p + 1));
            return Ok(());
        }
        logs::warn(format!("Profil {} ile Discord'a ulaşılamadı", p + 1));
    }

    tauri::async_runtime::spawn_blocking(|| {
        engine::stop();
        dns::clear();
    })
    .await
    .ok();
    logs::error("Hiçbir profil çalışmadı");
    set_status(&app, Status::Error);
    Err("Bağlanılamadı".into())
}

async fn do_disconnect(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(|| {
        engine::stop();
        engine::set_autostart(false);
        dns::clear();
    })
    .await
    .ok();
    core(&app).lock().unwrap().profile = None;
    set_status(&app, Status::Idle);
    logs::warn("Bağlantı kesildi");
}

// ---------- Arayüz komutları ----------

#[derive(Serialize)]
struct StateView {
    status: Status,
    uptime: u64,
    ping: Option<u32>,
    avg: Option<u32>,
    loss: u32,
    history: Vec<Option<u32>>,
}

#[tauri::command]
fn get_state(app: AppHandle) -> StateView {
    let c = core(&app);
    let c = c.lock().unwrap();
    let ok: Vec<u32> = c.pings.iter().flatten().copied().collect();
    let loss = if c.pings.is_empty() {
        0
    } else {
        (100 * (c.pings.len() - ok.len()) / c.pings.len()) as u32
    };
    StateView {
        status: c.status,
        uptime: c.since.map(|s| s.elapsed().as_secs()).unwrap_or(0),
        ping: c.pings.back().copied().flatten(),
        avg: (!ok.is_empty()).then(|| ok.iter().sum::<u32>() / ok.len() as u32),
        loss,
        history: c.pings.iter().copied().collect(),
    }
}

#[tauri::command]
async fn connect(app: AppHandle) -> Result<(), String> {
    do_connect(app).await
}

#[tauri::command]
async fn disconnect(app: AppHandle) {
    do_disconnect(app).await
}

#[derive(Serialize)]
struct Row {
    name: &'static str,
    value: String,
    tone: &'static str,
}

#[tauri::command]
async fn diagnose(app: AppHandle) -> (Vec<Row>, Vec<Row>) {
    logs::info("Tanılama testi çalıştırıldı");
    let services = probe::check_all()
        .await
        .into_iter()
        .map(|(name, ok)| Row {
            name,
            value: if ok { "Erişilebilir" } else { "Engelli" }.into(),
            tone: if ok { "ok" } else { "err" },
        })
        .collect();

    let procs = util::process_names();
    let has = |needles: &[&str]| procs.iter().any(|p| needles.iter().any(|n| p.contains(n)));
    let route = {
        let c = core(&app);
        let c = c.lock().unwrap();
        match (c.status, c.profile) {
            (Status::Connected, Some(p)) => format!("Doğrudan · Profil {}", p + 1),
            (Status::Connected, None) => "Doğrudan".into(),
            _ => "—".into(),
        }
    };
    let foreign = tauri::async_runtime::spawn_blocking(|| {
        engine::foreign_winws_running() || engine::foreign_driver_loaded()
    })
    .await
    .unwrap_or(false);
    let vpn = has(&[
        "openvpn", "wireguard", "warp-svc", "nordvpn", "protonvpn", "expressvpn", "surfshark",
        "windscribe", "hotspotshield", "wiresock", "cyberghost", "psiphon", "outline",
    ]);
    let compat = vec![
        Row { name: "Bağlantı türü", value: route, tone: "plain" },
        if has(&["exitlag"]) {
            Row { name: "ExitLag", value: "Uyumlu".into(), tone: "ok" }
        } else {
            Row { name: "ExitLag", value: "Yok".into(), tone: "muted" }
        },
        if has(&["goodbyedpi"]) || foreign {
            Row { name: "GoodbyeDPI / zapret", value: "Çakışıyor".into(), tone: "err" }
        } else {
            Row { name: "GoodbyeDPI / zapret", value: "Yok".into(), tone: "muted" }
        },
        if vpn {
            Row { name: "Diğer VPN", value: "Açık".into(), tone: "warn" }
        } else {
            Row { name: "Diğer VPN", value: "Yok".into(), tone: "muted" }
        },
    ];
    (services, compat)
}

#[tauri::command]
fn get_logs() -> Vec<logs::Entry> {
    logs::all()
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    core(&app).lock().unwrap().settings.clone()
}

#[tauri::command]
fn set_setting(app: AppHandle, key: String, value: bool) {
    let c = core(&app);
    let mut c = c.lock().unwrap();
    match key.as_str() {
        "start_with_windows" => {
            c.settings.start_with_windows = value;
            set_logon_task(value);
            engine::set_autostart(value && c.status == Status::Connected);
        }
        "auto_connect" => c.settings.auto_connect = value,
        "close_to_tray" => c.settings.close_to_tray = value,
        _ => return,
    }
    settings::save(&c.settings);
}

#[tauri::command]
fn close_window(app: AppHandle) {
    let to_tray = core(&app).lock().unwrap().settings.close_to_tray;
    if to_tray {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    } else {
        app.exit(0);
    }
}

#[tauri::command]
async fn uninstall(app: AppHandle) {
    logs::warn("Arunnel kaldırılıyor");
    tauri::async_runtime::spawn_blocking(|| {
        engine::uninstall();
        dns::clear();
        set_logon_task(false);
        let _ = std::fs::remove_file(util::data_dir().join("settings.json"));
    })
    .await
    .ok();
    app.exit(0);
}

// ---------- Arka plan döngüsü ----------

async fn background(app: AppHandle) {
    let mut tick: u64 = 0;
    let mut discord_was_open = true; // açılışta zaten açık olan Discord'u "yeni açıldı" sayma
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        tick += 1;
        let (status, auto) = {
            let c = core(&app);
            let c = c.lock().unwrap();
            (c.status, c.settings.auto_connect)
        };

        if status == Status::Connected {
            let p = probe::ping().await;
            {
                let c = core(&app);
                let mut c = c.lock().unwrap();
                c.pings.push_back(p);
                while c.pings.len() > HISTORY {
                    c.pings.pop_front();
                }
            }
            if tick % 5 == 0 {
                let state = tauri::async_runtime::spawn_blocking(engine::state).await;
                if !matches!(state, Ok(engine::ServiceState::Running)) {
                    logs::error("Motor beklenmedik şekilde durdu");
                    dns::clear();
                    set_status(&app, Status::Error);
                }
            }
            // Discord adresleri zamanla değişebilir; 30 dakikada bir tazele.
            if tick % 900 == 0 {
                if let Err(e) = dns::apply().await {
                    logs::warn(e);
                }
            }
        }

        if tick % 2 == 0 {
            let open = util::process_names().iter().any(|n| n == "discord.exe");
            if open && !discord_was_open && auto && matches!(status, Status::Idle | Status::Error) {
                logs::info("Discord açıldı, otomatik bağlanılıyor");
                let _ = do_connect(app.clone()).await;
            }
            discord_was_open = open;
        }
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Arunnel'i aç", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Bağlan", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çık", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(TrayToggle(toggle));

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(TRAY_OFF)?)
        .tooltip("Arunnel · Bağlı değil")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let connected = core(&app).lock().unwrap().status == Status::Connected;
                    if connected {
                        do_disconnect(app).await;
                    } else {
                        let _ = do_connect(app).await;
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    let settings = settings::load();
    let shared: Shared = Arc::new(Mutex::new(Core {
        status: Status::Idle,
        profile: None,
        since: None,
        pings: VecDeque::new(),
        settings,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .manage(shared)
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;

            engine::extract();
            let s = core(&handle).lock().unwrap().settings.clone();
            set_logon_task(s.start_with_windows);

            // Servis önceki oturumdan (ya da açılışta) çalışıyorsa bağlı kabul et.
            if engine::state() == engine::ServiceState::Running {
                core(&handle).lock().unwrap().profile = Some(s.last_profile);
                set_status(&handle, Status::Connected);
                logs::ok(format!("Servis zaten çalışıyor · Profil {}", s.last_profile + 1));
                tauri::async_runtime::spawn(async {
                    if let Err(e) = dns::apply().await {
                        logs::warn(e);
                    }
                });
            } else {
                dns::clear();
                logs::info("Servis hazır");
            }

            if !std::env::args().any(|a| a == "--tray") {
                show_window(&handle);
            }
            // Eski GoodbyeDPI/WARP/zapret kalıntılarını arka planda temizle.
            tauri::async_runtime::spawn_blocking(conflicts::clean);
            tauri::async_runtime::spawn(background(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Alt+F4 de kapat butonu gibi davransın.
                api.prevent_close();
                close_window(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            connect,
            disconnect,
            diagnose,
            get_logs,
            get_settings,
            set_setting,
            close_window,
            uninstall
        ])
        .run(tauri::generate_context!())
        .expect("Arunnel başlatılamadı");
}
