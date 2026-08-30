use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// 备忘录数据
#[derive(Serialize, Deserialize, Clone)]
struct MemoData { content: String, updated_at: String }

/// ip-api.com IP 定位返回（免费接口，中文城市名 + 经纬度）
#[derive(Deserialize, Clone)]
struct IpLocateResp {
    status: String,
    #[serde(default)] city: Option<String>,
    #[serde(default)] lat: Option<f64>,
    #[serde(default)] lon: Option<f64>,
}

/// Open-Meteo 城市搜索返回
#[derive(Deserialize, Clone)]
struct GeoSearch { #[serde(default)] results: Option<Vec<GeoItem>> }
#[derive(Deserialize, Clone)]
struct GeoItem { name: String, latitude: f64, longitude: f64 }

/// 城市坐标缓存：城市名不变时跳过地理编码请求
#[derive(Serialize, Deserialize, Clone)]
struct GeoCache { name: String, lat: f64, lon: f64 }

/// Open-Meteo 天气返回
#[derive(Deserialize, Clone)]
struct OwmResponse { current: OwmCurrent }
#[derive(Deserialize, Clone)]
struct OwmCurrent {
    #[serde(default)] temperature_2m: Option<f64>,
    #[serde(default)] weather_code: Option<i32>,
    #[serde(default)] wind_speed_10m: Option<f64>,
    #[serde(default)] relative_humidity_2m: Option<i64>,
}

/// 返回给前端的天气数据
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WeatherInfo {
    city: String,
    temp: String,
    text: String,
    code: i32,
    wind: String,
    humidity: String,
    #[serde(default)]
    fetched_at: String,
}

/// 窗口位置（物理像素）
#[derive(Deserialize)]
struct WinPos { x: i32, y: i32 }

/// 共享 HTTP 客户端，复用连接池避免每次刷新重新握手
struct HttpClient(reqwest::Client);

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
fn memo_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { Ok(data_dir(app)?.join("memo.json")) }
fn weather_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { Ok(data_dir(app)?.join("weather_cache.json")) }
fn city_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { Ok(data_dir(app)?.join("city.txt")) }
fn geo_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { Ok(data_dir(app)?.join("geo_cache.json")) }
fn window_pos_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { Ok(data_dir(app)?.join("window_pos.json")) }

fn wmo_text(code: i32) -> &'static str {
    match code {
        0 => "晴", 1|2 => "多云", 3 => "阴", 45|48 => "雾",
        51|53|55 => "毛毛雨", 56|57 => "冻雨",
        61 => "小雨", 63 => "中雨", 65 => "大雨", 66|67 => "冻雨",
        71|73 => "小雪", 75 => "大雪", 77 => "米雪",
        80|81 => "阵雨", 82 => "大阵雨", 85|86 => "阵雪",
        95|96|99 => "雷暴", _ => "未知",
    }
}
fn wind_desc(s: f64) -> &'static str {
    if s < 5.0 {"微风"} else if s < 12.0 {"轻风"} else if s < 20.0 {"和风"} else if s < 28.0 {"强风"} else {"大风"}
}

#[tauri::command]
fn load_memo(app: tauri::AppHandle) -> Result<MemoData, String> {
    let path = memo_path(&app)?;
    if path.exists() {
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str::<MemoData>(&data).map_err(|e| e.to_string())
    } else { Ok(MemoData { content: String::new(), updated_at: String::new() }) }
}

#[tauri::command]
fn save_memo(app: tauri::AppHandle, content: String) -> Result<MemoData, String> {
    let memo = MemoData { updated_at: chrono::Local::now().to_rfc3339(), content };
    let path = memo_path(&app)?;
    let json = serde_json::to_string_pretty(&memo).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(memo)
}

