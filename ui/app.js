"use strict";

// Tauri dışında (tarayıcıda) açılırsa arayüzü denemek için sahte bir arka uç kullanılır.
const tauri = window.__TAURI__;
const invoke = tauri ? tauri.core.invoke : mockInvoke;

const $ = (id) => document.getElementById(id);
const pad = (n) => String(n).padStart(2, "0");
const clock = (s) => (s >= 3600 ? `${Math.floor(s / 3600)}:` : "") + `${pad(Math.floor(s / 60) % 60)}:${pad(s % 60)}`;

let page = "home";
let state = { status: "idle", uptime: 0, ping: null, avg: null, loss: 0, history: [] };
let logs = [];

// ---------- Sayfa geçişleri ----------
document.querySelectorAll(".nav").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".nav").forEach((b) => b.classList.toggle("active", b === btn));
    document.querySelectorAll(".page").forEach((p) => p.classList.remove("active"));
    page = btn.dataset.page;
    $(`page-${page}`).classList.add("active");
    if (page === "diag") runDiagnose();
    if (page === "logs") refreshLogs();
    if (page === "settings") loadSettings();
    render();
  });
});

// ---------- Pencere ----------
$("win-min").addEventListener("click", () => tauri?.window.getCurrentWindow().minimize());
$("win-close").addEventListener("click", () => invoke("close_window"));
document.addEventListener("contextmenu", (e) => e.preventDefault());

// ---------- Bağlan butonu ----------
$("power").addEventListener("click", async () => {
  if (state.status === "connecting") return;
  const wasConnected = state.status === "connected";
  state.status = "connecting";
  render();
  try {
    await invoke(wasConnected ? "disconnect" : "connect");
  } catch (_) {
    /* hata durumu get_state ile yansır */
  }
  await poll();
});

const STATUS = {
  idle: ["Bağlan", "Bağlı değil"],
  connecting: ["Bağlanıyor", "Bağlanıyor…"],
  connected: ["Bağlı", "Discord erişilebilir"],
  error: ["Tekrar dene", "Bağlanılamadı"],
};

function render() {
  const s = state.status;
  const power = $("power");
  power.className = `power ${s === "idle" ? "" : s}`;
  $("power-label").textContent = STATUS[s][0];
  $("status-title").textContent = STATUS[s][1];
  $("status-sub").textContent = s === "connected" ? clock(state.uptime) : "";

  if (page === "stats") {
    $("m-ping").textContent = state.ping ?? "—";
    $("m-avg").textContent = state.avg ?? "—";
    $("m-loss").textContent = s === "connected" && state.history.length ? state.loss : "—";
    $("m-uptime").textContent = clock(state.uptime);
    drawChart(state.history);
  }
}

function drawChart(history) {
  const values = history.filter((v) => v != null);
  const line = $("chart-line");
  if (values.length < 2) {
    line.setAttribute("points", "");
    return;
  }
  const max = Math.max(...values) * 1.25;
  const min = Math.max(0, Math.min(...values) * 0.75);
  const span = Math.max(max - min, 10);
  const step = 100 / (history.length - 1);
  const pts = [];
  history.forEach((v, i) => {
    if (v != null) pts.push(`${(i * step).toFixed(2)},${(58 - ((v - min) / span) * 54).toFixed(2)}`);
  });
  line.setAttribute("points", pts.join(" "));
}

async function poll() {
  try {
    state = await invoke("get_state");
  } catch (_) {}
  render();
  if (page === "logs") refreshLogs();
}

// ---------- Tanılama ----------
let diagBusy = false;
function rows(list) {
  return list
    .map((r) => `<div class="box"><span class="key">${r.name}</span><span class="val tone-${r.tone}">${r.value}</span></div>`)
    .join("");
}
async function runDiagnose() {
  if (diagBusy) return;
  diagBusy = true;
  $("diag-run").classList.add("busy");
  const wait = (names) => names.map((name) => ({ name, value: "Test ediliyor", tone: "wait" }));
  if (!$("diag-services").children.length) {
    $("diag-services").innerHTML = rows(wait(["Giriş", "Mesajlaşma", "Medya", "Sesli sohbet"]));
    $("diag-compat").innerHTML = rows(wait(["Bağlantı türü", "ExitLag", "GoodbyeDPI / zapret", "Diğer VPN"]));
  } else {
    document.querySelectorAll("#page-diag .val").forEach((v) => {
      v.className = "val tone-wait";
      v.textContent = "Test ediliyor";
    });
  }
  try {
    const [services, compat] = await invoke("diagnose");
    $("diag-services").innerHTML = rows(services);
    $("diag-compat").innerHTML = rows(compat);
  } catch (_) {}
  $("diag-run").classList.remove("busy");
  diagBusy = false;
}
$("diag-run").addEventListener("click", runDiagnose);

