import fs from "node:fs";
import path from "node:path";

const bindingPath = process.env.KREUZBERG_NODE_BINDING_PATH;
if (!bindingPath) {
	console.error("KREUZBERG_NODE_BINDING_PATH env var is required");
	process.exit(1);
}

const binding = require(bindingPath);
const fixture = path.resolve("./fixtures/report.txt");
if (!fs.existsSync(fixture)) {
	console.error(`Fixture not found: ${fixture}`);
	process.exit(1);
}

const result = binding.extractFileSync(fixture, null, null);
if (!result || typeof result.content !== "string" || !result.content.includes("smoke")) {
	console.error("Smoke test failed: snippet missing");
	process.exit(1);
}

console.log("[node smoke] extraction succeeded");