#[tauri::command]
fn save_city(app: tauri::AppHandle, city: String) -> Result<(), String> {
    fs::write(city_path(&app)?, &city).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_city(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let path = city_path(&app)?;
    if path.exists() {
        let city = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let trimmed = city.trim().to_string();
        if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
    } else { Ok(None) }
}

/// 解析城市坐标：坐标缓存命中则跳过地理编码，未命中时请求并写缓存
async fn resolve_geo(app: &tauri::AppHandle, client: &reqwest::Client, city: &str) -> Result<GeoCache, String> {
    if let Ok(Some(cached)) = fs::read_to_string(geo_cache_path(app)?)
        .map(|s| serde_json::from_str::<GeoCache>(&s).ok())
    {
        if cached.name == city {
            return Ok(cached);
        }
    }

    let geo_resp = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", city), ("language", "zh"), ("count", "1")])
        .send().await
        .map_err(|e| format!("城市搜索失败: {}", e))?;
    let geo: GeoSearch = geo_resp.json().await.map_err(|e| format!("解析城市失败: {}", e))?;
    let item = geo.results.and_then(|r| r.into_iter().next()).ok_or("未找到该城市")?;

    let cache = GeoCache { name: item.name, lat: item.latitude, lon: item.longitude };
    if let Ok(path) = geo_cache_path(app) {
        if let Ok(json) = serde_json::to_string(&cache) {
            let _ = fs::write(path, json);
        }
    }
    Ok(cache)
}

/// 用城市名搜索经纬度 + 获取天气
#[tauri::command]
async fn get_weather_by_city(
    app: tauri::AppHandle,
    client: tauri::State<'_, HttpClient>,
    city: String,
) -> Result<WeatherInfo, String> {
    let geo = resolve_geo(&app, &client.0, &city).await?;
    fetch_weather(&app, &client.0, &geo.name, geo.lat, geo.lon).await
}

/// 自动定位 + 天气（优先用保存的城市）
#[tauri::command]
async fn get_weather(app: tauri::AppHandle, client: tauri::State<'_, HttpClient>) -> Result<WeatherInfo, String> {
    // 优先用保存的城市（带坐标缓存）
    if let Ok(Some(city)) = load_city(app.clone()) {
        if !city.is_empty() {
            if let Ok(geo) = resolve_geo(&app, &client.0, &city).await {
                return fetch_weather(&app, &client.0, &geo.name, geo.lat, geo.lon).await;
            }
        }
    }

    // IP 定位兜底（ip-api.com 免费接口）
    let resp = client.0
        .get("http://ip-api.com/json/?lang=zh-CN&fields=status,city,lat,lon")
        .send().await
        .map_err(|e| format!("定位请求失败: {}", e))?;
    let loc: IpLocateResp = resp.json().await.map_err(|e| format!("解析定位失败: {}", e))?;
    if loc.status != "success" {
        return Err("IP 定位失败".to_string());
    }
    let city = loc.city.ok_or("定位数据为空")?;
    let lat = loc.lat.ok_or("定位数据为空")?;
    let lon = loc.lon.ok_or("定位数据为空")?;

    fetch_weather(&app, &client.0, &city, lat, lon).await
}

async fn fetch_weather(app: &tauri::AppHandle, client: &reqwest::Client, city: &str, lat: f64, lon: f64) -> Result<WeatherInfo, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code,wind_speed_10m,relative_humidity_2m&timezone=auto",
        lat, lon
    );
    let resp = client.get(&url).send().await.map_err(|e| format!("天气请求失败: {}", e))?;
    let owm: OwmResponse = resp.json().await.map_err(|e| format!("解析天气失败: {}", e))?;

    let cur = owm.current;
    let code = cur.weather_code.unwrap_or(0);
    let temp = cur.temperature_2m.map(|t| format!("{:.0}", t.round())).unwrap_or_else(|| "--".to_string());
    let ws = cur.wind_speed_10m.unwrap_or(0.0);
    let h = cur.relative_humidity_2m.unwrap_or(0);

    let info = WeatherInfo {
        city: city.to_string(), temp,
        text: wmo_text(code).to_string(), code,
        wind: format!("{} {:.0}km/h", wind_desc(ws), ws),
        humidity: format!("{}%", h),
        fetched_at: chrono::Local::now().to_rfc3339(),
    };
    let cache = serde_json::to_string(&info).map_err(|e| e.to_string())?;
    let _ = fs::write(weather_cache_path(app)?, cache);
    Ok(info)
}

#[tauri::command]
fn load_cached_weather(app: tauri::AppHandle) -> Result<Option<WeatherInfo>, String> {
    let path = weather_cache_path(&app)?;
    if path.exists() {
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let w: WeatherInfo = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(Some(w))
    } else { Ok(None) }
}

/// 把窗口压到普通窗口的最底层：壁纸之上、其他应用窗口之下
#[cfg(windows)]
fn bottom_hwnd(hwnd: *mut std::ffi::c_void) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };
    unsafe {
        SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}
#[cfg(not(windows))]
fn bottom_hwnd(_hwnd: *mut std::ffi::c_void) {}

