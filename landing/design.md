# OptiDock Landing Design Context

## Purpose

This landing site exists to present OptiDock AI as a serious, terminal-first container operations product rather than a lightweight marketing page. The site should support three goals:

1. explain the product clearly
2. make the CLI feel premium and credible
3. provide navigable documentation and practical use cases

## Platform

- Framework: Next.js App Router
- Language: TypeScript
- Styling: custom global CSS in `app/globals.css`
- Shared visual component: `components/LetterGlitch.tsx`

## Visual Direction

The site uses a monochrome design system.

- Primary palette: black, off-black, charcoal, silver, white
- No colorful accents
- Contrast comes from opacity, texture, blur, and typography
- The page should feel cinematic, technical, and restrained

The overall mood should feel like:

- terminal-native
- infrastructure-focused
- premium but not glossy
- confident without looking like a startup template

## Background System

The `LetterGlitch` component is the ambient background for the full site, not just a demo box.

### Background rules

- It should sit behind all page content globally in `app/layout.tsx`
- It should remain subtle enough that text stays readable
- The active palette for the glitch effect is monochrome:
  - `#1a1a1a`
  - `#4d4d4d`
  - `#d8d8d8`
- The overlay layers should darken and soften the canvas so the site feels atmospheric rather than noisy

## Layout Structure

### Global layout

- Shared top navigation
- Shared background effect
- Shared constrained content width

### Primary pages

- `/`
  - hero message
  - terminal command preview
  - product positioning
  - install flow and core loop
- `/docs`
  - installation guidance
  - analysis command usage
  - pipeline moderation explanation
  - provider strategy summary
- `/use-cases`
  - audience-specific value framing
  - practical deployment and audit scenarios

## Page Maps

### `/` Home page

Purpose:

- establish product identity quickly
- reinforce the terminal-first nature of OptiDock
- direct users into docs or use cases

Sections:

1. Header navigation
   - brand mark
   - links to Overview, Documentation, Use Cases
2. Hero section
   - product statement
   - supporting copy
   - primary CTA to docs
   - secondary CTA to use cases
   - CLI preview panel
3. Product positioning panel
   - reasons the tool exists
   - trust-building product signals
4. Install panel
   - install commands
   - first-run commands
5. Core loop panel
   - short operational workflow summary

### `/docs` Documentation page

Purpose:

- help users understand how to operate the product right now
- frame the current implementation honestly
- provide a credible quick-start reference

Sections:

1. Header navigation
2. Documentation hero
   - `Threads` background treatment
   - page heading
   - explanation of the docs scope
3. Documentation cards
   - installation
   - analysis
   - pipeline moderation
4. Provider strategy panel
   - provider-agnostic architecture summary

### `/use-cases` Use cases page

Purpose:

- show who the product is for
- connect features to practical deployment and audit workflows

Sections:

1. Header navigation
2. Intro hero panel
   - positioning around effectiveness and practical fit
3. Use case card grid
   - startup teams
   - platform engineers
   - consultants
   - AI-native internal tools
4. Practical fit panel
   - summary of where the product is strongest

## Typography

Typography intentionally mixes editorial and technical tone.

- Main reading and headlines: serif stack
- Labels, pills, code, navigation: monospace stack

This combination is meant to make the product feel more distinctive than a default SaaS layout while still remaining readable and technical.

## Component Intent

## Component Inventory

### Layout-level components

- `app/layout.tsx`
  - owns shared navigation
  - mounts the global `LetterGlitch` background
  - wraps all pages in the same shell

### Background components

- `components/LetterGlitch.tsx`
  - global monochrome animated canvas background
  - should remain subtle and atmospheric
- `components/Threads.tsx`
  - used specifically for the documentation hero
  - adds layered motion with optional mouse interaction

### Structural UI patterns

- brand mark
  - small product identifier in the header
- nav pill links
  - rounded monochrome navigation chips
- panel
  - reusable glass-dark content container
- hero panel / terminal panel
  - stronger showcase containers for key messaging
- code block
  - monospace reference surface for commands and examples
- pill row
  - lightweight status or product-tag elements
- use case card
  - reusable content card for audience scenarios

### `LetterGlitch`

- Rebuilt as a React client component from the provided canvas logic
- Used as a site-wide background
- Supports vignette and smooth transitions

