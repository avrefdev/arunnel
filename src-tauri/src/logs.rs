use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

#[derive(Clone, Serialize)]
pub struct Entry {
    pub time: String,
    pub msg: String,
    pub level: &'static str,
}

static LOGS: Mutex<VecDeque<Entry>> = Mutex::new(VecDeque::new());
const MAX: usize = 300;

#[cfg(windows)]
fn now() -> String {
    // Yerel saat için chrono gibi bir bağımlılık eklemek yerine doğrudan Windows API'si.
    let mut st = unsafe { std::mem::zeroed::<SystemTime>() };
    unsafe { GetLocalTime(&mut st) };
    format!("{:02}:{:02}:{:02}", st.hour, st.minute, st.second)
}

#[cfg(windows)]
#[repr(C)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    ms: u16,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLocalTime(st: *mut SystemTime);
}

#[cfg(target_os = "macos")]
fn now() -> String {
    let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        libc::localtime_r(&t, &mut tm);
    }
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

fn push(level: &'static str, msg: String) {
    let entry = Entry { time: now(), msg, level };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(crate::util::data_dir().join("arunnel.log"))
    {
        let _ = writeln!(f, "{} [{}] {}", entry.time, entry.level, entry.msg);
    }
    let mut logs = LOGS.lock().unwrap();
    logs.push_back(entry);
    while logs.len() > MAX {
        logs.pop_front();
    }
}

pub fn info(msg: impl Into<String>) {
    push("info", msg.into());
}
pub fn ok(msg: impl Into<String>) {
    push("ok", msg.into());
}
pub fn warn(msg: impl Into<String>) {
    push("warn", msg.into());
}
pub fn error(msg: impl Into<String>) {
    push("error", msg.into());
}

pub fn all() -> Vec<Entry> {
    LOGS.lock().unwrap().iter().cloned().collect()
}
