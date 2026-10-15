import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../config.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';

class ProfileScreen extends ConsumerStatefulWidget {
  const ProfileScreen({super.key});

  @override
  ConsumerState<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends ConsumerState<ProfileScreen> {
  late final TextEditingController _name;
  late final TextEditingController _income;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    final u = ref.read(authProvider).user!;
    _name = TextEditingController(text: u.name);
    _income = TextEditingController(text: u.monthlyIncome == 0 ? '' : groupThousands(u.monthlyIncome));
  }

  @override
  void dispose() {
    _name.dispose();
    _income.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() => _busy = true);
    try {
      final user = await ref
          .read(apiProvider)
          .updateMe(name: _name.text.trim(), monthlyIncome: parseRupiah(_income.text) ?? 0);
      ref.read(authProvider.notifier).updateUser(user);
      refreshData(ref);
      if (mounted) showSnack(context, S.saved);
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final user = ref.watch(authProvider).user;
    if (user == null) return const Scaffold(body: Loading());
    return Scaffold(
      appBar: AppBar(title: const Text(S.profile, style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700))),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Center(
            child: CircleAvatar(
              radius: 36,
              backgroundColor: AppColors.blue,
              child: Text(user.initials,
                  style: const TextStyle(color: Colors.white, fontSize: 22, fontWeight: FontWeight.w800)),
            ),
          ),
          const SizedBox(height: 8),
          Text(user.email, textAlign: TextAlign.center, style: const TextStyle(color: AppColors.muted)),
          const SizedBox(height: 24),
          TextField(controller: _name, decoration: const InputDecoration(labelText: S.name)),
          const SizedBox(height: 12),
          TextField(
            controller: _income,
            keyboardType: TextInputType.number,
            inputFormatters: [RupiahInputFormatter()],
            decoration: const InputDecoration(labelText: S.monthlyIncome, prefixText: 'Rp '),
          ),
          const SizedBox(height: 16),
          FilledButton(onPressed: _busy ? null : _save, child: const Text(S.save)),
          const SizedBox(height: 32),
          OutlinedButton.icon(
            onPressed: () {
              Navigator.popUntil(context, (r) => r.isFirst);
              ref.read(authProvider.notifier).signOut();
            },
            style: OutlinedButton.styleFrom(foregroundColor: AppColors.redText),
            icon: const Icon(Icons.logout),
            label: const Text(S.signOut),
          ),
          const SizedBox(height: 24),
          Text('Server: ${AppConfig.apiBaseUrl}',
              textAlign: TextAlign.center, style: const TextStyle(fontSize: 12, color: AppColors.muted)),
        ],
      ),
    );
  }
}
