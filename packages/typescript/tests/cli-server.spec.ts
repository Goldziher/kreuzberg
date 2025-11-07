/**
 * Tests for CLI server commands (serve and mcp) via TypeScript CLI proxy.
 */

import { spawn, spawnSync } from "node:child_process";
import { unlinkSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("CLI Server Commands", () => {
	it("serve command help is accessible via TypeScript CLI proxy", () => {
		const result = spawnSync("npx", ["kreuzberg", "serve", "--help"], {
			encoding: "utf-8",
		});

		expect(result.status).toBe(0);
		expect(result.stdout).toContain("Start the API server");
		expect(result.stdout).toContain("--host");
		expect(result.stdout).toContain("--port");
		expect(result.stdout).toContain("--config");
	});

	it("mcp command help is accessible via TypeScript CLI proxy", () => {
		const result = spawnSync("npx", ["kreuzberg", "mcp", "--help"], {
			encoding: "utf-8",
		});

		expect(result.status).toBe(0);
		expect(result.stdout).toContain("Start the MCP (Model Context Protocol) server");
		expect(result.stdout).toContain("--config");
	});

	it("API server starts and responds to HTTP requests", async () => {
		const port = 18005;

		// Start server in background
		const process = spawn("npx", ["kreuzberg", "serve", "-H", "127.0.0.1", "-p", port.toString()], {
			stdio: ["ignore", "pipe", "pipe"],
		});

		try {
			// Wait for server to start
			await new Promise((resolve) => setTimeout(resolve, 5000));

			// Test health endpoint
			const healthResponse = await fetch(`http://127.0.0.1:${port}/health`);
			expect(healthResponse.status).toBe(200);

			const healthData = await healthResponse.json();
			expect(healthData.status).toBe("healthy");
			expect(healthData.version).toBeDefined();

			// Test info endpoint
			const infoResponse = await fetch(`http://127.0.0.1:${port}/info`);
			expect(infoResponse.status).toBe(200);

			const infoData = await infoResponse.json();
			expect(infoData.rust_backend).toBe(true);
		} finally {
			// Kill server
			process.kill("SIGTERM");
			await new Promise((resolve) => {
				process.on("exit", resolve);
				setTimeout(() => {
					process.kill("SIGKILL");
					resolve(null);
				}, 5000);
			});
		}
	}, 60000);

	it("server starts with custom config file", async () => {
		const port = 18006;
		const configPath = "test_server_config.toml";

		// Create test config
		writeFileSync(
			configPath,
			`
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"
`,
		);

		// Start server with config
		const process = spawn("npx", ["kreuzberg", "serve", "-H", "127.0.0.1", "-p", port.toString(), "-c", configPath], {
			stdio: ["ignore", "pipe", "pipe"],
		});

		try {
			// Wait for server to start
			await new Promise((resolve) => setTimeout(resolve, 5000));

			// Test health endpoint
			const response = await fetch(`http://127.0.0.1:${port}/health`);
			expect(response.status).toBe(200);
		} finally {
			// Kill server
			process.kill("SIGTERM");
			await new Promise((resolve) => {
				process.on("exit", resolve);
				setTimeout(() => {
					process.kill("SIGKILL");
					resolve(null);
				}, 5000);
			});

			// Cleanup
			try {
				unlinkSync(configPath);
			} catch {
				// Ignore cleanup errors
			}
		}
	}, 60000);

	it("server extract endpoint works", async () => {
		const port = 18007;

		// Start server
		const process = spawn("npx", ["kreuzberg", "serve", "-H", "127.0.0.1", "-p", port.toString()], {
			stdio: ["ignore", "pipe", "pipe"],
		});

		try {
			// Wait for server to start
			await new Promise((resolve) => setTimeout(resolve, 5000));

			// Create test file
			const testContent = "Hello, Kreuzberg API from TypeScript!";
			const formData = new FormData();
			const blob = new Blob([testContent], { type: "text/plain" });
			formData.append("files", blob, "test.txt");

			// Test extract endpoint
			const response = await fetch(`http://127.0.0.1:${port}/extract`, {
				method: "POST",
				body: formData,
			});

			expect(response.status).toBe(200);
			const results = await response.json();
			expect(Array.isArray(results)).toBe(true);
			expect(results).toHaveLength(1);
			expect(results[0].content).toContain(testContent);
		} finally {
			// Kill server
			process.kill("SIGTERM");
			await new Promise((resolve) => {
				process.on("exit", resolve);
				setTimeout(() => {
					process.kill("SIGKILL");
					resolve(null);
				}, 5000);
			});
		}
	}, 60000);
});
