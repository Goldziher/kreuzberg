# Node Smoke App

Loads the built `kreuzberg` binding and extracts text from a tiny fixture.

Set `KREUZBERG_NODE_BINDING_PATH` to the `.node` file you want to verify, then
run:

```bash
cd e2e/smoke/node
pnpm install
KREUZBERG_NODE_BINDING_PATH=/path/to/kreuzberg.linux-x64-gnu.node pnpm run check
```
