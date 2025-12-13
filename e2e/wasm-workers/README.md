# Kreuzberg WASM Workers E2E Tests

End-to-end tests for @kreuzberg/wasm package in Cloudflare Workers environment using Miniflare.

## Prerequisites

- Node.js 18+
- pnpm
- @kreuzberg/wasm package built

## Setup

```bash
pnpm install
```

## Running Tests

```bash
# Run all tests
pnpm test

# Watch mode
pnpm test:watch

# With UI
pnpm test:ui
```

## Test Structure

- `tests/helpers.ts` - Test utilities with embedded fixtures
- `tests/*.spec.ts` - Category-specific tests

Tests use embedded fixtures (no file system in Workers). Only small fixtures (<500KB) are included.
