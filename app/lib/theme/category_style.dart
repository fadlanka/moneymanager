import 'package:flutter/material.dart';

import 'tokens.dart';

class CategoryStyle {
  final Color bg;
  final Color fg;
  final IconData icon;
  const CategoryStyle(this.bg, this.fg, this.icon);
}

/// Color tokens stored on categories by the backend.
(Color, Color) colorPair(String token) {
  switch (token) {
    case 'yellow':
      return (AppColors.yellow, AppColors.ink);
    case 'red':
      return (AppColors.red, AppColors.ink);
    case 'green':
      return (AppColors.green, AppColors.ink);
    case 'blue':
      return (AppColors.blue, Colors.white);
    case 'ink':
      return (AppColors.ink, Colors.white);
    case 'yellow-soft':
      return (AppColors.yellowSoft, AppColors.yellowText);
    case 'red-soft':
      return (AppColors.redSoft, AppColors.redText);
    case 'green-soft':
      return (AppColors.greenSoft, AppColors.greenText);
    case 'blue-soft':
      return (AppColors.blueSoft, AppColors.blue);
    default:
      return (AppColors.track, AppColors.ink);
  }
}

IconData categoryIcon(String slug) {
  switch (slug) {
    case 'food':
      return Icons.local_cafe_outlined;
    case 'groceries':
      return Icons.local_grocery_store_outlined;
    case 'shopping':
      return Icons.shopping_bag_outlined;
    case 'transport':
      return Icons.train_outlined;
    case 'housing':
      return Icons.home_outlined;
    case 'bills':
      return Icons.bolt_outlined;
    case 'subscriptions':
      return Icons.play_circle_outline;
    case 'health':
      return Icons.favorite_outline;
    case 'entertainment':
      return Icons.movie_outlined;
    case 'debt':
      return Icons.credit_card_outlined;
    case 'income':
      return Icons.south_rounded;
    default:
      return Icons.more_horiz;
  }
}

CategoryStyle categoryStyle(String? slug, String? color) {
  final (bg, fg) = colorPair(color ?? 'grey');
  return CategoryStyle(bg, fg, categoryIcon(slug ?? 'other'));
}
