import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../transactions/transaction_form_screen.dart';
import 'review_screen.dart';

/// Uploads the photo, then polls the receipt until the AI is done.
/// Extraction is asynchronous on the server: a local model may take a minute.
class ProcessingScreen extends ConsumerStatefulWidget {
  final Uint8List bytes;
  final String filename;
  const ProcessingScreen({super.key, required this.bytes, required this.filename});

  @override
  ConsumerState<ProcessingScreen> createState() => _ProcessingScreenState();
}

enum _Phase { uploading, reading, failed }

class _ProcessingScreenState extends ConsumerState<ProcessingScreen> {
  _Phase _phase = _Phase.uploading;
  String? _receiptId;
  String? _error;
  Timer? _poll;
  final _clock = Stopwatch();

  @override
  void initState() {
    super.initState();
    _upload();
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _upload() async {
    setState(() {
      _phase = _Phase.uploading;
      _error = null;
    });
    try {
      final id = await ref.read(apiProvider).uploadReceipt(widget.bytes, widget.filename);
      _receiptId = id;
      _startPolling();
    } on ApiException catch (e) {
      _fail(e.message);
    }
  }

  void _startPolling() {
    _clock
      ..reset()
      ..start();
    setState(() => _phase = _Phase.reading);
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(seconds: 2), (_) => _check());
  }

  Future<void> _check() async {
    final id = _receiptId;
    if (id == null) return;
    final Receipt r;
    try {
      r = await ref.read(apiProvider).receipt(id);
    } on ApiException {
      return; // Transient network blip: keep polling.
    }
    if (!mounted) return;
    if (r.isDone) {
      _poll?.cancel();
      await Navigator.pushReplacement(
        context,
        MaterialPageRoute(builder: (_) => ReviewScreen(receipt: r, imageBytes: widget.bytes)),
      );
    } else if (r.isFailed) {
      _fail(r.error ?? S.readFailed);
    } else {
      setState(() {}); // Refresh the elapsed-time hint.
    }
  }

  void _fail(String message) {
    _poll?.cancel();
    _clock.stop();
    if (mounted) {
      setState(() {
        _phase = _Phase.failed;
        _error = message;
      });
    }
  }

  Future<void> _retry() async {
    final id = _receiptId;
    if (id == null) return _upload();
    try {
      await ref.read(apiProvider).retryReceipt(id);
      _startPolling();
    } on ApiException catch (e) {
      _fail(e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final failed = _phase == _Phase.failed;
    final slow = _clock.elapsed.inSeconds >= 15;
    return Scaffold(
      appBar: AppBar(title: const Text(S.scanReceipt, style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700))),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(children: [
            Expanded(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(20),
                child: Stack(fit: StackFit.expand, children: [
                  Image.memory(widget.bytes, fit: BoxFit.cover, errorBuilder: (context, error, stack) => Container(color: AppColors.track)),
                  Container(color: const Color(0x99FFFFFF)),
                  if (!failed) const Center(child: CircularProgressIndicator(strokeWidth: 3)),
                ]),
              ),
            ),
            const SizedBox(height: 24),
            if (!failed) ...[
              Text(
                _phase == _Phase.uploading ? S.uploading : S.reading,
                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 8),
              Text(
                slow ? S.readingSlow : 'AI is reading the store, items and total.',
                textAlign: TextAlign.center,
                style: const TextStyle(color: AppColors.muted),
              ),
            ] else ...[
              const Text(S.readFailed, style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
              const SizedBox(height: 8),
              Text(_error ?? '', textAlign: TextAlign.center, style: const TextStyle(color: AppColors.muted)),
              const SizedBox(height: 20),
              FilledButton(onPressed: _retry, child: const Text(S.tryAgain)),
              const SizedBox(height: 10),
              OutlinedButton(
                onPressed: () => Navigator.pushReplacement(
                  context,
                  MaterialPageRoute(builder: (_) => const TransactionFormScreen()),
                ),
                child: const Text(S.typeInstead),
              ),
            ],
          ]),
        ),
      ),
    );
  }
}
