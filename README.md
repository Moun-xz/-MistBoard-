<div align="center">

# 🌧️ MistBoard

**真·透明的 Windows 桌面看板 —— 像直接生长在壁纸上**

时钟 · 天气 · 备忘录，浮在桌面最底层，被应用窗口自然遮盖，随壁纸融为一体。

![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D4?logo=windows11&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green.svg)
![Size](https://img.shields.io/badge/exe-~4%20MB-9cf)

**[下载最新版](../../releases)** · [快速开始](#-快速开始) · [实现原理](#-实现原理) · [English](./README_EN.md)

<img src="docs/screenshot.png" width="480" alt="MistBoard 桌面截图"/>

</div>

---

## ✨ 特性

- **真透明**：不是半透明黑块，壁纸的水滴和纹理直接从看板底下透出来
- **贴在桌面**：看板始终压在所有应用窗口之下 —— 打开任何软件它都会被自然盖住，回到桌面就能看到
- **渐变时钟 + 今日进度条**：大字号渐变数字、呼吸冒号，一条细线告诉你这一天已经过去了多少
- **天气**：Open-Meteo 数据，30 分钟自动刷新，支持手动设置城市，断网自动显示缓存并快速重试
- **备忘录**：默认展开的便签，自动保存，重启不丢
- **位置记忆**：拖到哪儿，下次启动就在哪儿
- **开机自启（可选）**：顶栏 ⚡ 一键开关，开机登录后自动出现在桌面
- **Win+D 常驻**：按 Win+D 显示桌面时看板不会消失，永远等在壁纸上
- **极轻量**：单个 exe 约 4 MB，内存占用约 35 MB

## 📦 安装

### 方式一：直接下载（推荐）

前往 [Releases](../../releases) 下载 `MistBoard_x.y.z_x64-setup.exe` 安装，或下载绿色版 exe 直接运行。

> 系统要求：Windows 10 / 11（需自带或安装 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2)，Win11 已内置）

### 方式二：从源码构建

```bash
# 前置：安装 Rust (MSVC)、Node.js ≥ 18、WebView2
git clone https://github.com/你的用户名/MistBoard.git
cd MistBoard
npm install

npm run dev     # 开发模式
npm run build   # 构建产物在 src-tauri/target/release/
```

## 🖱️ 使用

| 操作 | 效果 |
|---|---|
| 按住时钟/天气区域拖动 | 移动看板（位置自动记忆） |
| 点击 📌 | 悬浮置顶 / 放回桌面 |
| 点击右上角城市名 | 设置天气城市 |
| 点击 🗒 | 展开 / 收起备忘录 |
| Win + D | 显示桌面，看板随时可见 |

## 🔍 实现原理

MistBoard 是一个**普通的顶层透明窗口**（Tauri 2 / WebView2），通过 `SetWindowPos(HWND_BOTTOM)` 常驻 Z 序最底层，并在失去焦点时自动归位，以此实现"贴在桌面上、被应用窗口自然遮盖"的效果。

> 踩坑记录：桌面小组件常用的 Progman/WorkerW 子窗口嵌入方案，在 DPI 缩放 ≠ 100% 的机器上，透明像素会被 DWM 垫上一层**缩放错位的壁纸拷贝**而非真实桌面。MistBoard 因此放弃了嵌入方案，才有了"真透明"。

## 📚 数据来源

- 天气数据：[Open-Meteo](https://open-meteo.com/)（免费、无需密钥）
- IP 定位：[ip-api.com](https://ip-api.com/)（仅在未设置城市时用于自动定位）

本项目不收集、不上传任何用户数据；备忘录与设置仅保存在本机 `%APPDATA%`。

## 🗺️ Roadmap

- [ ] 系统托盘菜单（显示/隐藏）
- [ ] 主题配置文件（自定义颜色 / 字号 / 模块开关）
- [ ] 多显示器适配
- [ ] 农历与节日显示
- [ ] 更多小组件：番茄钟、倒计时

欢迎提 Issue / PR！

## 📄 License

[MIT](./LICENSE) © 2026 MistBoard Contributors
