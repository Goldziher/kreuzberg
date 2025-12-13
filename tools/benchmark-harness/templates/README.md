# Benchmark Visualization Templates

This directory contains Jinja2 templates for generating benchmark HTML visualizations.

## Quick Start for Juniors

### Where to Find Things

- **Colors and theme**: `styles/variables.css.jinja`
- **Page layout**: `styles/layout.css.jinja`, `styles/components.css.jinja`
- **Chart titles and sections**: `charts/*.html.jinja`
- **Chart JavaScript code**: `charts/*_script.js.jinja`
- **Header and metadata**: `components/header.html.jinja`
- **Tab navigation**: `components/tabs.html.jinja`
- **"No Data" message**: `components/empty_state.html.jinja`
- **Main page structure**: `base.html.jinja`

### Directory Structure

```
templates/
├── README.md                    # This file
├── base.html.jinja             # Main HTML shell - includes all other templates
├── components/                  # Reusable UI components
│   ├── header.html.jinja       # Page header with title and metadata
│   ├── tabs.html.jinja         # Tab navigation buttons
│   ├── success_summary.html.jinja  # Success rate summary cards
│   └── empty_state.html.jinja  # "No Data Available" placeholder
├── charts/                      # Chart visualizations (HTML + JavaScript pairs)
│   ├── duration.html.jinja     # Duration chart canvas
│   ├── duration_script.js.jinja # Duration chart Chart.js code
│   ├── throughput.html.jinja   # Throughput chart canvas
│   ├── throughput_script.js.jinja
│   ├── memory.html.jinja       # Memory usage chart canvas
│   ├── memory_script.js.jinja
│   ├── filetype.html.jinja     # File type breakdown chart
│   ├── filetype_script.js.jinja
│   ├── success.html.jinja      # Success rate chart
│   └── success_script.js.jinja
└── styles/                      # CSS styling (inlined in HTML output)
    ├── variables.css.jinja     # CSS variables matching MkDocs theme
    ├── layout.css.jinja        # Page layout (containers, grids)
    ├── components.css.jinja    # Component styles (tabs, buttons, cards)
    └── charts.css.jinja        # Chart canvas styling
```

## Common Tasks

### Change Accent Color

The accent color (purple in light mode, cyan in dark mode) is defined in `styles/variables.css.jinja`:

```css
:root {
  --md-accent-fg-color: #da2ae0;  /* Change this for light mode */
}

@media (prefers-color-scheme: dark) {
  :root {
    --md-accent-fg-color: #58fbda;  /* Change this for dark mode */
  }
}
```

### Change Background Color

Edit `styles/variables.css.jinja`:

```css
:root {
  --md-default-bg-color: #ffffff;  /* Light mode background */
}

@media (prefers-color-scheme: dark) {
  :root {
    --md-default-bg-color: #23232c;  /* Dark mode background */
  }
}
```

### Change Font

Edit `styles/variables.css.jinja`:

```css
:root {
  --font-family: Inter, -apple-system, BlinkMacSystemFont, sans-serif;
  --font-family-headings: "Exo 2", var(--font-family);
}
```

### Update "No Data" Message

Edit `components/empty_state.html.jinja`:

```html
<h1>No Benchmark Data Available</h1>
<p>Your custom message here...</p>
```

### Test Your Changes

After editing templates, rebuild and test:

```bash
# From repository root
cd tools/benchmark-harness

# Build the harness
cargo build --release

# Generate test HTML (using example data from tests)
cargo run --release --example test_html_generation

# The output will be at /tmp/test-benchmark-viz/index.html
open /tmp/test-benchmark-viz/index.html
```

## How Templates Work

### Data Injection

Templates receive a `data` object with all benchmark results:

```jinja2
{# Access frameworks list #}
{{ data.frameworks|length }} frameworks tested

{# Access specific framework metrics #}
{{ data.framework_metrics['kreuzberg-native'].p95_duration_ms|round(2) }} ms

{# Check if data exists #}
{% if data.frameworks|length > 0 %}
  Show charts
{% else %}
  Show "No Data" message
{% endif %}
```

### Template Inclusion

Templates use `{% include %}` to compose the page:

```jinja2
{# In base.html.jinja #}
<style>
  {% include "styles/variables.css.jinja" %}
  {% include "styles/layout.css.jinja" %}
</style>

<body>
  {% include "components/header.html.jinja" %}
  {% include "charts/duration.html.jinja" %}
</body>
```

### CSS Variables

All colors come from CSS custom properties (variables):

```css
/* Define once */
:root {
  --accent-color: #da2ae0;
}

/* Use everywhere */
.tab-button.active {
  background: var(--accent-color);
  border-color: var(--accent-color);
}
```

Dark mode is automatic via `@media (prefers-color-scheme: dark)` - no JavaScript needed.

## MkDocs Theme Integration

This template system uses the **exact same CSS variable names** as the MkDocs Material theme:

- `--md-primary-fg-color`: Primary text/element color (#323040)
- `--md-accent-fg-color`: Accent color (purple/cyan)
- `--md-default-bg-color`: Background color (white/dark)
- `--md-default-fg-color-light`: Secondary text color

This ensures the benchmark visualization matches the documentation theme perfectly.

## Troubleshooting

### Templates not found error

Error: `Template not found: base.html.jinja`

Solution: Templates are embedded at compile time. After editing templates, you must rebuild:

```bash
cargo build --release -p benchmark-harness
```

### Styles not applying

1. Check CSS syntax in `styles/*.css.jinja` files
2. Verify CSS variables are defined in `styles/variables.css.jinja`
3. Rebuild and regenerate HTML

### Charts not showing

1. Check browser console for JavaScript errors
2. Verify Chart.js CDN is accessible (needs internet)
3. Check that chart data is present: View Page Source → search for `benchmarkData`

### Dark mode not working

Dark mode uses `@media (prefers-color-scheme: dark)`:

1. Test by toggling OS/browser dark mode setting
2. Or use browser DevTools: Toggle device toolbar → More options → Emulate CSS prefers-color-scheme
3. Verify dark mode variables are defined in `styles/variables.css.jinja`

## Need Help?

- View generated HTML source to see how templates were rendered
- Check `../src/html.rs` for template loading code
- Ask senior developers on the team
- Read MiniJinja documentation: https://docs.rs/minijinja/
