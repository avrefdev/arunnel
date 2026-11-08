use crate::{ctl, logs, util};
use std::time::Duration;

pub const PROFILES: &[&str] = &[
    "--split-pos=1,midsld --disorder",
    "--tlsrec=sniext --split-pos=1,midsld",
    "--split-pos=1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1 --disorder",
    "--tlsrec=midsld --split-pos=1 --disorder",
    "--split-pos=midsld --oob",
    "--tlsrec=sniext+1 --split-pos=host+1 --oob",
];

const PID: &str = "/var/run/arunnel-tpws.pid";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ServiceState {
    Missing,
    Stopped,
    Running,
}

fn our_pid() -> Option<u32> {
    std::fs::read_to_string(PID).ok()?.trim().parse().ok()
}

fn tpws_pids() -> Vec<u32> {
    util::run("/usr/bin/pgrep", &["-x", "tpws"])
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

pub fn state() -> ServiceState {
    if !ctl::installed() {
        return ServiceState::Missing;
    }
    match our_pid() {
        Some(p) if tpws_pids().contains(&p) => ServiceState::Running,
        _ => ServiceState::Stopped,
    }
}

pub fn extract() {
    if ctl::installed() && ctl::up_to_date() {
        return;
    }
    if let Err(e) = ctl::install() {
        logs::warn(e);
    }
}

pub fn foreign_driver_loaded() -> bool {
    false
}

pub fn start(profile: usize, autostart: bool, roblox: bool) -> Result<(), String> {
    if !ctl::installed() {
        return Err("Bağlantı bileşeni kurulu değil".into());
    }
    ctl::update(true, |r| {
        r.engine = true;
        r.profile = profile % PROFILES.len();
        r.roblox = roblox;
        r.autostart = autostart;
    })?;
    std::thread::sleep(Duration::from_millis(200));
    if state() == ServiceState::Running {
        Ok(())
    } else {
        Err("Motor başlatılamadı".into())
    }
}

pub fn stop() {
    if let Err(e) = ctl::update(true, |r| r.engine = false) {
        logs::warn(e);
    }
}

pub fn set_autostart(on: bool) {
    let _ = ctl::update(false, |r| r.autostart = on);
}

pub fn uninstall() {
    ctl::uninstall();
}

pub fn foreign_winws_running() -> bool {
    let ours = our_pid();
    tpws_pids().iter().any(|p| Some(*p) != ours)
}
