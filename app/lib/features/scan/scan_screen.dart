import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';

import '../../l10n/strings.dart';
import '../../theme/tokens.dart';
import '../../widgets/common.dart';
import '../transactions/transaction_form_screen.dart';
import 'processing_screen.dart';

/// Step 1: get a photo. Uses the system camera via image_picker, which also
/// works in the web build (Safari opens the camera from a file input).
class ScanScreen extends StatelessWidget {
  const ScanScreen({super.key});

  Future<void> _pick(BuildContext context, ImageSource source) async {
    final XFile? file;
    try {
      // Downscale before upload: smaller, faster, and still sharp enough to read.
      file = await ImagePicker().pickImage(source: source, maxWidth: 2000, maxHeight: 2000, imageQuality: 85);
    } catch (e) {
      if (context.mounted) showSnack(context, 'Couldn\'t open the ${source == ImageSource.camera ? 'camera' : 'gallery'}.');
      return;
    }
    if (file == null || !context.mounted) return;
    final name = file.name;
    final bytes = await file.readAsBytes();
    if (!context.mounted) return;
    await Navigator.pushReplacement(
      context,
      MaterialPageRoute(builder: (_) => ProcessingScreen(bytes: bytes, filename: name)),
    );
  }

  @override
  Widget build(BuildContext context) {
    const corner = BorderSide(color: Colors.white, width: 4);
    Widget mark({required Alignment at, required Border border, required BorderRadius radius}) => Align(
          alignment: at,
          child: Padding(
            padding: const EdgeInsets.all(22),
            child: Container(width: 44, height: 44, decoration: BoxDecoration(border: border, borderRadius: radius)),
          ),
        );

    return Scaffold(
      backgroundColor: AppColors.cameraBg,
      body: SafeArea(
        child: Column(children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
            child: Row(children: [
              _RoundButton(icon: Icons.close, label: 'Close scanner', onTap: () => Navigator.pop(context)),
              const Expanded(
                child: Column(children: [
                  Text(S.scanReceipt, style: TextStyle(color: Colors.white, fontSize: 17, fontWeight: FontWeight.w700)),
                  Text(S.step1, style: TextStyle(color: AppColors.mutedLight, fontSize: 12)),
                ]),
              ),
              const SizedBox(width: 44),
            ]),
          ),
          Expanded(
            child: Container(
              margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              decoration: BoxDecoration(color: AppColors.cameraFrame, borderRadius: BorderRadius.circular(28)),
              child: Stack(children: [
                Center(
                  child: Transform.rotate(
                    angle: -0.05,
                    child: Container(
                      width: 180,
                      height: 270,
                      decoration: BoxDecoration(color: const Color(0xFFF4F4EF), borderRadius: BorderRadius.circular(10)),
                      child: const Icon(Icons.receipt_long, size: 56, color: AppColors.mutedLight),
                    ),
                  ),
                ),
                mark(at: Alignment.topLeft, border: const Border(left: corner, top: corner), radius: const BorderRadius.only(topLeft: Radius.circular(18))),
                mark(at: Alignment.topRight, border: const Border(right: corner, top: corner), radius: const BorderRadius.only(topRight: Radius.circular(18))),
                mark(at: Alignment.bottomLeft, border: const Border(left: corner, bottom: corner), radius: const BorderRadius.only(bottomLeft: Radius.circular(18))),
                mark(at: Alignment.bottomRight, border: const Border(right: corner, bottom: corner), radius: const BorderRadius.only(bottomRight: Radius.circular(18))),
                Align(
                  alignment: Alignment.bottomCenter,
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: 30),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                      decoration: BoxDecoration(color: AppColors.yellow, borderRadius: BorderRadius.circular(999)),
                      child: const Text(S.fitReceipt, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w700)),
                    ),
                  ),
                ),
              ]),
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 24, 24, 28),
            child: Row(children: [
              Expanded(
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Tooltip(
                    message: S.gallery,
                    child: Material(
                      color: const Color(0xFF2A2B33),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(14),
                        side: const BorderSide(color: Colors.white, width: 2),
                      ),
                      child: InkWell(
                        borderRadius: BorderRadius.circular(14),
                        onTap: () => _pick(context, ImageSource.gallery),
                        child: const SizedBox(width: 52, height: 52, child: Icon(Icons.photo_library_outlined, color: Colors.white)),
                      ),
                    ),
                  ),
                ),
              ),
              Semantics(
                button: true,
                label: S.takePhoto,
                child: GestureDetector(
                  onTap: () => _pick(context, ImageSource.camera),
                  child: Container(
                    width: 78,
                    height: 78,
                    decoration: BoxDecoration(
                      color: Colors.white,
                      shape: BoxShape.circle,
                      border: Border.all(color: AppColors.blue, width: 5),
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Align(
                  alignment: Alignment.centerRight,
                  child: TextButton(
                    style: TextButton.styleFrom(foregroundColor: Colors.white, minimumSize: const Size(44, 44)),
                    onPressed: () => Navigator.pushReplacement(
                      context,
                      MaterialPageRoute(builder: (_) => const TransactionFormScreen()),
                    ),
                    child: const Text(S.typeIt, style: TextStyle(fontWeight: FontWeight.w700, fontSize: 15)),
                  ),
                ),
              ),
            ]),
          ),
        ]),
      ),
    );
  }
}

class _RoundButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  const _RoundButton({required this.icon, required this.label, required this.onTap});

  @override
  Widget build(BuildContext context) => Tooltip(
        message: label,
        child: Material(
          color: const Color(0x1FFFFFFF),
          shape: const CircleBorder(),
          child: InkWell(
            customBorder: const CircleBorder(),
            onTap: onTap,
            child: SizedBox(width: 44, height: 44, child: Icon(icon, color: Colors.white)),
          ),
        ),
      );
}
