// ===== Tauri IPC =====
const __T = window.__TAURI__;
const invoke = __T?.core?.invoke || __T?.tauri?.invoke || __T?.invoke;
function getWin() {
  try {
    const w = __T?.window;
    if (w && typeof w.getCurrent === "function") return w.getCurrent();
    if (w && typeof w.getCurrentWindow === "function") return w.getCurrentWindow();
    if (__T?.core?.window) {
      const cw = __T.core.window;
      if (typeof cw.getCurrent === "function") return cw.getCurrent();
      if (typeof cw.getCurrentWindow === "function") return cw.getCurrentWindow();
    }
  } catch (_) {}
  return null;
}
async function startDrag() {
  try { const w = getWin(); if (w && typeof w.startDragging === "function") await w.startDragging(); } catch (_) {}
}

// ===== DOM =====
const greetingEl = document.getElementById("greeting");
const clockHm = document.getElementById("clock-hm");
const clockSec = document.getElementById("clock-sec");
const clockSep = document.getElementById("clock-sep");
const dpFill = document.getElementById("dp-fill");
const dpLabel = document.getElementById("dp-label");
const dateEl = document.getElementById("date");
const wEmoji = document.getElementById("w-emoji");
const wTemp = document.getElementById("w-temp");
const wText = document.getElementById("w-text");
const wCity = document.getElementById("w-city");
const wDetail = document.getElementById("w-detail");
const weatherEl = document.getElementById("weather");
const memoToggle = document.getElementById("memo-toggle");
const memoPanel = document.getElementById("memo-panel");
const memoText = document.getElementById("memo-text");
const memoStatus = document.getElementById("memo-status");
const memoClear = document.getElementById("memo-clear");
const quitBtn = document.getElementById("quit-btn");
const pinBtn = document.getElementById("pin-btn");
const cityPopover = document.getElementById("city-popover");
const cityInput = document.getElementById("city-input");
const citySave = document.getElementById("city-save");
const cityClose = document.getElementById("city-close");
const dragEls = document.querySelectorAll(".card-header, .greeting, .weather");

// ===== 问候 =====
function greet(h) {
  if (h >= 5 && h < 9) return "早安，小智陪你开启新的一天";
  if (h >= 9 && h < 12) return "上午好，小智在这里";
  if (h >= 12 && h < 14) return "午安，小智提醒你休息一下";
  if (h >= 14 && h < 18) return "下午好，小智在这里";
  if (h >= 18 && h < 22) return "晚上好，小智陪你";
  return "夜深了，小智伴你守候";
}
const WEEKDAYS = ["星期日","星期一","星期二","星期三","星期四","星期五","星期六"];

let lastGreeting = "", lastDate = "";
function updateClock() {
  const now = new Date();
  const g = greet(now.getHours());
  if (g !== lastGreeting) { lastGreeting = g; greetingEl.textContent = g; }
  const p = (n) => String(n).padStart(2, "0");
  clockHm.textContent = `${p(now.getHours())}:${p(now.getMinutes())}`;
  clockSec.textContent = p(now.getSeconds());
  // 冒号呼吸：随秒数明暗交替
  clockSep.classList.toggle("dim", now.getSeconds() % 2 === 1);
  const d = `${now.getFullYear()}年${now.getMonth()+1}月${now.getDate()}日 ${WEEKDAYS[now.getDay()]}`;
  if (d !== lastDate) { lastDate = d; dateEl.textContent = d; }
  // 今日进度：一天已过去多少
  const pct = (now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds()) / 864;
  dpFill.style.width = pct.toFixed(2) + "%";
  dpLabel.textContent = `今日 ${Math.floor(pct)}%`;
}
// 对齐到秒边界触发，避免 setInterval 的累积漂移丢秒
function tickClock() {
  updateClock();
  setTimeout(tickClock, 1000 - new Date().getMilliseconds() + 30);
}

// ===== 天气 emoji =====
function wmoEmoji(code) {
  if (code === 0) return "☀️";
  if (code === 1 || code === 2) return "⛅";
  if (code === 3) return "☁️";
  if (code === 45 || code === 48) return "🌫️";
  if (code >= 51 && code <= 55) return "🌦️";
  if ((code >= 56 && code <= 67) || (code >= 80 && code <= 82)) return "🌧️";
  if (code >= 71 && code <= 77) return "🌨️";
  if (code === 85 || code === 86) return "🌨️";
  if (code >= 95) return "⛈️";
  return "🌤️";
}
function errMsg(e) { return typeof e === "string" ? e : e?.message || "网络不可用"; }
function renderWeather(w, stale) {
  weatherEl.classList.remove("error");
  wEmoji.textContent = wmoEmoji(w.code);
  wTemp.textContent = w.temp ? `${w.temp}°` : "--°";
  wText.textContent = (stale ? "离线 · " : "") + (w.text || "");
  wCity.textContent = w.city || "";
  wDetail.textContent = [w.wind, w.humidity && `湿度 ${w.humidity}`].filter(Boolean).join(" · ");
  weatherEl.title = w.fetchedAt ? `更新于 ${new Date(w.fetchedAt).toLocaleString("zh-CN")}` : "";
}

