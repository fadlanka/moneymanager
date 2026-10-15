import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'features/auth/login_screen.dart';
import 'features/shell/app_shell.dart';
import 'l10n/strings.dart';
import 'state/providers.dart';
import 'theme/app_theme.dart';
import 'widgets/common.dart';

class MoneyManagerApp extends ConsumerWidget {
  const MoneyManagerApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    return MaterialApp(
      title: S.appName,
      debugShowCheckedModeBanner: false,
      theme: buildTheme(),
      home: auth.restoring
          ? const Scaffold(body: Loading())
          : auth.signedIn
              // Keyed by user so all per-user state resets on account switch.
              ? AppShell(key: ValueKey(auth.user!.id))
              : const LoginScreen(),
    );
  }
}