// ---------- Kayıtlar ----------
const esc = (s) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
async function refreshLogs() {
  try {
    logs = await invoke("get_logs");
  } catch (_) {}
  $("logs").innerHTML = logs
    .slice(-14)
    .map((l) => `<div><span class="t">${l.time}</span><span class="${l.level}">${esc(l.msg)}</span></div>`)
    .join("");
}
$("logs-copy").addEventListener("click", async () => {
  const text = logs.map((l) => `${l.time} ${l.msg}`).join("\n");
  try {
    await navigator.clipboard.writeText(text);
    const label = $("logs-copy").querySelector("span");
    label.textContent = "Kopyalandı";
    setTimeout(() => (label.textContent = "Kopyala"), 1500);
  } catch (_) {}
});

// ---------- Ayarlar ----------
async function loadSettings() {
  try {
    const s = await invoke("get_settings");
    document.querySelectorAll(".toggle").forEach((t) => (t.checked = !!s[t.dataset.key]));
  } catch (_) {}
}
document.querySelectorAll(".toggle").forEach((t) =>
  t.addEventListener("change", () => invoke("set_setting", { key: t.dataset.key, value: t.checked })),
);

let confirmTimer;
$("uninstall").addEventListener("click", () => {
  const b = $("uninstall");
  if (b.dataset.confirm) {
    b.textContent = "Kaldırılıyor…";
    invoke("uninstall");
    return;
  }
  b.dataset.confirm = "1";
  b.textContent = "Emin misiniz?";
  clearTimeout(confirmTimer);
  confirmTimer = setTimeout(() => {
    delete b.dataset.confirm;
    b.textContent = "Kaldır";
  }, 3000);
});

// ---------- Başlat ----------
loadSettings();
poll();
setInterval(poll, 1000);

// ---------- Tarayıcı önizlemesi için sahte arka uç ----------
function mockInvoke(cmd, args) {
  const m = (window.__mock ??= {
    st: { status: "idle", since: 0, history: [] },
    settings: { start_with_windows: true, auto_connect: false, close_to_tray: true },
    logs: [{ time: "12:00:00", msg: "Servis hazır", level: "info" }],
  });
  const log = (msg, level = "info") =>
    m.logs.push({ time: new Date().toTimeString().slice(0, 8), msg, level });
  const delay = (ms) => new Promise((r) => setTimeout(r, ms));
  switch (cmd) {
    case "get_state": {
      const s = m.st;
      if (s.status === "connected") {
        s.history.push(Math.round(30 + Math.random() * 12));
        if (s.history.length > 30) s.history.shift();
      }
      const ok = s.history.filter((v) => v != null);
      return Promise.resolve({
        status: s.status,
        uptime: s.status === "connected" ? Math.floor((Date.now() - s.since) / 1000) : 0,
        ping: ok.at(-1) ?? null,
        avg: ok.length ? Math.round(ok.reduce((a, b) => a + b, 0) / ok.length) : null,
        loss: 0,
        history: s.history,
      });
    }
    case "connect":
      m.st.status = "connecting";
      log("Profil 1 deneniyor");
      return delay(1800).then(() => {
        Object.assign(m.st, { status: "connected", since: Date.now(), history: [] });
        log("Bağlandı · Profil 1", "ok");
      });
    case "disconnect":
      return delay(300).then(() => {
        m.st.status = "idle";
        log("Bağlantı kesildi", "warn");
      });
    case "diagnose": {
      const on = m.st.status === "connected";
      return delay(900).then(() => [
        ["Giriş", "Mesajlaşma", "Medya", "Sesli sohbet"].map((name) => ({
          name, value: on ? "Erişilebilir" : "Engelli", tone: on ? "ok" : "err",
        })),
        [
          { name: "Bağlantı türü", value: on ? "Doğrudan · Profil 1" : "—", tone: "plain" },
          { name: "ExitLag", value: "Uyumlu", tone: "ok" },
          { name: "GoodbyeDPI / zapret", value: "Yok", tone: "muted" },
          { name: "Diğer VPN", value: "Yok", tone: "muted" },
        ],
      ]);
    }
    case "get_logs": return Promise.resolve(m.logs);
    case "get_settings": return Promise.resolve(m.settings);
    case "set_setting": m.settings[args.key] = args.value; return Promise.resolve();
    default: return Promise.resolve();
  }
}
