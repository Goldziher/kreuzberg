#!/usr/bin/env node

/**
 * Proxy entry point that forwards to the Rust-based Kreuzberg CLI.
 *
 * This keeps `npx kreuzberg` working without shipping an additional TypeScript CLI implementation.
 */

import { spawnSync } from "node:child_process";
import which from "which";

function main(argv: string[]): number {
	const args = argv.slice(2);

	let cliPath: string;
	try {
		cliPath = which.sync("kreuzberg-cli");
	} catch {
		console.error(
			"The embedded Kreuzberg CLI binary could not be located. " +
				"This indicates a packaging issue; please open an issue at " +
				"https://github.com/Goldziher/kreuzberg/issues so we can investigate.",
		);
		return 1;
	}

	const result = spawnSync(cliPath, args, {
		stdio: "inherit",
		shell: false,
	});

	if (result.error) {
		console.error(`Failed to execute kreuzberg-cli: ${result.error.message}`);
		return 1;
	}

	return result.status ?? 1;
}

if (require.main === module) {
	process.exit(main(process.argv));
}

export { main };
