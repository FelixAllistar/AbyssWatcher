# Feature Specification: Alphabetical Sorting of Character Totals

## Objective
Sort the character list/totals in the UI alphabetically by character name.

## Context
The user has requested that the "totals at bottom" be sorted alphabetically. This refers to the list of characters and their aggregated metrics displayed in the application's UI. Currently, the sort order is likely insertion order or random.

## Requirements

### 1. Sorting Logic
- The list of characters displayed in the UI must be sorted alphabetically (A-Z) based on their names.
- This sorting should be applied whenever the list is rendered or updated.

### 2. UI Impact
- This change should affect the rendering order of the character rows/cards in the main display area.
- No other visual changes are required, just the order.

## Success Criteria
- Characters appear in alphabetical order by name.
- New characters added dynamically also follow the sort order (or the list is re-sorted).
