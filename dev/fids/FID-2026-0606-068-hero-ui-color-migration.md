# FID-068: Migrate Dashboard Colors to HeroUI Theme System

**Status:** created
**Severity:** low
**Created:** 2026-06-06
**Author:** Kilo

---

## Problem

Dashboard imports HeroUI styles (`@import "@heroui/styles"`) and uses HeroUI components (`ProgressBarRoot`, `ProgressBarFill`), but the color system is entirely custom CSS variables in `globals.css` (`--cyan`, `--green`, `--red`, `--amber`, `--violet`). HeroUI's semantic color tokens (`danger`, `warning`, `primary`, `success`, etc.) are not used.

This means:
- Colors don't benefit from HeroUI's theme system (dark mode, contrast, accessibility)
- Adding new colors requires manual CSS variable management
- Inconsistent with HeroUI's design system

## Goal

Migrate from custom CSS variables to HeroUI's semantic color tokens. Use `text-danger`, `text-warning`, `text-primary`, `text-success` etc. instead of `text-[var(--red)]`, `text-[var(--amber)]`, etc.

## Scope

- Audit which HeroUI color tokens map to current custom colors
- Update all `var(--*)` color references to HeroUI tokens
- Remove custom color variables from `globals.css`
- Verify visual consistency

## Deferred

Low priority — current custom system works. Migrate when HeroUI theme becomes relevant (e.g., light mode support, accessibility audit).
