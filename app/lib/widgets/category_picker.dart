import 'package:flutter/material.dart';

import '../api/models.dart';
import 'common.dart';

/// Horizontal pills for choosing one category.
class CategoryPicker extends StatelessWidget {
  final List<Category> categories;
  final String? selectedId;
  final ValueChanged<String?> onSelect;
  const CategoryPicker({super.key, required this.categories, required this.selectedId, required this.onSelect});

  @override
  Widget build(BuildContext context) => Wrap(
        spacing: 8,
        runSpacing: 8,
        children: [
          for (final c in categories)
            Pill(
              label: c.name,
              selected: c.id == selectedId,
              trailing: c.id == selectedId ? null : CategoryBadge(slug: c.slug, color: c.color, size: 20),
              onTap: () => onSelect(c.id == selectedId ? null : c.id),
            ),
        ],
      );
}
