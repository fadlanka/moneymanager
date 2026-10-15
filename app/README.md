# Money Manager — Flutter app

Android (APK), web/PWA (for iPhone), and iOS (optional, needs a Mac).
Design: Claude Design "Budgeting App Modern".

## First-time setup

This folder only contains `lib/`, `test/`, `pubspec.yaml` and the Android manifests.
Generate the rest of the platform folders once (existing files are kept):

```bash
cd app
flutter create . --org com.fadlanka --project-name moneymanager --platforms android,ios,web
flutter pub get
flutter analyze
flutter test
```

`flutter create .` only adds missing files, so the committed
`android/app/src/main/AndroidManifest.xml` (share intent + singleTask) stays as is.

## Run

The app reads the API address at build time:

```bash
# Android emulator -> backend on your laptop
flutter run --dart-define=API_BASE_URL=http://10.0.2.2:8080/api

# Real phone on the same Wi-Fi (use your laptop's LAN IP)
flutter run --dart-define=API_BASE_URL=http://192.168.1.10:8080/api

# Production
flutter build apk --release --dart-define=API_BASE_URL=https://money.example.com/api
```

Plain `http://` only works in debug builds (`android/app/src/debug/AndroidManifest.xml`).
Release builds need HTTPS.

## Sharing receipts into the app

- **Android:** share a photo from Gallery or WhatsApp → choose *Money Manager* →
  the app goes straight to "Reading your receipt…".
- **Web (iPhone):** iOS doesn't let web apps receive shares. Use the Scan button
  (camera or photo library) inside the app.

## Web / PWA for iPhone users

```bash
flutter build web --release --dart-define=API_BASE_URL=https://money.example.com/api
```

Copy `build/web/` to the server (see `deploy/` in the repo root; Caddy serves it).
Before building, edit `web/manifest.json`: set `"name"`/`"short_name"` to `Money Manager`
and `"theme_color"`/`"background_color"` to `#0038FF` / `#F3F4F8`.

On the iPhone: open the site in Safari → Share → **Add to Home Screen**.

If the web app runs on a different origin than the API, add that origin to
`CORS_ORIGINS` in the backend `.env`.

## iOS native (optional)

Needs macOS + Xcode, and an Apple Developer account to install on friends' phones (TestFlight).

1. Add to `ios/Runner/Info.plist`:
   ```xml
   <key>NSCameraUsageDescription</key>
   <string>Take photos of receipts</string>
   <key>NSPhotoLibraryUsageDescription</key>
   <string>Choose receipt photos</string>
   ```
2. For the share sheet, add a Share Extension target and App Group as described in the
   `receive_sharing_intent` README (iOS section).

## Structure

```
lib/
  main.dart             wiring only
  app.dart              MaterialApp + sign-in gate
  config.dart           API_BASE_URL
  api/                  HTTP client + models (money is always int rupiah)
  state/providers.dart  Riverpod providers
  theme/                design tokens, category colors/icons
  l10n/strings.dart     all UI copy in one file (language still to decide)
  util/format.dart      rupiah/date formatting
  widgets/              shared UI pieces
  features/
    auth/ home/ activity/ scan/ transactions/ budget/ debts/ profile/ shell/
```
