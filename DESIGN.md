---
version: alpha
name: Pinto
description: A bright, paper-flat Thai-first social feed app held together by a single confident green, warm-ink typography, and barely-there elevation.

colors:
  primary: "#2ECC71"
  primary-tint: "#EAFBF1"
  primary-progress-track: "#BFF2D4"
  on-primary: "#FFFFFF"

  secondary: "#E6F4EA"
  on-secondary: "#0F5132"

  background: "#FAFAFA"
  surface: "#FFFFFF"
  surface-muted: "#F9FAFB"
  scrim: "#737373"
  overlay: "#1A1A1A"

  on-surface: "#433D4D"
  on-surface-secondary: "#696373"
  on-surface-subtitle: "#85818B"
  on-surface-action: "#7A7582"
  on-surface-disabled: "#A8A4AD"
  on-surface-hint: "#D1CED6"
  on-surface-neutral: "#757575"
  content: "#111827"

  divider: "#DFDDE3"
  input-border: "#D1CED6"
  outline-border: "#D4D1D8"
  hairline: "#EBEBEB"

  success: "#56CA00"
  success-container: "#EAFBF1"
  warning: "#FFB400"
  warning-soft: "#FCD34D"
  warning-container: "#FFF8E6"
  info: "#16B1FF"
  info-container: "#E8F7FF"
  error: "#FF7074"
  error-strong: "#FF4C51"
  error-container: "#FFEDEE"
  purple: "#8C57FF"
  purple-container: "#F4EEFF"
  location: "#1E88E5"

  category-orange: "#FF955C"
  category-orange-container: "#FFF1E9"
  category-pink: "#F06292"
  category-pink-container: "#FDEFF4"
  category-coral: "#F2877C"
  category-coral-container: "#FDEEEB"
  category-teal: "#3FC2B5"
  category-teal-container: "#ECF9F8"
  category-peach: "#FFA85E"
  category-peach-container: "#FFF3E7"

  rank-gold: "#FAC361"
  rank-silver: "#ACB6C7"
  rank-bronze: "#C77438"

  avatar-fallback: "#BBDEFB"

typography:
  display-lg:
    fontFamily: IBM Plex Sans Thai
    fontSize: 3rem
    fontWeight: 700
    lineHeight: 1
    letterSpacing: -0.01em
  display-md:
    fontFamily: IBM Plex Sans Thai
    fontSize: 2.25rem
    fontWeight: 700
    lineHeight: 1.11
  display-sm:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.875rem
    fontWeight: 700
    lineHeight: 1.2
  headline-lg:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.5rem
    fontWeight: 600
    lineHeight: 1.33
  headline-md:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.25rem
    fontWeight: 700
    lineHeight: 1.4
  title-lg:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.125rem
    fontWeight: 600
    lineHeight: 1.56
  title-md:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1rem
    fontWeight: 600
    lineHeight: 1.5
  body-lg:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.125rem
    fontWeight: 500
    lineHeight: 1.56
  body-md:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1rem
    fontWeight: 400
    lineHeight: 1.5
  body-sm:
    fontFamily: IBM Plex Sans Thai
    fontSize: 0.875rem
    fontWeight: 400
    lineHeight: 1.43
  label-lg:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1.125rem
    fontWeight: 600
    lineHeight: 1.33
  label-md:
    fontFamily: IBM Plex Sans Thai
    fontSize: 1rem
    fontWeight: 600
    lineHeight: 1.5
  label-sm:
    fontFamily: IBM Plex Sans Thai
    fontSize: 0.875rem
    fontWeight: 600
    lineHeight: 1.43
  caption:
    fontFamily: IBM Plex Sans Thai
    fontSize: 0.75rem
    fontWeight: 500
    lineHeight: 1.33
  overline:
    fontFamily: IBM Plex Sans Thai
    fontSize: 0.625rem
    fontWeight: 600
    lineHeight: 1.33
    letterSpacing: 0.04em

rounded:
  none: 0px
  xs: 2px
  sm: 8px
  md: 10px
  lg: 12px
  xl: 16px
  "2xl": 24px
  pill: 100px
  capsule: 500px
  full: 9999px

