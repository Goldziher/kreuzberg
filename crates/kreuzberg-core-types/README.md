# @kreuzberg/core

Kreuzberg shared core types and utilities for use across all language bindings and packages.

## Features

- **Shared Types**: Common type definitions used across the Kreuzberg ecosystem
- **Utilities**: Reusable utility functions and helpers
- **Constants**: Common constants and configuration values

## Installation

```bash
npm install @kreuzberg/core
```

## Usage

### Import from root

```typescript
import { /* types */ } from "@kreuzberg/core";
```

### Import from submodules

```typescript
// Types only
import { /* types */ } from "@kreuzberg/core/types";

// Utilities only
import { /* utilities */ } from "@kreuzberg/core/utils";
```

## Development

### Build

```bash
npm run build
```

### Type checking

```bash
npm run typecheck
```

### Testing

```bash
npm test
```

### Linting

```bash
npm run lint
npm run lint:fix
```

## License

MIT
