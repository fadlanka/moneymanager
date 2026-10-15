/// Build-time configuration. Override with --dart-define:
///   flutter run --dart-define=API_BASE_URL=https://api.example.com/api
class AppConfig {
  /// 10.0.2.2 is the host machine as seen from the Android emulator.
  static const apiBaseUrl = String.fromEnvironment(
    'API_BASE_URL',
    defaultValue: 'http://10.0.2.2:8080/api',
  );
}