spacing:
  "0": 0px
  "1": 4px
  "2": 8px
  "3": 12px
  "4": 16px
  "5": 20px
  "6": 24px
  "7": 28px
  "8": 32px
  "10": 40px
  "12": 48px

components:

  app-background:
    backgroundColor: "{colors.background}"

  muted-panel:
    backgroundColor: "{colors.surface-muted}"
    rounded: "{rounded.sm}"
    padding: 16px

  modal-scrim:
    backgroundColor: "{colors.scrim}"

  progress-track:
    backgroundColor: "{colors.primary-progress-track}"
    rounded: "{rounded.full}"
    height: 4px

  body-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface-secondary}"
    typography: "{typography.body-sm}"

  input-border-swatch:
    backgroundColor: "{colors.input-border}"
    height: 1px

  outline-border-swatch:
    backgroundColor: "{colors.outline-border}"
    height: 1px

  status-success-icon:
    backgroundColor: "{colors.success}"
    rounded: "{rounded.full}"
    size: 24px

  status-warning-icon:
    backgroundColor: "{colors.warning}"
    rounded: "{rounded.full}"
    size: 24px

  status-info-icon:
    backgroundColor: "{colors.info}"
    rounded: "{rounded.full}"
    size: 24px

  status-purple-icon:
    backgroundColor: "{colors.purple}"
    rounded: "{rounded.full}"
    size: 24px


  subtitle-text-swatch:
    backgroundColor: "{colors.on-surface-subtitle}"
    height: 1px

  action-text-swatch:
    backgroundColor: "{colors.on-surface-action}"
    height: 1px

  disabled-text-swatch:
    backgroundColor: "{colors.on-surface-disabled}"
    height: 1px

  hint-text-swatch:
    backgroundColor: "{colors.on-surface-hint}"
    height: 1px

  hairline-swatch:
    backgroundColor: "{colors.hairline}"
    height: 1px

  category-orange-swatch:
    backgroundColor: "{colors.category-orange}"
    rounded: "{rounded.full}"
    size: 16px

  category-pink-swatch:
    backgroundColor: "{colors.category-pink}"
    rounded: "{rounded.full}"
    size: 16px

  category-coral-swatch:
    backgroundColor: "{colors.category-coral}"
    rounded: "{rounded.full}"
    size: 16px

  category-teal-swatch:
    backgroundColor: "{colors.category-teal}"
    rounded: "{rounded.full}"
    size: 16px

  category-peach-swatch:
    backgroundColor: "{colors.category-peach}"
    rounded: "{rounded.full}"
    size: 16px

  app-bar:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.content}"
    typography: "{typography.headline-md}"
    height: 56px
    padding: 8px

  bottom-nav:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface-neutral}"
    typography: "{typography.caption}"
    height: 50px
    padding: 6px

  bottom-nav-active:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-sm}"

  fab-create:
    backgroundColor: "{colors.primary}"
    rounded: "{rounded.full}"
    size: 44px

  button-primary:
    backgroundColor: "{colors.primary}"
    typography: "{typography.label-md}"
    rounded: "{rounded.lg}"
    padding: 16px
    height: 44px

  button-primary-sm:
    backgroundColor: "{colors.primary}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.md}"
    padding: 12px
    height: 36px

  button-primary-lg:
    backgroundColor: "{colors.primary}"
    typography: "{typography.label-lg}"
    rounded: "{rounded.xl}"
    padding: 20px
    height: 52px

  button-primary-pressed:
    backgroundColor: "#27AE60"
    rounded: "{rounded.lg}"

  button-primary-disabled:
    backgroundColor: "{colors.primary-tint}"
    rounded: "{rounded.lg}"

  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.on-secondary}"
    typography: "{typography.label-md}"
    rounded: "{rounded.lg}"
    padding: 16px
    height: 44px

  button-outline:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-md}"
    rounded: "{rounded.lg}"
    padding: 16px
    height: 44px

  button-outline-pill:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-md}"
    rounded: "{rounded.pill}"
    padding: 24px
    height: 44px

  button-ghost:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-md}"
    rounded: "{rounded.lg}"
    padding: 16px

  button-destructive:
    backgroundColor: "{colors.error-strong}"
    typography: "{typography.label-md}"
    rounded: "{rounded.lg}"
    padding: 16px
    height: 44px

  input-text:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.content}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 12px

  input-text-focused:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.content}"
    rounded: "{rounded.sm}"

  input-text-error:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sm}"

  input-label:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.label-sm}"

  input-placeholder:
    backgroundColor: "{colors.surface}"
    typography: "{typography.body-sm}"

  card-default:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    padding: 16px

  card-stat-primary:
    backgroundColor: "{colors.primary-tint}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  card-stat-success:
    backgroundColor: "{colors.success-container}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  card-stat-warning:
    backgroundColor: "{colors.warning-container}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  card-stat-info:
    backgroundColor: "{colors.info-container}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  card-stat-error:
    backgroundColor: "{colors.error-container}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  card-stat-purple:
    backgroundColor: "{colors.purple-container}"
    textColor: "{colors.on-surface}"
    typography: "{typography.headline-lg}"
    rounded: "{rounded.lg}"
    padding: 16px

  chip-filter:
    backgroundColor: "{colors.divider}"
    textColor: "{colors.on-surface}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  chip-category-orange:
    backgroundColor: "{colors.category-orange-container}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  chip-category-pink:
    backgroundColor: "{colors.category-pink-container}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  chip-category-teal:
    backgroundColor: "{colors.category-teal-container}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  chip-category-coral:
    backgroundColor: "{colors.category-coral-container}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  chip-category-peach:
    backgroundColor: "{colors.category-peach-container}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 12px

  badge-unread:
    backgroundColor: "{colors.error-strong}"
    rounded: "{rounded.full}"
    size: 8px

  avatar:
    backgroundColor: "{colors.avatar-fallback}"
    rounded: "{rounded.full}"
    size: 40px

  dialog-confirmation:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.title-lg}"
    rounded: "{rounded.xl}"
    padding: 24px

  dialog-title:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.title-lg}"

  dialog-subtitle:
    backgroundColor: "{colors.surface}"
    typography: "{typography.body-lg}"

  dialog-action-cancel:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-lg}"
    padding: 12px

  dialog-action-destructive:
    backgroundColor: "{colors.surface}"
    typography: "{typography.label-lg}"
    padding: 12px

  snackbar-success:
    backgroundColor: "{colors.primary}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.sm}"
    padding: 16px

  snackbar-info:
    backgroundColor: "{colors.primary}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.sm}"
    padding: 16px

  snackbar-warning:
    backgroundColor: "{colors.warning-soft}"
    textColor: "{colors.content}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.sm}"
    padding: 16px

  snackbar-error:
    backgroundColor: "{colors.error}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.sm}"
    padding: 16px

  toast-pill:
    backgroundColor: "{colors.overlay}"
    textColor: "{colors.on-primary}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.capsule}"
    padding: 16px

  progress-chip:
    backgroundColor: "{colors.overlay}"
    textColor: "{colors.on-primary}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.2xl}"
    padding: 20px

  skeleton-text:
    backgroundColor: "#E5E7EB"
    rounded: "{rounded.xs}"
    height: 16px

  skeleton-card:
    backgroundColor: "#E5E7EB"
    rounded: "{rounded.sm}"
    height: 120px

  skeleton-avatar:
    backgroundColor: "#E5E7EB"
    rounded: "{rounded.full}"
    size: 40px

  location-pin:
    backgroundColor: "{colors.location}"
    rounded: "{rounded.full}"
    size: 24px

  rank-badge-gold:
    backgroundColor: "{colors.rank-gold}"
    textColor: "{colors.content}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 8px

  rank-badge-silver:
    backgroundColor: "{colors.rank-silver}"
    textColor: "{colors.content}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 8px

  rank-badge-bronze:
    backgroundColor: "{colors.rank-bronze}"
    typography: "{typography.label-sm}"
    rounded: "{rounded.pill}"
    padding: 8px
