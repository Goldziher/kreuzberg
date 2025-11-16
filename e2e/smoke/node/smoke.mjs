import fs from "node:fs";
import path from "node:path";
import { extractFileSync } from "kreuzberg-node";

const fixture = path.resolve("./fixtures/report.txt");
if (!fs.existsSync(fixture)) {
	console.error(`Fixture not found: ${fixture}`);
	process.exit(1);
}

const result = extractFileSync(fixture, null, null);
if (!result || typeof result.content !== "string" || !result.content.includes("smoke")) {
	console.error("Smoke test failed: snippet missing");
	process.exit(1);
}

console.log("[node smoke] extraction succeeded");
