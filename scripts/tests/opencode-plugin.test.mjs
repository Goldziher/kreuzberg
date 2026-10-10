import assert from "node:assert/strict";
import { mkdtemp, writeFile, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import plugin from "../../plugin/.opencode/plugins/xberg.js";

async function register() {
  const out = { tools: [], skills: [], commands: [], mcp: {} };
  assert.equal(typeof plugin.setup, "function");
  await plugin.setup({
    location: { directory: process.cwd() },
    tool: { transform: (fn) => fn({ add: (tool) => out.tools.push(tool) }) },
    skill: { transform: (fn) => fn({ get: () => undefined, add: (skill) => out.skills.push(skill) }) },
    command: { transform: (fn) => fn({ add: (command) => out.commands.push(command) }) },
    agent: { transform: (fn) => fn({ update: () => {} }) },
    mcp: {
      transform: (fn) =>
        fn({
          set: (name, config) => {
            out.mcp[name] = config;
          },
        }),
    },
  });
  return out;
}

test("registers bundled skills and MCP through the OpenCode v2 API", async () => {
  const out = await register();
  assert.equal(out.skills.length, 7);
  assert.equal(out.commands.length, 2);
  assert.equal(Object.keys(out.mcp).length, 1);
  assert.ok(out.mcp["xberg"].command[0].endsWith("scripts/mcp-launch.sh"));
});

test("keeps xberg arguments intact, returns output, and forwards cancellation", async () => {
  const out = await register();
  assert.equal(out.tools.length, 3);
  const tool = out.tools.find((item) => item.name === "xberg_extract");
  const directory = await mkdtemp(path.join(tmpdir(), "opencode-contract-"));
  const originalPath = process.env.PATH;
  try {
    const binary = path.join(directory, "xberg");
    await writeFile(
      binary,
      `#!${process.execPath}
if (process.argv.includes("wait")) setTimeout(() => {}, 10000);
else if (process.argv.includes("fail")) { process.stderr.write("failed input"); process.exit(3); }
else process.stdout.write(JSON.stringify(process.argv.slice(2)));
`,
    );
    await chmod(binary, 0o755);
    process.env.PATH = `${directory}${path.delimiter}${originalPath}`;
    const result = await tool.execute(
      { path: "input with spaces.pdf", config_json: "{}", content_format: "markdown" },
      {},
    );
    assert.deepEqual(JSON.parse(result.content), [
      "extract",
      "input with spaces.pdf",
      "--format",
      "json",
      "--content-format",
      "markdown",
      "--config-json",
      "{}",
    ]);
    assert.throws(() => tool.execute({ path: "file.pdf", config_json: "invalid" }, {}), /must be valid JSON/);
    const detected = await out.tools.find((item) => item.name === "xberg_detect").execute({ path: "file.pdf" }, {});
    assert.deepEqual(JSON.parse(detected.content), ["detect", "file.pdf", "--format", "json"]);
    const formats = await out.tools.find((item) => item.name === "xberg_formats").execute({}, {});
    assert.deepEqual(JSON.parse(formats.content), ["formats", "--format", "json"]);
    const failed = await tool.execute({ path: "fail" }, {});
    assert.equal(failed.metadata.exitCode, 3);
    assert.equal(failed.content, "stderr:\nfailed input");
    const controller = new AbortController();
    controller.abort();
    const waiting = { path: "wait" };
    await assert.rejects(tool.execute(waiting, { signal: controller.signal }), { name: "AbortError" });
  } finally {
    process.env.PATH = originalPath;
    await rm(directory, { recursive: true, force: true });
  }
});