---

## Overview

Pinto is a Thai-first social feed app. The product should feel **bright, paper-flat, and trustworthy** — like a clean notebook with a single confident accent color, never glossy or skeuomorphic. The interface optimizes for thumb-reachable mobile layouts, fast scanning of feed cards, and predictable navigation between Home, Video, Create, Chat, and Profile.

The visual identity rests on three rules: one saturated brand green carries every "do something" or "this is selected" signal; all body copy uses warm tinted ink rather than pure black so long reading sessions feel softer; surfaces stay flat and rely on whitespace, hairlines, and rounded corners rather than shadows. Motion is short, soft, and never load-bearing.

Inspiration:

- Modern social-feed apps (Instagram, Threads) trimmed of visual noise.
- Tailwind CSS palette discipline — flat, saturated, mid-weight greens and grays.
- Material 3 surface language softened with rounded, paper-flat cards.

## Colors

Pinto's palette is organized into three layers.

**Brand.** A single Tailwind-style green `#2ECC71` is the only color associated with primary actions, active states, brand wordmarks, success toasts, and the floating create-post button. Its 8 % tint backs stat cards and brand washes; a 24 % tint forms the filled section of progress bars. `on-primary` is pure white in product UI; components that are icon-only or brand-fill avoid using token pairs that imply body-text contrast.

