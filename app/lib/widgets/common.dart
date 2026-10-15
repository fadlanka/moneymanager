import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../theme/app_theme.dart';
import '../theme/category_style.dart';
import '../theme/tokens.dart';

/// White rounded card with a hairline border (`.card`).
class AppCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color color;
  const AppCard({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.color = AppColors.card,
  });

  @override
  Widget build(BuildContext context) => Container(
        width: double.infinity,
        padding: padding,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(AppRadius.card),
          border: Border.all(color: AppColors.hair),
        ),
        child: child,
      );
}

/// Section title + optional trailing link (`.sechead`).
class SectionHeader extends StatelessWidget {
  final String title;
  final String? action;
  final VoidCallback? onAction;
  const SectionHeader(this.title, {super.key, this.action, this.onAction});

  @override
  Widget build(BuildContext context) => Row(
        children: [
          Expanded(child: Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700))),
          if (action != null)
            TextButton(
              onPressed: onAction,
              style: TextButton.styleFrom(foregroundColor: AppColors.blue, padding: EdgeInsets.zero),
              child: Text(action!, style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14)),
            ),
        ],
      );
}

enum TagTone { blue, yellow, green, red, grey }

class Tag extends StatelessWidget {
  final String text;
  final TagTone tone;
  const Tag(this.text, {super.key, this.tone = TagTone.grey});

  @override
  Widget build(BuildContext context) {
    final (bg, fg) = switch (tone) {
      TagTone.blue => (AppColors.blueSoft, AppColors.blue),
      TagTone.yellow => (AppColors.yellowSoft, AppColors.yellowText),
      TagTone.green => (AppColors.greenSoft, AppColors.greenText),
      TagTone.red => (AppColors.redSoft, AppColors.redText),
      TagTone.grey => (AppColors.track, AppColors.muted),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(AppRadius.pill)),
      child: Text(text, style: TextStyle(color: fg, fontSize: 12, fontWeight: FontWeight.w700)),
    );
  }
}

/// Rounded square with the category icon (`.cat`).
class CategoryBadge extends StatelessWidget {
  final String? slug;
  final String? color;
  final double size;
  const CategoryBadge({super.key, required this.slug, required this.color, this.size = 44});

  @override
  Widget build(BuildContext context) {
    final s = categoryStyle(slug, color);
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(color: s.bg, borderRadius: BorderRadius.circular(size * 0.32)),
      child: Icon(s.icon, color: s.fg, size: size * 0.45),
    );
  }
}

class ProgressBar extends StatelessWidget {
  final double value;
  final Color color;
  final double height;
  final String? semanticsLabel;
  const ProgressBar({
    super.key,
    required this.value,
    this.color = AppColors.blue,
    this.height = 10,
    this.semanticsLabel,
  });

  @override
  Widget build(BuildContext context) => Semantics(
        label: semanticsLabel,
        value: '${(value * 100).round()}%',
        child: ClipRRect(
          borderRadius: BorderRadius.circular(AppRadius.pill),
          child: SizedBox(
            height: height,
            child: Stack(children: [
              Container(color: AppColors.track),
              FractionallySizedBox(
                widthFactor: value.clamp(0.0, 1.0),
                child: Container(
                  decoration: BoxDecoration(color: color, borderRadius: BorderRadius.circular(AppRadius.pill)),
                ),
              ),
            ]),
          ),
        ),
      );
}

/// Bar color for a budget state, as in the design: blue ok, yellow near, red over.
Color budgetColor(String state) => switch (state) {
      'over' => AppColors.red,
      'near' => AppColors.yellow,
      _ => AppColors.blue,
    };

class Pill extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback? onTap;
  final Widget? trailing;
  const Pill({super.key, required this.label, this.selected = false, this.onTap, this.trailing});

  @override
  Widget build(BuildContext context) => Semantics(
        button: true,
        selected: selected,
        child: Material(
          color: selected ? AppColors.ink : AppColors.card,
          shape: StadiumBorder(side: BorderSide(color: selected ? AppColors.ink : AppColors.hair)),
          child: InkWell(
            customBorder: const StadiumBorder(),
            onTap: onTap,
            child: Container(
              constraints: const BoxConstraints(minHeight: 40),
              padding: const EdgeInsets.symmetric(horizontal: 16),
              alignment: Alignment.center,
              child: Row(mainAxisSize: MainAxisSize.min, children: [
                Text(label,
                    style: TextStyle(
                      color: selected ? Colors.white : AppColors.ink,
                      fontWeight: FontWeight.w600,
                      fontSize: 14,
                    )),
                if (trailing != null) ...[const SizedBox(width: 6), trailing!],
              ]),
            ),
          ),
        ),
      );
}

