const fs = require("node:fs");
const path = require("node:path");

const spec = process.env.KREUZBERG_NODE_SPEC || "";
if (!spec) {
	console.error("KREUZBERG_NODE_SPEC missing");
	process.exit(1);
}

const pkgPath = path.resolve("package.json");
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));

pkg.dependencies = pkg.dependencies || {};
pkg.dependencies["kreuzberg-node"] = spec.replace(/\\/g, "/");

fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