**Ink on paper.** Text is never pure black. All copy uses a warm-violet ink `#2E263D` at six opacity steps — six blended token steps for headings (`on-surface`), secondary body (`on-surface-secondary`), captions and timestamps (`on-surface-subtitle`), tertiary actions (`on-surface-action`), disabled labels (`on-surface-disabled`), and placeholders (`on-surface-hint`). A neutral `#757575` covers inactive nav glyphs, and `#111827` (`content`) is reserved for the densest data and destructive labels.

**Status & categorical.** Saturated mid-bright accents drive stat cards, badges, chips, and category pills. Each ships with an 8–16 % tinted container (`*-container`) so it can sit on white without clashing — success green, warning amber, info sky, error coral, purple highlight, location blue, plus five category accents (orange, pink, coral, teal, peach) and a three-tier rank palette (gold, silver, bronze).

Avoid stacking more than two brand or status colors per screen. Category pills should treat their container as the dominant surface and their saturated color only as icon, ring, and label.

## Typography

The single family is **IBM Plex Sans Thai**, loaded via Google Fonts so the same font covers Thai and Latin without rhythm or weight drift across locales. Headlines, body, and labels all share the family — hierarchy is driven by **weight and color**, not by font swaps.

The scale spans `display-lg` (3 rem) down to `overline` (0.625 rem), with `body-md` (1 rem / 400) as the workhorse. Labels carry semibold (600), body copy carries regular (400) or medium (500), and display sizes carry bold (700). Active and selected text always shifts from regular ink to **semibold brand green** — this rule is consistent across nav tabs, route buttons, and toggle states.

Use `display-*` only for empty states, reward dialogs, and onboarding. Use `headline-*` for app-bar wordmarks and page titles. `title-lg` is the default dialog title. `body-lg` is reserved for dialog subtitles where a slight upweight relative to body copy adds gravity without raising the visual temperature.

## Layout

Pinto is a one-column mobile app. The default scaffold background is `surface` (`#FFFFFF`) for primary screens; lists and dashboards may sit on `background` (`#FAFAFA`) so white cards float against a softer canvas.

Spacing is a 4 px grid (`spacing.1` = 4 px through `spacing.12` = 48 px). The dominant insets are:

- **Screen edge padding:** 16 px.
- **Card padding:** 16 px.
- **Dialog padding:** 24 px horizontal, 20 px vertical.
- **Input padding:** 12 px.
- **App bar padding:** 8 px horizontal, 4 px vertical.
- **Bottom nav padding:** 6 px horizontal.

Vertical rhythm uses `spacing.2` (8 px) between a field label and its input, `spacing.1` (4 px) between an input and its error message, `spacing.2` (8 px) between a dialog title and subtitle, and `spacing.7` (28 px) between dialog body and actions.