/// Segmented control (`.seg`).
class Segmented<T> extends StatelessWidget {
  final List<(T, String)> options;
  final T value;
  final ValueChanged<T> onChanged;
  const Segmented({super.key, required this.options, required this.value, required this.onChanged});

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.all(4),
        decoration: BoxDecoration(color: AppColors.segBg, borderRadius: BorderRadius.circular(14)),
        child: Row(
          children: [
            for (final (v, label) in options)
              Expanded(
                child: Semantics(
                  button: true,
                  selected: v == value,
                  child: GestureDetector(
                    onTap: () => onChanged(v),
                    child: Container(
                      height: 40,
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: v == value ? AppColors.card : Colors.transparent,
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Text(label,
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: v == value ? FontWeight.w700 : FontWeight.w600,
                            color: v == value ? AppColors.ink : AppColors.muted,
                          )),
                    ),
                  ),
                ),
              ),
          ],
        ),
      );
}

/// Square icon button with hairline border (`.iconbtn`).
class IconSquareButton extends StatelessWidget {
  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  const IconSquareButton({super.key, required this.icon, required this.tooltip, this.onPressed});

  @override
  Widget build(BuildContext context) => Tooltip(
        message: tooltip,
        child: Material(
          color: AppColors.card,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.iconButton),
            side: const BorderSide(color: AppColors.hair),
          ),
          child: InkWell(
            borderRadius: BorderRadius.circular(AppRadius.iconButton),
            onTap: onPressed,
            child: SizedBox(width: 44, height: 44, child: Icon(icon, size: 22, color: AppColors.ink)),
          ),
        ),
      );
}

class Amount extends StatelessWidget {
  final String text;
  final double size;
  final FontWeight weight;
  final Color color;
  const Amount(this.text,
      {super.key, this.size = 16, this.weight = FontWeight.w700, this.color = AppColors.ink});

  @override
  Widget build(BuildContext context) => Text(
        text,
        maxLines: 1,
        style: TextStyle(fontSize: size, fontWeight: weight, color: color, fontFeatures: tabular),
      );
}

class ErrorView extends StatelessWidget {
  final Object error;
  final VoidCallback onRetry;
  const ErrorView({super.key, required this.error, required this.onRetry});

  @override
  Widget build(BuildContext context) => Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            const Icon(Icons.cloud_off_outlined, size: 40, color: AppColors.muted),
            const SizedBox(height: 12),
            Text(error.toString(), textAlign: TextAlign.center, style: const TextStyle(color: AppColors.muted)),
            const SizedBox(height: 16),
            OutlinedButton(onPressed: onRetry, child: const Text(S.retry)),
          ]),
        ),
      );
}

class Loading extends StatelessWidget {
  const Loading({super.key});

  @override
  Widget build(BuildContext context) =>
      const Center(child: Padding(padding: EdgeInsets.all(32), child: CircularProgressIndicator()));
}

/// Yellow notice row used for warnings and "needs a look" hints.
class Notice extends StatelessWidget {
  final String text;
  final IconData icon;
  final Color bg;
  final Color iconBg;
  final Widget? trailing;
  final VoidCallback? onTap;
  const Notice({
    super.key,
    required this.text,
    this.icon = Icons.priority_high_rounded,
    this.bg = AppColors.yellowSoft,
    this.iconBg = AppColors.yellow,
    this.trailing,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) => Material(
        color: bg,
        borderRadius: BorderRadius.circular(18),
        child: InkWell(
          borderRadius: BorderRadius.circular(18),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
            child: Row(children: [
              Container(
                width: 34,
                height: 34,
                decoration: BoxDecoration(color: iconBg, shape: BoxShape.circle),
                child: Icon(icon, size: 18, color: AppColors.ink),
              ),
              const SizedBox(width: 12),
              Expanded(child: Text(text, style: const TextStyle(fontSize: 14, height: 1.35))),
              if (trailing != null) ...[const SizedBox(width: 8), trailing!],
            ]),
          ),
        ),
      );
}

void showSnack(BuildContext context, String message) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text(message)));
}
