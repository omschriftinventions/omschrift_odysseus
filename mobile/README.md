# Omschrift — Odysseus Mobile (whitelabel)

Expo / React Native app that wraps the [Odysseus](https://github.com/pewdiepie-archdaemon/odysseus)
web UI in a native WebView, whitelabeled with **Omschrift Inventions** branding
(logo + colors from omsinv.com) and an animated splash screen.

## What it does

1. Video splash: plays `assets/branding/splash.mp4` (oms_odysseus.mp4) full-screen, tap to skip, fades into the app. App icon is `oms_odysseus.png`.
2. Loads the Odysseus URL in a full-screen WebView.
3. In-app server switcher (⚙ button): preset **Server (LAN)** `http://187.127.137.38:7000/`
   and **Localhost** `http://localhost:7000/`, plus a custom URL field. Choice persisted with AsyncStorage.
4. Android hardware back navigates WebView history; pull-to-refresh; error screen with retry.

Cleartext HTTP is enabled (`usesCleartextTraffic` / iOS ATS `NSAllowsArbitraryLoads`)
so the plain-`http` Odysseus URLs load.

## Brand

| Token | Value |
|-------|-------|
| Primary blue | `#2563eb` |
| Primary dark | `#1d4ed8` |
| Accent purple | `#7c3aed` |
| Accent green | `#4ade80` |
| Slate bg | `#1e293b` / `#0f172a` |

Source logos in `assets/branding/`. Icons/splash regenerate with `node scripts/gen-assets.mjs` (needs `sharp`).

## Config

Edit `src/config.js` to change brand colors, the default URL, or URL presets.

## Run (development)

```bash
cd mobile
npm install
npx expo run:android    # builds a dev client onto a device/emulator
```

> `expo-video` is a native module, so the splash video does **not** run in
> Expo Go. Use `expo run:android` (or an EAS dev/preview build), not the QR + Expo Go flow.

> Note: `localhost:7000` only resolves on an emulator / the same machine.
> A physical phone must use the LAN IP (`187.127.137.38:7000`) or a tunnel.

## Build a release APK

> The agent sandbox on this machine blocks AF_UNIX sockets, so Gradle (which needs
> `java.nio.Selector`) cannot run inside Claude Code here. Run these in a normal
> terminal or Android Studio — they work fine outside the sandbox.

```powershell
cd mobile
$env:JAVA_HOME = "C:\Program Files\Java\jdk-17"     # or Android Studio JBR
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
npx expo prebuild -p android         # regenerates ./android (gitignored)
cd android
.\gradlew.bat assembleRelease        # APK -> app/build/outputs/apk/release/
# or assembleDebug for a quick test build
```

Or run straight onto a device/emulator:

```bash
npx expo run:android
```

### Cloud build (no local Android toolchain)

```bash
npm i -g eas-cli
eas login
eas build -p android --profile preview   # produces an installable APK
```

## Project layout

```
mobile/
  App.js                 WebView host, URL switcher, error/loading UI
  src/SplashScreen.js    video splash (expo-video), tap-to-skip + fade
  src/config.js          brand tokens + URL presets
  assets/branding/       splash.mp4, icon_source.png, source logos
  scripts/gen-assets.mjs regenerates icon/splash/adaptive icons via sharp
  app.json               Expo config: name, icons, splash, cleartext, package id
```