### Navigation

- Minimal links only
- Rounded monochrome chips
- Should feel like a control surface, not a blog nav

### Panels

- Glassy dark surfaces
- Soft borders
- Strong radius
- Heavy shadow for depth

### Hero terminal card

- Used to reinforce that OptiDock is terminal-first
- Must feel like a real command surface
- Should show practical commands instead of placeholder marketing text

## Color And Token Definitions

The site is intentionally monochrome, so tokens rely on contrast, opacity, and layering rather than hue.

### Base tokens

- `--bg: #050505`
  - main page background
- `--bg-soft: #0d0d0d`
  - softer dark backing tone
- `--surface: rgba(10, 10, 10, 0.78)`
  - standard panel background
- `--surface-strong: rgba(18, 18, 18, 0.92)`
  - stronger block for terminal/code areas
- `--surface-border: rgba(255, 255, 255, 0.12)`
  - panel and chip borders
- `--text: #f3f3f3`
  - primary text
- `--muted: #afafaf`
  - secondary text
- `--soft: #8a8a8a`
  - tertiary and utility text
- `--accent: #ffffff`
  - high-emphasis highlight in a monochrome system
- `--shadow: 0 30px 100px rgba(0, 0, 0, 0.5)`
  - main depth shadow

### Radius tokens

- `--radius-lg: 28px`
  - large panels
- `--radius-md: 20px`
  - code blocks and tighter surfaces

### Token usage rules

- Do not introduce saturated accent colors unless the design direction changes explicitly
- Use `--text`, `--muted`, and `--soft` to create hierarchy
- Use border opacity and blur for depth before adding more decoration
- Preserve a dark-field atmosphere across all pages

## Content Strategy

The copy should reflect the real repository direction:

- Rust-first
- terminal-first
- deterministic analysis before AI
- deployment-aware optimization
- provider-flexible architecture

Avoid copy that makes the product sound finished in areas that are still conceptual. The tone should be ambitious but grounded.

## UX Principles

- The site should read quickly
- Important information should be chunked into panels
- Motion should come primarily from the background effect, not from excessive UI animation
- The monochrome palette should never make the page feel flat; depth should come from layers and contrast

## Responsive Behavior

- Desktop: asymmetrical hero with text and terminal card
- Tablet/mobile: stacked layout
- Navigation wraps cleanly without breaking visual rhythm
- Panels maintain spacing and readability on smaller screens

## File Map

- `app/layout.tsx`
  - global shell, navigation, background composition
- `app/page.tsx`
  - landing page
- `app/docs/page.tsx`
  - product documentation page
- `app/use-cases/page.tsx`
  - use cases page
- `app/globals.css`
  - full design system and layout styling
- `components/LetterGlitch.tsx`
  - animated background component

## Future Design Extensions

Good next improvements if the site expands:

- add a CLI screenshot or rendered terminal transcript section
- add a roadmap page
- add benchmark comparison visual blocks
- add install steps with copy buttons
- add a sticky docs sidebar if documentation grows

## React Bits Usage Rules

The landing site should be built with a React Bits-style mindset.

This means:

- prefer reusable visual components over one-off page fragments
- allow motion and texture to support the product narrative
- use background and section components intentionally, not as decoration for its own sake
- keep components composable so new supplied pieces can be slotted into the site cleanly

### When new components are provided

- integrate them as actual React components
- do not leave them as standalone HTML or snippet-only artifacts
- place them in `components/`
- style them to match the existing monochrome system unless explicitly asked otherwise
- document their role in both `README.md` and `design.md` if they affect the site direction

### React Bits compatibility expectations

- components should feel expressive and modern
- interaction should remain smooth and purposeful
- sections should avoid generic template layouts
- effects should not damage readability or usability

### Guardrails for component use

- avoid stacking multiple heavy effects in the same viewport region
- keep one dominant motion system per section when possible
- prioritize composability and readability over novelty
- preserve the terminal-first product identity even when adding more visual components

## Guardrails

- Keep the site monochrome unless explicitly asked otherwise
- Keep the glitch background subtle and readable
- Avoid generic gradient-heavy SaaS visuals
- Preserve the feeling that OptiDock is a terminal product first
