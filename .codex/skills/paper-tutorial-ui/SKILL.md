---
name: paper-tutorial-ui
description: "Frontend visual design system for a warm paper tutorial aesthetic inspired by educational explainer screenshots: off-white notebook backgrounds, subtle grid or paper grain, thick ink-like outlines, rounded cards, offset navy shadows, flat playful icons, serif editorial headings, and sparse instructional layouts. Use when designing or implementing Stream Learn screens, landing sections, onboarding flows, lesson cards, feature explainers, agent-output panels, or other front-end UI that should match this illustrated paper classroom style."
---

# Paper Tutorial UI

## Overview

Use this skill to design interfaces in a warm, tutorial-video style: paper texture, classroom clarity, cartoon-outline components, and restrained playful color. The look sits between paper UI, hand-drawn anti-polish, light claymorphism, and editorial education graphics.

## Style Diagnosis

Name the style "Paper Tutorial UI" or "Warm Notebook Explainer".

Primary influences:
- Paper / notebook UI: cream background, faint grid or fiber texture, matte surfaces.
- Educational explainer graphics: centered diagrams, step badges, simple icon metaphors, generous empty space.
- Soft neubrutalism: thick outlines, offset shadows, confident flat shapes.
- Light claymorphism: rounded blocks with physical depth, but no glossy 3D or gradients.
- Editorial serif UI: bookish headings and labels rather than futuristic SaaS typography.

Use the style for learning products, agent workflows, feature walkthroughs, course pages, product onboarding, and small diagrammatic UI modules. Avoid it for dense trading dashboards, luxury ecommerce, cyberpunk/dark products, or serious enterprise admin screens unless explicitly requested.

## Visual System

Use a warm off-white base:
- `--paper: #F8F1DF`
- `--paper-soft: #FBF7EA`
- `--surface: #FFFDF5`
- `--ink: #334454`
- `--ink-dark: #263747`
- `--muted: #7A7F83`
- `--blue: #65B9E6`
- `--red: #EF5549`
- `--yellow: #F3B642`
- `--green: #4E8B6F`
- `--shadow: #2F4152`
- `--line-soft: rgba(51, 68, 84, 0.08)`

Keep the palette mostly cream, ink navy, and one accent per component. Use red for important actions or hero symbols, blue for agent/code/selection, yellow for badges and highlights, green for workflow/status.

Use paper texture through CSS, not heavy bitmap assets:

```css
.paper-bg {
  background-color: #F8F1DF;
  background-image:
    linear-gradient(rgba(51, 68, 84, 0.045) 1px, transparent 1px),
    linear-gradient(90deg, rgba(51, 68, 84, 0.045) 1px, transparent 1px);
  background-size: 24px 24px;
}
```

For a softer slide-like background, replace the grid with subtle diagonal hatching:

```css
.paper-hatch {
  background-color: #F8F1DF;
  background-image: repeating-linear-gradient(
    35deg,
    rgba(51, 68, 84, 0.035) 0,
    rgba(51, 68, 84, 0.035) 1px,
    transparent 1px,
    transparent 12px
  );
}
```

## Typography

Prefer serif headings for the captured look:
- Heading: `Fraunces`, `Libre Baskerville`, `Georgia`, serif.
- UI/body: `Nunito`, `Inter`, `system-ui`, sans-serif.
- Code/demo labels: `JetBrains Mono`, `SFMono-Regular`, monospace.

Use large editorial headings sparingly. Component labels should feel printed: medium weight, comfortable line height, no negative letter spacing, no all-caps unless the screenshot-like title needs a poster effect.

For a closer playful title treatment, pair a red fill with a navy offset shadow:

```css
.paper-title {
  color: #EF5549;
  font-family: Fraunces, Georgia, serif;
  font-weight: 800;
  text-shadow: 3px 3px 0 #334454;
}
```

## Components

Use thick outlined cards:

```css
.paper-card {
  background: #FFFDF5;
  border: 4px solid #334454;
  border-radius: 16px;
  box-shadow: 14px 14px 0 rgba(47, 65, 82, 0.16);
}
```

Use smaller icon tiles with stronger offset depth:

```css
.paper-tile {
  width: 88px;
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  background: #FFFDF5;
  border: 4px solid #334454;
  border-radius: 18px;
  box-shadow: 8px 8px 0 #334454;
}
```

Use step markers as circular badges partly overlapping the card corner:

```css
.step-badge {
  width: 42px;
  height: 42px;
  border-radius: 999px;
  display: grid;
  place-items: center;
  background: #F3B642;
  border: 4px solid #334454;
  color: #263747;
  font-weight: 800;
}
```

Use code/output panels as dark rounded capsules:
- Background `#334A5E` or `#2F4356`.
- Rounded corners 12-18px.
- Cyan code brackets or bot icons.
- Muted line placeholders in blue-gray.
- Keep inner content abstract and diagrammatic unless real code is needed.

## Layout

Favor slide-like composition:
- Center one focal object, then place 2-4 supporting tiles below it.
- Keep generous whitespace; do not pack many controls.
- Use large framed panels for demos and agent output.
- Let small badges overlap corners for a handmade instructional feel.
- Align content on simple horizontal or vertical axes; avoid complex masonry.
- On mobile, stack focal object, title, then tiles in a 2-column grid.

Use cards for actual modules and repeated items. Do not nest cards inside cards. Page sections should be full-width paper bands or unframed layouts.

## Icons And Illustration

Use Lucide or custom inline SVG icons with:
- 3-4px stroke.
- Rounded caps and joins.
- No emoji icons.
- Flat fill blocks with white symbol strokes for hero marks.
- Tiny accent marks such as sparks, cursor clicks, target rings, check circles, and simple workflow nodes.

Keep icons intentionally simple and readable at 32-64px. Use blue for tool/agent icons, red for security/primary feature icons, yellow for badges and sparks, green for flow/status.

## Motion

Use short tactile motion:
- Hover: translate `-2px -2px`, shadow grows by 2px.
- Active: translate `2px 2px`, shadow contracts.
- Duration: 140-220ms.
- Easing: `cubic-bezier(.2,.8,.2,1)`.
- Respect `prefers-reduced-motion`.

Avoid constant bouncing, parallax, glass blur, glow, and gradient-heavy animation.

## Implementation Checklist

Before delivery, verify:
- The first viewport reads as warm cream paper, not beige corporate SaaS.
- Major containers use 3-4px ink outlines and visible offset shadows.
- Accent colors are sparse: one primary accent per focal element.
- Background grid/hatching is subtle and does not reduce text contrast.
- Buttons, cards, and tiles have stable fixed dimensions or responsive constraints.
- Text does not overlap subtitles, watermarks, or decorative icons.
- Icons are SVG/Lucide-style, not emoji.
- Contrast reaches WCAG AA for body text and controls.
- Mobile layout preserves the central explainer composition without horizontal scroll.

## Avoid

Do not use dark mode as the default for this style. Do not use glassmorphism, purple-blue SaaS gradients, excessive beige/brown palettes, thin gray borders, tiny dense tables, stock photos, photorealistic 3D, or emoji-based iconography. Do not add decorative blobs/orbs; use paper texture, grid, frames, badges, and simple line icons instead.
