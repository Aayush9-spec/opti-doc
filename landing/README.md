# OptiDock Landing README

## Project Context

This `landing` directory contains the Next.js marketing and documentation site for OptiDock AI.

The site is meant to present OptiDock as:

- Rust-first
- terminal-first
- infrastructure-focused
- serious and production-minded

## Important Design Context

The UI direction for this website is based on React Bits-style interface components.

That means future UI work in this folder should prefer:

- expressive component-driven sections
- visually intentional layouts
- interactive background systems
- polished, reusable presentation blocks

This context should be preserved when adding or changing the landing site.

## Current Background Components

The site currently uses custom React components for ambient motion:

- `components/LetterGlitch.tsx`
  - used as the global monochrome background
- `components/Threads.tsx`
  - used as the background treatment for the documentation hero

## Current Pages

- `/`
  - landing page
- `/docs`
  - documentation page
- `/use-cases`
  - use cases page

## Styling Rules

- Keep the visual system monochrome unless a new direction is explicitly requested
- Preserve readability over heavy effects
- Keep the product feeling terminal-native, not generic SaaS
- Reuse and extend component-based UI patterns where possible

## Notes For Future Work

- If new React Bits-inspired components are provided, they should be integrated into this site instead of building throwaway one-off sections
- Prefer reusable React components over static HTML fragments
- Keep all design rationale aligned with `design.md`
