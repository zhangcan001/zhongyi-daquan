# 安卓应用构建说明

《中医大全》当前基于 Tauri v2 + React。安卓版本走 Tauri Android 路线，保留现有 Rust/SQLite 后端能力，并复用前端页面。

## 当前状态

- 已加入 Android 脚本：
  - `npm run android:init`
  - `npm run android:dev`
  - `npm run android:build`
- 已补手机端响应式布局，知识库、导入中心、详情页在窄屏下改为单列。
- 主软件仍不解析 PDF；安卓端同样只处理标准数据包和已导入数据。

## 本机缺失环境

当前机器尚未检测到：

- Java / JDK
- Android SDK 命令行工具
- `adb`
- Rust Android targets

因此现在还不能直接生成 APK。运行 `npm run android:init` 会提示 Java 未安装。

## 环境安装

1. 安装 Android Studio。
2. 在 Android Studio 中安装：
   - Android SDK Platform
   - Android SDK Build-Tools
   - Android SDK Command-line Tools
   - Android NDK
3. 设置环境变量：
   - `JAVA_HOME`
   - `ANDROID_HOME`
   - `NDK_HOME`
4. 安装 Rust Android targets：

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

## 初始化安卓工程

```powershell
npm run android:init
```

初始化成功后会生成 `src-tauri/gen/android`。

## 开发运行

连接安卓手机或打开模拟器后：

```powershell
npm run android:dev
```

## 打包 APK/AAB

```powershell
npm run android:build
```

打包产物会出现在 Tauri Android 的 Gradle 输出目录中。

## 功能建议

安卓 MVP 建议主打查阅学习：

- 知识库浏览
- 搜索
- 详情页
- 收藏
- 查看导入批次数据

批量导入、回滚、备份恢复仍建议优先保留在桌面端作为管理功能。