let weatherRetry = null;
async function refreshWeather() {
  clearTimeout(weatherRetry);
  try {
    const w = await invoke("get_weather");
    renderWeather(w);
  } catch (e) {
    weatherEl.classList.add("error");
    wEmoji.textContent = "📡";
    const cached = await invoke("load_cached_weather").catch(() => null);
    if (cached) { renderWeather(cached, true); }
    else { wTemp.textContent = "--°"; wText.textContent = errMsg(e); wCity.textContent = ""; wDetail.textContent = ""; }
    // 失败后短周期重试，不等下一个 30 分钟
    weatherRetry = setTimeout(refreshWeather, 5 * 60 * 1000);
  }
}

// ===== 城市设置 =====
wCity.addEventListener("click", async () => {
  try { const saved = await invoke("load_city"); cityInput.value = saved || ""; } catch (_) {}
  cityPopover.hidden = false;
  setTimeout(() => cityInput.focus(), 50);
});
cityClose.addEventListener("click", () => { cityPopover.hidden = true; });
citySave.addEventListener("click", async () => {
  const city = cityInput.value.trim();
  if (!city) return;
  cityPopover.hidden = true;
  wText.textContent = "搜索中…";
  try {
    await invoke("save_city", { city });
    await refreshWeather();
  } catch (e) { wText.textContent = "城市未找到"; }
});
cityInput.addEventListener("keydown", (e) => { if (e.key === "Enter") citySave.click(); });
document.addEventListener("click", (e) => {
  if (cityPopover.hidden) return;
  if (!cityPopover.contains(e.target) && e.target !== wCity) cityPopover.hidden = true;
});

// ===== 备忘录 =====
async function loadMemo() {
  try {
    const memo = await invoke("load_memo");
    memoText.value = memo.content || "";
    if (memo.updated_at) memoStatus.textContent = `上次更新 ${new Date(memo.updated_at).toLocaleString("zh-CN")}`;
  } catch (_) { memoStatus.textContent = "加载失败"; }
}
let saveTimer = null;
function scheduleSave() {
  memoStatus.textContent = "输入中…"; memoStatus.classList.add("saving");
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    try { await invoke("save_memo", { content: memoText.value }); memoStatus.textContent = "已同步本地"; memoStatus.classList.remove("saving"); }
    catch (_) { memoStatus.textContent = "保存失败"; memoStatus.classList.remove("saving"); }
  }, 600);
}
memoText.addEventListener("input", scheduleSave);
memoClear.addEventListener("click", () => {
  try { if (memoText.value.trim() && !window.confirm("确定清空备忘录吗？")) return; } catch (_) {}
  memoText.value = ""; scheduleSave();
});

// ===== 备忘录折叠 =====
memoToggle.addEventListener("click", () => {
  const open = memoPanel.classList.toggle("open");
  memoToggle.classList.toggle("active", open);
  memoPanel.setAttribute("aria-hidden", String(!open));
  if (open) setTimeout(() => memoText.focus(), 320);
});

// ===== 窗口拖拽（固定时禁止） =====
dragEls.forEach((el) => {
  el.addEventListener("mousedown", (e) => {
    if (isPinned) return;
    const tag = e.target.tagName;
    if (["BUTTON","INPUT","TEXTAREA","A"].includes(tag)) return;
    if (e.target.closest("button") || e.target.closest("input") || e.target.closest("textarea")) return;
    if (e.target.closest(".popover")) return;
    if (e.target.closest("#w-city")) return;
    if (e.button !== 0) return;
    e.preventDefault();
    startDrag();
  });
});

// ===== 固定看板（窗口置顶，由后端返回实际状态） =====
let isPinned = false;
pinBtn.addEventListener("click", async () => {
  try { isPinned = await invoke("toggle_pin"); }
  catch (_) { isPinned = !isPinned; }
  pinBtn.classList.toggle("active", isPinned);
  pinBtn.title = isPinned ? "已悬浮置顶，点击放回桌面" : "悬浮置顶（当前贴在桌面）";
});

// ===== 退出 =====
quitBtn.addEventListener("click", async () => { try { await invoke("quit_app"); } catch (_) {} });

// ===== 启动 =====
async function init() {
  tickClock();
  await loadMemo();
  refreshWeather();
  setInterval(refreshWeather, 30 * 60 * 1000);
  // 备忘录默认展开，填满卡片下部
  memoPanel.classList.add("open");
  memoToggle.classList.add("active");
  memoPanel.setAttribute("aria-hidden", "false");
}
init();
