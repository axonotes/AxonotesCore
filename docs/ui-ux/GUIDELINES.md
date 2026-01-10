# Axonotes UI/UX Guidelines

**Version:** 1.0
**Last Updated:** January 2026

This document defines the UI and UX standards for Axonotes. All frontend code must follow these guidelines.

---

## Table of Contents

1. [Philosophy](#1-philosophy)
2. [Spacing System](#2-spacing-system)
3. [Typography](#3-typography)
4. [Colors & Theme](#4-colors--theme)
5. [Components](#5-components)
6. [Animation & Motion](#6-animation--motion)
7. [Responsive Design](#7-responsive-design)
8. [UX Patterns](#8-ux-patterns)
9. [Interaction](#9-interaction)
10. [Accessibility](#10-accessibility)
11. [Anti-Patterns](#11-anti-patterns)

---

## Severity Ratings

Each rule is rated by importance:

| Rating    | Label        | Meaning                                              |
| --------- | ------------ | ---------------------------------------------------- |
| **[5/5]** | Critical     | Breaking this creates serious UX problems. Must fix. |
| **[4/5]** | Important    | Should follow unless there's a strong reason not to. |
| **[3/5]** | Recommended  | Good practice, some flexibility allowed.             |
| **[2/5]** | Nice-to-have | Implement when time allows.                          |
| **[1/5]** | Optional     | Edge cases, special situations only.                 |

---

## 1. Philosophy

### 1.1 Every Element Earns Its Place [5/5]

Every UI element must serve a clear purpose. If an element can be removed without losing functionality or clarity, remove it.

**Before adding any element, ask:**

- Does this help the user complete their task?
- Is this information necessary right now?
- Can this be combined with something else?
- Would the interface work without this?

**Remove:**

- Decorative dividers that don't separate meaningful sections
- Labels that repeat what's already obvious
- Icons that don't add meaning beyond the text
- "Helper" text that states the obvious
- Unnecessary borders and backgrounds
- Empty states that just say "empty"

### 1.2 Every Action Feels Natural [5/5]

Users bring expectations from other applications they use daily. The interface should work the way users expect it to work, even if they're trying something for the first time.

**Core principle:** If a user tries an action by instinct (based on experience with other apps), it should work.

**Mental models to respect:**

- Ctrl/Cmd+S saves
- Ctrl/Cmd+Z undoes
- Escape closes modals/cancels
- Enter submits forms
- Tab moves between fields
- Right-click shows context menu
- Drag and drop reorders items
- Double-click edits/opens
- Clicking outside closes dropdowns/modals

**Platform conventions:**

- Follow OS conventions for keyboard shortcuts
- Match browser behavior for navigation (back, forward, refresh)
- Use familiar iconography (trash for delete, pencil for edit, plus for add)

### 1.3 Consistency Over Creativity [5/5]

Internal consistency is more important than novelty. The same action should look and behave the same way everywhere.

**Maintain consistency in:**

- Button styles and sizes for same-level actions
- Spacing between similar elements
- Error message patterns
- Loading state presentation
- Modal/dialog structure
- Icon usage and sizing
- Color usage for same semantic meaning

---

## 2. Spacing System

### 2.1 8px Base Grid [5/5]

All spacing uses multiples of 8px. This creates visual rhythm and consistency.

**Scale:**

```
4px   (space-1)   - Tight spacing, rare use
8px   (space-2)   - Minimal gaps, icon+text
12px  (space-3)   - Compact spacing
16px  (space-4)   - Standard spacing (DEFAULT)
20px  (space-5)   - Form field spacing
24px  (space-6)   - Section spacing, card padding
32px  (space-8)   - Major section separation (DEFAULT for sections)
40px  (space-10)  - Large gaps
48px  (space-12)  - Hero spacing
```

### 2.2 Tailwind Classes [5/5]

**Gap (between items):**

- `gap-2` (8px) - Icon + text, tightly related items
- `gap-4` (16px) - Standard spacing (DEFAULT)
- `gap-6` (24px) - Loose spacing
- `gap-8` (32px) - Section separation

**Padding (inside elements):**

- `p-2` (8px) - Dense UI, badges, small buttons
- `p-4` (16px) - Standard padding
- `p-6` (24px) - Cards, modals, panels (DEFAULT for cards)
- `p-8` (32px) - Large containers, page content

**Vertical Spacing:**

- `space-y-2` (8px) - Label + input pairs
- `space-y-4` (16px) - Component groups
- `space-y-5` (20px) - Form field groups (DEFAULT for forms)
- `space-y-6` (24px) - Card sections
- `space-y-8` (32px) - Major page sections (DEFAULT for sections)

### 2.3 Internal ≤ External Rule [4/5]

Spacing inside an element should be less than or equal to spacing outside it. This creates clear visual grouping.

**Correct:**

```svelte
<!-- Card has p-6 (24px) internal, gap-6 (24px) between cards -->
<div class="space-y-6">
  <div class="space-y-4 p-6">
    <!-- Internal 16px < External 24px -->
    <h3>Title</h3>
    <p>Content</p>
  </div>
  <div class="space-y-4 p-6">
    <h3>Title</h3>
    <p>Content</p>
  </div>
</div>
```

**Incorrect:**

```svelte
<!-- Internal spacing larger than external -->
<div class="space-y-2">
  <!-- 8px between cards -->
  <div class="p-8">
    <!-- 32px inside - feels disconnected -->
    ...
  </div>
</div>
```

---

## 3. Typography

### 3.1 Size Hierarchy [4/5]

```
12px (text-xs)   - Captions, timestamps, metadata
14px (text-sm)   - UI labels, secondary content (DEFAULT for UI)
16px (text-base) - Body text, paragraphs (DEFAULT for body)
18px (text-lg)   - Emphasized text, lead paragraphs
20px (text-xl)   - Small headings (H4)
24px (text-2xl)  - Section headings (H3)
30px (text-3xl)  - Page headings (H2) (DEFAULT for headings)
36px (text-4xl)  - Hero headings (H1)
```

### 3.2 Font Weights [4/5]

Limit to 3 weights for visual clarity:

```
font-normal (400)   - Body text, descriptions
font-medium (500)   - Emphasized text, UI labels
font-semibold (600) - Headings, important actions (DEFAULT for headings)
```

**Avoid:** `font-light` (300) and `font-bold` (700) unless absolutely necessary.

### 3.3 Line Height [3/5]

```
leading-tight (1.25)   - Headings, large text (DEFAULT for headings)
leading-snug (1.375)   - Subheadings
leading-normal (1.5)   - Body text (DEFAULT for body)
leading-relaxed (1.625) - Long-form content
```

### 3.4 Letter Spacing [3/5]

```
tracking-tight (-0.025em) - Large headings (text-2xl and above)
tracking-normal (0)       - Default
tracking-wide (0.025em)   - Small caps, labels (text-xs uppercase)
```

### 3.5 Internationalization (Paraglide) [5/5]

All user-facing text must use Paraglide for internationalization. Never hardcode strings.

**Usage:**

```svelte
<script>
  import * as m from "$lib/paraglide/messages.js";
</script>

<h1>{m.auth_unlock_title()}</h1>
<p>{m.auth_unlock_description()}</p>
<Button>{m.common_button_save()}</Button>
```

**Key Naming Convention:**

Structure: `{scope}_{feature}_{element}_{modifier}`

| Part         | Description              | Examples                                                                        |
| ------------ | ------------------------ | ------------------------------------------------------------------------------- |
| **scope**    | Top-level section        | `auth`, `editor`, `settings`, `dashboard`, `common`                             |
| **feature**  | Specific feature/page    | `unlock`, `signin`, `profile`, `document`, `sidebar`                            |
| **element**  | UI element type          | `button`, `input`, `label`, `title`, `description`, `error`, `success`, `toast` |
| **modifier** | Variant/state (optional) | `primary`, `secondary`, `loading`, `empty`, `incorrect`, `placeholder`          |

**Examples:**

```
auth_unlock_title                    - "Welcome back"
auth_unlock_description              - "Enter your password to continue"
auth_unlock_button_primary           - "Unlock"
auth_unlock_button_loading           - "Unlocking..."
auth_unlock_error_incorrect          - "Incorrect password"

common_button_save                   - "Save"
common_button_cancel                 - "Cancel"
common_error_network                 - "Network error. Please try again."

editor_document_title_placeholder    - "Untitled"
editor_document_toast_autosaved      - "Document autosaved"
```

**Rules:**

- For reusable text (Save, Cancel, Delete), use `common_*` prefix
- Keys are deterministic: following the pattern, you know exactly what the key should be
- Related keys group alphabetically for easy scanning

### 3.6 Standard Patterns [4/5]

```svelte
<!-- Page heading -->
<h1 class="text-3xl font-semibold tracking-tight leading-tight">

<!-- Section heading -->
<h2 class="text-2xl font-semibold tracking-tight leading-tight">

<!-- Card/component title -->
<h3 class="text-lg font-medium leading-snug">

<!-- UI label -->
<span class="text-sm font-medium">

<!-- Body text -->
<p class="text-base leading-normal">

<!-- Secondary/muted text -->
<p class="text-sm text-muted-foreground">

<!-- Caption/metadata -->
<span class="text-xs text-muted-foreground">
```

---

## 4. Colors & Theme

### 4.1 Never Hardcode Colors [5/5]

Always use theme tokens. Never use Tailwind color utilities directly.

**Correct:**

```svelte
<div class="bg-card text-foreground">
<p class="text-muted-foreground">
<button class="bg-primary text-primary-foreground">
<div class="border border-destructive">
```

**Incorrect:**

```svelte
<div class="bg-gray-100 text-gray-900">
<p class="text-gray-500">
<button class="bg-blue-500 text-white">
<div class="border border-red-500">
```

**Why:** Theme tokens automatically adapt to light/dark mode.

### 4.2 Semantic Color Usage [5/5]

**Backgrounds:**

```
bg-background       - Main app background
bg-card             - Elevated surfaces (cards, modals, panels)
bg-muted            - Subtle backgrounds, disabled states
bg-accent           - Hover states, highlighted areas
bg-primary          - Primary action buttons
bg-destructive      - Destructive actions, error states
```

**Text:**

```
text-foreground          - Primary text (DEFAULT)
text-muted-foreground    - Secondary text, hints, placeholders
text-primary             - Accent text, links
text-destructive         - Error messages
text-primary-foreground  - Text on primary background
```

**Borders:**

```
border              - Default borders (uses theme)
border-input        - Form inputs
border-primary      - Focused/active elements
border-destructive  - Error states
```

### 4.3 Status Colors [4/5]

For status indicators, use semantic classes:

```svelte
<!-- Success -->
<div class="text-green-600 dark:text-green-500">

<!-- Warning -->
<div class="text-yellow-600 dark:text-yellow-500">

<!-- Error -->
<div class="text-destructive">

<!-- Info -->
<div class="text-primary">
```

### 4.4 No Gradients [4/5]

Use flat, solid colors only. Gradients add visual noise without functional benefit.

**Correct:**

```svelte
<div class="bg-primary">
<div class="bg-card">
```

**Incorrect:**

```svelte
<div class="bg-gradient-to-r from-blue-500 to-purple-500">
<div style="background: linear-gradient(...)">
```

### 4.5 Shadows [3/5]

Use shadows sparingly and only for elevation hierarchy.

**When to use:**

- Dropdowns and popovers: `shadow-md`
- Modals: `shadow-lg`
- Floating elements (FABs, toasts): `shadow-md`

**When NOT to use:**

- Cards (use borders instead)
- Buttons
- Decorative purposes

**Correct:**

```svelte
<!-- Dropdown -->
<div class="rounded-md border bg-popover shadow-md">

<!-- Modal -->
<div class="rounded-lg border bg-card shadow-lg">

<!-- Card - no shadow, use border -->
<div class="rounded-lg border bg-card">
```

---

## 5. Components

### 5.1 Button & Input Heights [4/5]

```
h-8  (32px) - Small/compact (table actions, inline buttons)
h-10 (40px) - Default (most buttons and inputs)
h-11 (44px) - Large/prominent (primary CTAs, auth forms)
```

**Usage:**

```svelte
<!-- Standard button -->
<Button class="h-10">Save</Button>

<!-- Prominent CTA -->
<Button class="h-11 w-full">Sign In</Button>

<!-- Compact/inline -->
<Button size="sm" class="h-8">Edit</Button>
```

### 5.2 Icons (Lucide) [4/5]

Always use Lucide icon components. Never write SVG markup directly.

**Correct:**

```svelte
<script>
  import {Trash2, Pencil, Plus} from "@lucide/svelte";
</script>

<button>
  <Trash2 class="h-4 w-4" />
</button>
```

**Incorrect:**

```svelte
<!-- NEVER write SVG directly -->
<svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
  <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7..." />
</svg>
```

If an icon doesn't exist in Lucide, create a reusable component in `$lib/components/icons/`.

**Icon Sizes:**

```
h-3 w-3 (12px) - Inline with text-xs
h-4 w-4 (16px) - Standard UI icons (DEFAULT)
h-5 w-5 (20px) - Larger UI elements, sidebar icons
h-6 w-6 (24px) - Hero icons, emphasis
```

**Icon + Text Pattern:**

```svelte
<button class="flex items-center gap-2">
  <Icon class="h-4 w-4" />
  <span>Label</span>
</button>
```

### 5.3 Cards & Panels [4/5]

**Standard card:**

```svelte
<div class="rounded-lg border bg-card p-6">
```

**Border radius scale:**

```
rounded-sm (2px)  - Small elements, tags
rounded-md (6px)  - Buttons, inputs (DEFAULT for small elements)
rounded-lg (8px)  - Cards, panels (DEFAULT for cards)
rounded-xl (12px) - Large modals, hero sections
```

### 5.4 Container Widths [4/5]

```
max-w-sm (384px)  - Auth forms, narrow dialogs
max-w-md (448px)  - Standard forms
max-w-lg (512px)  - Wide forms
max-w-2xl (672px) - Reading content
max-w-4xl (896px) - Wide content areas
```

---

## 6. Animation & Motion

### 6.1 Organic Easing [4/5]

Use easing curves with overshoot for spatial animations. This creates a natural, "alive" feel.

**For position/transform changes:**

```css
transition-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1);
```

The `1.56` value exceeds 1.0, causing the element to overshoot its target and settle back. This mimics real-world physics.

**In Tailwind (inline style required for custom bezier):**

```svelte
<div
  class="transition-transform duration-300"
  style="transition-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1);"
>
```

**For color/opacity (no overshoot needed):**

```svelte
<button class="transition-colors duration-150">
```

### 6.2 Duration Hierarchy [4/5]

```
75ms   - Instant feedback (button press)
150ms  - Quick transitions (color, opacity) (DEFAULT for colors)
200ms  - Standard transitions (transforms, scale) (DEFAULT for transforms)
300ms  - Layout changes, spatial movement (DEFAULT for position)
400ms+ - Large/complex animations (use sparingly)
```

**Tailwind classes:**

```
duration-75   - Instant
duration-150  - Colors, opacity
duration-200  - Transforms, scale
duration-300  - Position, layout
```

### 6.3 What to Animate [3/5]

**Always animate:**

- Hover state changes (color, background)
- Focus indicators
- Modal/dropdown open/close
- Drag operations
- State transitions (expanded/collapsed)

**Consider animating:**

- List item additions/removals
- Tab switches
- Loading skeleton shimmer

**Never animate:**

- Initial page load (content should appear ready)
- User input (typing, clicking)
- Error states (should appear instantly)
- Critical information

### 6.4 Transition Properties [3/5]

```svelte
<!-- Color changes -->
<button class="transition-colors duration-150 hover:bg-accent">

<!-- Scale on hover -->
<div class="transition-transform duration-200 hover:scale-[1.02]">

<!-- Opacity -->
<div class="transition-opacity duration-150">

<!-- Multiple properties (use sparingly) -->
<div class="transition-all duration-200">
```

### 6.5 Svelte Transitions [3/5]

For enter/exit animations:

```svelte
<script>
  import { slide, fade } from 'svelte/transition';
</script>

<!-- Error message slide -->
{#if error}
  <div transition:slide={{duration: 150}}>
    <p class="text-destructive text-sm">{error}</p>
  </div>
{/if}

<!-- Fade for overlays -->
{#if showOverlay}
  <div transition:fade={{duration: 150}}>
```

---

## 7. Responsive Design

### 7.1 Desktop-First, Mobile-Aware [3/5]

Primary target is desktop. Mobile should work but is lower priority.

**Breakpoints:**

```
sm: 640px   - Small tablets
md: 768px   - Tablets
lg: 1024px  - Small desktops
xl: 1280px  - Standard desktops
2xl: 1536px - Large screens
```

### 7.2 Touch Targets [4/5]

Interactive elements must be at least 44x44px on touch devices.

```svelte
<!-- Ensure minimum touch target -->
<button class="min-h-[44px] min-w-[44px]">
```

### 7.3 Responsive Patterns [3/5]

```svelte
<!-- Stack on mobile, row on desktop -->
<div class="flex flex-col gap-4 md:flex-row">

<!-- Full width on mobile, constrained on desktop -->
<div class="w-full max-w-sm mx-auto">

<!-- Adjust padding for screen size -->
<div class="p-4 md:p-6 lg:p-8">
```

---

## 8. UX Patterns

### 8.1 Feedback States [5/5]

Every action must have immediate, visible feedback.

**Loading:**

```svelte
<Button disabled={isLoading}>
  {#if isLoading}
    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
    Processing...
  {:else}
    Submit
  {/if}
</Button>
```

**Success:**

```svelte
{#if saved}
  <div class="flex items-center gap-2 text-sm text-green-600">
    <CheckCircle class="h-4 w-4" />
    Changes saved
  </div>
{/if}
```

**Error:**

```svelte
{#if error}
  <div transition:slide={{duration: 150}}>
    <p class="text-destructive text-sm">{error}</p>
  </div>
{/if}
```

### 8.2 Empty States [4/5]

Empty states should guide users toward action, not just state "nothing here."

**Structure:**

```svelte
<div class="flex flex-col items-center justify-center py-12 text-center">
  <Icon class="text-muted-foreground mb-4 h-12 w-12" />
  <h3 class="mb-2 text-lg font-medium">No documents yet</h3>
  <p class="text-muted-foreground mb-4 text-sm">
    Create your first document to get started
  </p>
  <Button>Create Document</Button>
</div>
```

**Rules:**

- Include an icon (muted, not prominent)
- Brief title explaining the state
- Short description with next step
- Primary action button when applicable

### 8.3 Form Validation [4/5]

**When to validate:**

- On blur for individual fields
- On submit for the whole form
- Real-time only for specific cases (password strength)

**Error placement:**

```svelte
<div class="space-y-2">
  <Label for="email">Email</Label>
  <Input id="email" type="email" class={error ? "border-destructive" : ""} />
  {#if error}
    <p class="text-destructive flex items-center gap-1 text-sm">
      <AlertCircle class="h-3 w-3" />
      {error}
    </p>
  {/if}
</div>
```

**Error messages must:**

- Be specific ("Email must include @" not "Invalid email")
- Appear immediately below the field
- Not shift layout unexpectedly (reserve space or use transitions)

### 8.4 User Control [4/5]

Users must always be able to escape, undo, or go back.

**Escape hatches:**

- Escape key closes modals/dropdowns
- Click outside closes popups
- Cancel button on forms
- Back navigation works

**Undo for destructive actions:**

```svelte
<!-- Toast with undo -->
<div
  class="bg-card flex items-center justify-between gap-4 rounded-lg border p-4"
>
  <span class="text-sm">Item deleted</span>
  <Button variant="outline" size="sm" onclick={undo}>Undo</Button>
</div>
```

**Confirmation for irreversible actions:**

```svelte
<AlertDialog>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Delete permanently?</AlertDialogTitle>
      <AlertDialogDescription>
        This action cannot be undone.
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>Cancel</AlertDialogCancel>
      <AlertDialogAction class="bg-destructive">Delete</AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
```

### 8.5 Progressive Disclosure [4/5]

Show only what's needed. Hide complexity until the user asks for it.

**Patterns:**

- Collapsible sections for advanced options
- "Show more" for long lists
- Tooltips for explanations
- Modals for detailed settings

```svelte
<!-- Basic search visible, advanced hidden -->
<Input placeholder="Search..." />

{#if showAdvanced}
  <div transition:slide class="bg-muted/50 space-y-3 rounded-lg border p-4">
    <!-- Advanced filters -->
  </div>
{/if}

<button
  class="text-muted-foreground hover:text-foreground text-sm"
  onclick={() => (showAdvanced = !showAdvanced)}
>
  {showAdvanced ? "Hide" : "Show"} advanced options
</button>
```

### 8.6 Micro-copy [4/5]

**Button labels:**

- Use verbs: "Save", "Create", "Delete", "Cancel"
- Be specific: "Save Document" not just "Save" (when context unclear)
- Match the action: "Delete" for destructive, "Remove" for reversible

**Error messages:**

- Explain what happened and how to fix it
- Don't blame the user
- Be concise

```
Good: "Password must be at least 8 characters"
Bad:  "Error: Invalid password"
Bad:  "You entered an invalid password. Passwords must contain..."
```

**Placeholder text:**

- Show format examples: "name@example.com"
- Don't repeat the label
- Don't use as the only label

**Confirmation dialogs:**

- Title: What will happen ("Delete document?")
- Description: Consequences ("This cannot be undone")
- Actions: Clear verb labels ("Delete" / "Cancel")

### 8.7 Reduce Cognitive Load [3/5]

**Hick's Law:** More choices = slower decisions

- Limit options when possible (3-5 items ideal)
- Group related options
- Provide defaults
- Use progressive disclosure for advanced options

**Miller's Law:** 7±2 items in working memory

- Break long lists into groups
- Use pagination or infinite scroll for large datasets
- Don't show more than 7 ungrouped items at once

### 8.8 Subtle Delight [3/5]

Playful design works when it **enhances** the experience without **distracting** from the task. Delight is the "icing on the cake" - it comes after functional, reliable, and usable.

**The Principle:**

For a productivity app, the core experience should be **invisible** - users should enter a flow state and forget they're using software. Save playfulness for the **edges** of the experience.

**Where subtle delight works:**

| Context                  | Example                                            |
| ------------------------ | -------------------------------------------------- |
| **Transitions**          | Overshoot easing on position changes (feels alive) |
| **Success moments**      | Subtle animation on save/complete                  |
| **Empty states**         | Friendly illustration + encouraging text           |
| **Loading states**       | Skeleton shimmer instead of spinners               |
| **Drag & drop**          | Physics-based, smooth movement                     |
| **Workspace management** | Playful but functional interactions                |

**Where to avoid playfulness:**

| Context                       | Why                                 |
| ----------------------------- | ----------------------------------- |
| **Error states**              | Keep serious, clear, actionable     |
| **Destructive confirmations** | Not the time for whimsy             |
| **Core editing/writing**      | Should be invisible (flow state)    |
| **Frequent repeated actions** | Novelty wears off, becomes annoying |

**Rules for subtle delight:**

- **Under 500ms** - anything longer drags and interrupts
- **Purposeful** - every animation has a reason to exist
- **Skippable** - power users shouldn't be slowed down
- **Contextual** - match the emotional moment (don't celebrate errors)
- **Rare enough to stay fresh** - overuse kills the magic

**Organic motion:**

Make animations feel alive with physics-inspired behavior:

```css
/* Overshoot easing - element goes past target, settles back */
transition-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1);
```

- Use overshoot for spatial movement (feels natural)
- Use acceleration/deceleration (not linear)
- Small scale bounces on completion (1.05 → 1.0)

**What NOT to do:**

- Confetti/particles for routine actions
- Sound effects (unless explicitly enabled)
- Animations that block user input
- Forced waiting for animations to complete
- Cutesy copy in serious contexts

---

## 9. Interaction

### 9.1 Hover States [4/5]

All interactive elements need hover feedback.

```svelte
<!-- Button hover -->
<button class="hover:bg-accent transition-colors duration-150">

<!-- Link hover -->
<a class="hover:underline underline-offset-4">

<!-- Card hover (if clickable) -->
<div class="hover:bg-accent/50 transition-colors duration-150 cursor-pointer">
```

### 9.2 Focus States [5/5]

Focus indicators must be visible for keyboard navigation.

```svelte
<!-- Standard focus ring -->
<button class="focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none">

<!-- Input focus -->
<input class="focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none">
```

**Never remove focus outlines:**

```svelte
<!-- NEVER do this -->
<button class="outline-none focus:outline-none">
```

### 9.3 Disabled States [4/5]

```svelte
<button
  disabled={isDisabled}
  class="disabled:opacity-50 disabled:cursor-not-allowed"
>
```

**Note:** Consider keeping pointer events to allow tooltips explaining why it's disabled.

### 9.4 Active/Pressed States [3/5]

```svelte
<!-- Scale down on press -->
<button class="active:scale-95 transition-transform">

<!-- Darken on press -->
<button class="active:brightness-90">
```

### 9.5 Keyboard Support [3/5]

All functionality should be accessible via keyboard.

**Essential shortcuts to support:**

- `Tab` / `Shift+Tab` - Navigate between elements
- `Enter` / `Space` - Activate buttons/links
- `Escape` - Close modals, cancel actions
- `Arrow keys` - Navigate within components (menus, tabs)

**Custom shortcuts (implement over time):**

- `Ctrl/Cmd+S` - Save
- `Ctrl/Cmd+Z` - Undo
- `Ctrl/Cmd+Shift+Z` - Redo
- `Ctrl/Cmd+K` - Command palette

**For custom interactive elements:**

```svelte
<div
  role="button"
  tabindex="0"
  onclick={handleClick}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  }}
>
```

---

## 10. Accessibility

### 10.1 ARIA Labels [4/5]

Icon-only buttons must have labels:

```svelte
<button aria-label="Close dialog">
  <X class="h-4 w-4" />
</button>

<button aria-label="Edit document">
  <Pencil class="h-4 w-4" />
</button>
```

### 10.2 Form Labels [4/5]

All inputs must have associated labels:

```svelte
<div class="space-y-2">
  <Label for="email">Email</Label>
  <Input id="email" type="email" aria-describedby="email-help" />
  <p id="email-help" class="text-muted-foreground text-sm">
    We'll never share your email
  </p>
</div>
```

### 10.3 Semantic HTML [4/5]

Use semantic elements:

```svelte
<!-- Correct -->
<nav>
<main>
<article>
<aside>
<header>
<footer>
<button>
<a href="...">

<!-- Avoid -->
<div onclick="...">  <!-- Use button -->
<span onclick="..."> <!-- Use button or a -->
```

### 10.4 Color Contrast [4/5]

Minimum contrast ratios (WCAG AA):

- Normal text: 4.5:1
- Large text (18px+): 3:1
- UI components: 3:1

Theme tokens are designed to meet these requirements.

---

## 11. Anti-Patterns

### Things to Never Do

**[5/5] Hardcoded colors:**

```svelte
<!-- NEVER -->
<div class="bg-gray-100 text-gray-700">
<div class="bg-[#f5f5f5]">
```

**[5/5] Hardcoded text (no Paraglide):**

```svelte
<!-- NEVER -->
<h1>Welcome back</h1>
<Button>Save</Button>

<!-- ALWAYS use Paraglide -->
<h1>{m.auth_unlock_title()}</h1>
<Button>{m.common_button_save()}</Button>
```

**[4/5] Inline SVG instead of Lucide:**

```svelte
<!-- NEVER write SVG directly -->
<svg viewBox="0 0 24 24"><path d="..." /></svg>

<!-- ALWAYS use Lucide components -->
<Trash2 class="h-4 w-4" />
```

**[4/5] Gradients:**

```svelte
<!-- NEVER -->
<div class="bg-gradient-to-r from-blue-500 to-purple-500">
```

**[5/5] Missing loading states:**

```svelte
<!-- NEVER - no feedback during async operation -->
<button onclick={saveData}>Save</button>
```

**[5/5] No error handling:**

```svelte
<!-- NEVER - errors silently fail -->
{#await promise}
  <Spinner />
{:then data}
  <Content {data} />
{/await}
<!-- Missing {:catch} -->
```

**[4/5] Layout shift on state change:**

```svelte
<!-- AVOID - error appearing shifts content -->
{#if error}
  <p class="text-destructive">{error}</p>
{/if}
<!-- Better: use transition:slide or reserve space -->
```

**[4/5] Inconsistent spacing:**

```svelte
<!-- AVOID - mixing spacing systems -->
<div class="mb-3">   <!-- 12px -->
<div class="mb-5">   <!-- 20px -->
<div class="mb-[17px]">  <!-- arbitrary -->
```

**[4/5] Removed focus outlines:**

```svelte
<!-- NEVER -->
<button class="outline-none focus:outline-none">
```

**[3/5] Overusing animations:**

```svelte
<!-- AVOID - everything animates -->
<div class="transition-all duration-500 animate-bounce">
```

**[3/5] Using `transition-all` everywhere:**

```svelte
<!-- AVOID - performance cost, unintended animations -->
<button class="transition-all">

<!-- Better - specify what animates -->
<button class="transition-colors">
```

---

## Quick Reference

### Most Common Patterns

**Spacing:**

- Between items: `gap-4` (16px)
- Inside cards: `p-6` (24px)
- Between sections: `space-y-8` (32px)
- Form fields: `space-y-5` (20px)

**Typography:**

- UI labels: `text-sm font-medium`
- Body: `text-base leading-normal`
- Headings: `text-3xl font-semibold tracking-tight leading-tight`

**Components:**

- Buttons/Inputs: `h-10` default, `h-11` for CTAs
- Icons: `h-4 w-4`
- Cards: `rounded-lg border bg-card p-6`

**Colors:**

- Primary text: `text-foreground`
- Secondary text: `text-muted-foreground`
- Backgrounds: `bg-background`, `bg-card`, `bg-muted`
- Interactive: `hover:bg-accent transition-colors duration-150`

**Animation:**

- Color changes: `transition-colors duration-150`
- Transforms: `transition-transform duration-200`
- Position with overshoot: `duration-300` + `cubic-bezier(0.34, 1.56, 0.64, 1)`

---

## Review Checklist

Before considering frontend work complete:

- [ ] Every element serves a purpose (no decorative extras)
- [ ] Spacing follows 8px grid
- [ ] No hardcoded colors (theme tokens only)
- [ ] No gradients (flat colors only)
- [ ] All text uses Paraglide (no hardcoded strings)
- [ ] Icons use Lucide components (no inline SVG)
- [ ] Correct component sizes (h-10 buttons, h-4 w-4 icons)
- [ ] Loading state implemented for async actions
- [ ] Error state implemented and visible
- [ ] Hover states on interactive elements
- [ ] Focus states visible (no outline-none)
- [ ] Keyboard accessible
- [ ] ARIA labels on icon-only buttons
- [ ] Matches existing patterns in codebase