Content max width is 600 px on tablets so feed cards do not over-stretch. Screen padding never collapses below 16 px even on small phones. The bottom navigation respects the device safe area inset.

## Elevation & Depth

Elevation is intentionally restrained. Five conceptual levels exist:

- **Level 0 — flat.** App bar, bottom nav, list rows, secondary buttons, all dialogs' inner content. No shadow; separation comes from a 1 px divider or a 0.5 px hairline border.
- **Level 1 — `0 1px 2px rgba(0,0,0,0.10)`.** Lightweight content cards and skeleton placeholders.
- **Level 2 — `0 2px 4px rgba(0,0,0,0.05)`.** Sticky dialog and bottom-sheet headers floating over scrollable content.
- **Level 3 — `0 4px 8px rgba(0,0,0,0.08)`.** Primary `button-primary` lift — soft enough to register on tap, never strong enough to read as a card.
- **Level 4 — `0 4px 10px rgba(0,0,0,0.30)`.** Floating dark overlays — `toast-pill`, `progress-chip` — which need to read against any underlying content.

A higher decorative shadow `0 8px 24px #2E263D38` (`brand-glow-lg`) is available for marketing-style hero cards but should not appear in transactional UI. Never stack shadows on stacked surfaces; the app bar and bottom nav rely on the `divider` and `hairline` colors for separation, not elevation.

## Shapes

Pinto's rounding scale is opinionated. Cards and most surfaces use **8 px** (`rounded.sm`). Small buttons use 10 px, medium buttons 12 px, large buttons and confirmation dialogs 16 px. Progress chips bump up to 24 px (`rounded.2xl`). Outlined primary CTAs and filter chips are fully rounded (`rounded.pill` = 100 px). Centered toast pills use the larger `rounded.capsule` (500 px) so they read as fully circular regardless of length. Avatars, the floating create-post button, and the unread badge use `rounded.full`.

Border weights are: `0.5 px` hairline (bottom-nav top edge), `1 px` default (cards, app bar bottom), `1.2 px` outlined buttons, `1.5 px` outlined pill, `2 px` focused and error fields.

Use sharp corners (`rounded.none`) only inside data charts and dividers — never on tappable surfaces. The center create-post FAB is a perfect circle regardless of size.

## Components

**Buttons.** The primary green button carries the most brand expression. It comes in three sizes (sm 36 px, md 44 px, lg 52 px) with matching radii (10 / 12 / 16). All variants animate container properties over 150 ms on press and add an 8 % brand wash overlay on hover or pressed state. Outline buttons use a brand-green 1.2–1.5 px stroke and a transparent fill — pill variant is reserved for tertiary "view more" actions. Ghost buttons use brand green text only and rely on the press overlay for affordance. Destructive buttons appear filled red (`error-strong`); destructive text in dialogs uses the same red but stays as text-only.

**Inputs.** Inputs are quiet rounded rectangles with a 22 %-ink border (`input-border`) that doubles in width and shifts to brand green or `error-strong` on focus. Field labels above the input are semibold and small (`label-sm`); they recolor to `error` when validation fails. Tapping a field opens a full-screen white sheet that slides up from the bottom (300 ms easeOutCubic) rather than inline editing — this keeps the parent surface calm and gives Thai input methods room to breathe. Character counters live at the bottom-right of the sheet and turn `error` when over the limit.

**Cards.** Default content cards are white with 8 px rounding, a `card-soft` shadow, and 16 px padding. Stat cards swap their background for a status `*-container` tint so a row of metrics reads as a calm palette rather than a parade of saturated colors. Inside each stat card, the icon and value adopt the saturated `*` color while the label stays in `on-surface-secondary`.

**Chips.** Filter chips use the 8 % ink wash as background with primary ink as label, rounded into a pill. Category chips reuse the same pill geometry but adopt the categorical `*-container` background and saturated `*` color as label — these are the only places where a single chip displays its full color treatment.