/// 开机自启：HKCU\...\Run 注册表键
const AUTOSTART_VALUE: &str = "MistBoard";
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
fn autostart_enabled() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE};
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_QUERY_VALUE)
        .and_then(|k| k.get_value::<String, _>(AUTOSTART_VALUE))
        .is_ok()
}

#[cfg(windows)]
fn set_autostart_registry(enable: bool) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE)
        .map_err(|e| e.to_string())?;
    if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(AUTOSTART_VALUE, &exe.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    } else {
        let _ = key.delete_value(AUTOSTART_VALUE);
        Ok(())
    }
}

#[cfg(not(windows))]
fn autostart_enabled() -> bool { false }
#[cfg(not(windows))]
fn set_autostart_registry(_enable: bool) -> Result<(), String> { Ok(()) }

#[tauri::command]
fn get_autostart() -> bool {
    autostart_enabled()
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<(), String> {
    set_autostart_registry(enable)
}

/// 📌 切换悬浮置顶；默认贴在桌面，被应用窗口盖住
#[tauri::command]
fn toggle_pin(window: tauri::WebviewWindow) -> Result<bool, String> {
    let pinned = !window.is_always_on_top().map_err(|e| e.to_string())?;
    window.set_always_on_top(pinned).map_err(|e| e.to_string())?;
    Ok(pinned)
}

fn write_window_pos(app: &tauri::AppHandle, pos: tauri::PhysicalPosition<i32>) {
    if let Ok(path) = window_pos_path(app) {
        let _ = fs::write(path, serde_json::to_string(&serde_json::json!({ "x": pos.x, "y": pos.y })).unwrap_or_default());
    }
}

fn load_window_pos(app: &tauri::AppHandle) -> Option<WinPos> {
    let path = window_pos_path(app).ok()?;
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

#[tauri::command]
fn quit_app(window: tauri::WebviewWindow) {
    if let Ok(pos) = window.outer_position() {
        write_window_pos(window.app_handle(), pos);
    }
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build http client");

    tauri::Builder::default()
        .manage(HttpClient(http))
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // 去掉可最小化样式：Win+D/显示桌面会跳过本窗口，看板常驻桌面
                #[cfg(windows)]
                if let Ok(raw) = window.hwnd() {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
                    };
                    let hwnd = raw.0 as *mut std::ffi::c_void;
                    unsafe {
                        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                        SetWindowLongPtrW(hwnd, GWL_STYLE, style & !((WS_MINIMIZEBOX | WS_MAXIMIZEBOX) as isize));
                    }
                }
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let win_w = 260.0 * scale;
                    let margin = 16.0 * scale;

                    // 恢复上次位置；已脱离当前屏幕（如断开的显示器）时回到右上角
                    let saved = load_window_pos(app.handle()).filter(|p| {
                        p.x > -(win_w as i32 - 40)
                            && p.y > -40
                            && p.x < size.width as i32 - 40
                            && p.y < size.height as i32 - 40
                    });
                    let pos = match saved {
                        Some(p) => tauri::PhysicalPosition::new(p.x, p.y),
                        None => tauri::PhysicalPosition::new(
                            (size.width as f64 - win_w - margin) as i32,
                            margin as i32,
                        ),
                    };
                    let _ = window.set_position(pos);
                }

                // 贴到最底层：壁纸之上、应用窗口之下
                #[cfg(windows)]
                if let Ok(raw) = window.hwnd() {
                    bottom_hwnd(raw.0 as *mut std::ffi::c_void);
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // Alt+F4 等系统关闭路径也要保住窗口位置
                tauri::WindowEvent::CloseRequested { .. } => {
                    if let Ok(pos) = window.outer_position() {
                        write_window_pos(window.app_handle(), pos);
                    }
                }
                // 失去焦点后回到最底层，保持"在桌面上"
                tauri::WindowEvent::Focused(false) => {
                    #[cfg(windows)]
                    if let Ok(raw) = window.hwnd() {
                        bottom_hwnd(raw.0 as *mut std::ffi::c_void);
                    }
                }
                // Win+D 最小化时窗口尺寸变为 0x0：无激活地恢复并压回底层，保证回到桌面就能看到
                tauri::WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        #[cfg(windows)]
                        if let Ok(raw) = window.hwnd() {
                            use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
                            let hwnd = raw.0 as *mut std::ffi::c_void;
                            unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE); }
                            bottom_hwnd(hwnd);
                        }
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_memo, save_memo,
            get_weather, get_weather_by_city, load_cached_weather,
            save_city, load_city,
            toggle_pin, quit_app,
            get_autostart, set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
