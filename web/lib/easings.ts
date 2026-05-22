/**
 * Shared easing curves. Single source of truth so animation tuning
 * doesn't require touching every section file.
 *
 * EASE_OUT_EXPO — the in/out curve used by every section's entrance
 * animation. Long-tail decelerator, lands softly. Don't change without
 * a coordinated visual review.
 */
export const EASE_OUT_EXPO: [number, number, number, number] = [
  0.19, 1, 0.22, 1,
];