**Dialogs.** Confirmation dialogs are white rectangles with 16 px rounding, 24 px horizontal × 20 px vertical padding, and right-aligned tertiary text buttons separated by 16 px. There are no filled CTAs inside a dialog body — the action language carries the weight, and the destructive action uses `error-strong` as its only color hint.

**Snackbars.** Snackbars float, are radius 8, and live for 2.5 seconds. Success and info both adopt brand green; warning takes the soft yellow `warning-soft` (`#FCD34D`) with `content` ink for legibility; error uses `error` (`#FF7074`). Every variant pairs a filled Remix icon (20 px) with white semibold text and 12 px gap.

**Toasts & progress chips.** A centered dark capsule (`overlay`) holds short status messages — preferred over snackbars for transient confirmations like "saved" or "linked." The progress chip variant adds an inline 16 px indeterminate spinner with 2 px stroke and a longer Thai-language label.

**Skeletons.** Skeleton placeholders use a low-cost Shimmer in `#E5E7EB → #F3F4F6` and replace any spinner for content-loading states. Skeleton text uses 2 px rounding; skeleton cards reuse 8 px; skeleton avatars use full rounding.

**App bar.** Flat white with a 1 px `divider` bottom border, no elevation. The wordmark is a split-color RichText where the brand half is `primary` semibold/bold and the second half is `content` bold. Trailing actions are text buttons that switch from regular `content` to semibold `primary` when their route is active.

**Bottom nav.** 50 px tall, white, with a 0.5 px `hairline` top border. Five slots: Home, Video, Create, Chat, Profile. Inactive icons are Remix line glyphs in `on-surface-neutral`; active icons swap to Remix fill in `primary` via an `AnimatedSwitcher` over 200 ms. The center "Create" slot is a 44 px green circle (`fab-create`) inside a 70 px wide expanded gap. Unread indicators are 8 px circles in `error-strong`.

**Avatars & badges.** Avatars are always circular. Fallbacks are a light-blue (`#BBDEFB`) circle with a white filled user glyph at 70 % of the diameter. The unread badge is the only place outside a destructive flow where `error-strong` appears as a fill.

**Iconography.** Icons are exclusively from **Remix Icons**. The selection rule is mechanical: `line` for resting state, `fill` for active, selected, or emphatic. Default size is 24 px, dense rows step down to 16–18 px. Icons inherit their parent text color except for semantic overrides — location pins use `location`, error glyphs use `error`, warning glyphs use `warning`, success glyphs use `primary`.

**Motion.** The signature page transition slides the incoming page in from the right over 480 ms using `cubic-bezier(0.28, 1.0, 0.0, 1.0)` and exits over 200 ms. The covered page shifts 35 % laterally to imply depth. Cross-fades and selection swaps (icon state, button overlay) complete in 150–200 ms. Toasts and chips fade-and-scale rather than slide. Reduced-motion users fall back to no transition.

## Do's and Don'ts

**Do**

- Lead every screen with a single brand action — the green `button-primary` or the floating create circle — and let everything else stay neutral.
- Pair every status color with its matching `*-container` background when it lives on a white surface.
- Use weight before size to express hierarchy: a semibold `title-md` in `on-surface` outranks a regular `title-lg` in `on-surface-secondary`.
- Open full-screen sheets for text entry rather than inline editing, so Thai input methods get full vertical space.
- Replace spinners with Shimmer skeletons whenever a layout's shape is known in advance.
- Respect the bottom safe-area inset on every full-screen scroll surface.

**Don't**

- Don't introduce gradients on chrome, glassmorphism, or translucent surfaces over photographic backgrounds.
- Don't use pure `#000` text or full-saturation cyan/magenta accents — they break the warm-ink palette.
- Don't promote `error-strong` to a primary background outside the destructive button — it competes with the brand green.
- Don't stack shadows; if two surfaces feel like they need separation, add whitespace or a `divider` line.
- Don't ship a Material-3 extended FAB; Pinto's primary action is always a circular green plus, never a labeled FAB.
- Don't add decorative dividers between every row — whitespace and weight contrast carry separation.
- Don't rely on motion to convey meaning; every animation has a no-motion fallback.
