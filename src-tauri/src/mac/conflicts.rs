use crate::{logs, util};

const DPI_PROCESSES: &[&str] = &["spoofdpi", "ciadpi", "byedpi", "goodbyedpi"];

pub fn clean() {
    let procs = util::process_names();
    for p in DPI_PROCESSES.iter().filter(|p| procs.iter().any(|n| n == *p)) {
        let killed = util::run("/usr/bin/pkill", &["-x", p]).map(|o| o.status.success()).unwrap_or(false);
        if killed {
            logs::warn(format!("{p} kapatıldı"));
        } else {
            logs::warn(format!("{p} çalışıyor, kapatılamadı"));
        }
    }
}
