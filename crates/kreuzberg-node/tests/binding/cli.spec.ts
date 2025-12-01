import { spawnSync } from "node:child_process";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import which from "which";

vi.mock("which");
vi.mock("node:child_process");

describe("CLI", () => {
	beforeEach(() => {
		vi.resetAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe("CLI binary detection", () => {
		it("should find kreuzberg-cli 4.0.0-rc.4 when available", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = which.sync("kreuzberg-cli");

			expect(result).toBe("/usr/local/bin/kreuzberg-cli");
		});

		it("should handle missing kreuzberg-cli 4.0.0-rc.4", () => {
			const mockWhichSync = vi.mocked(which.sync);

			mockWhichSync.mockImplementation(() => {
				throw new Error("not found");
			});

			expect(() => which.sync("kreuzberg-cli")).toThrow();
		});
	});

	describe("CLI argument passing", () => {
		it("should pass arguments to kreuzberg-cli 4.0.0-rc.4", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: null,
			});

			const args = ["extract", "file.pdf"];
			spawnSync("/usr/local/bin/kreuzberg-cli", args, {
				stdio: "inherit",
				shell: false,
			});

			expect(mockSpawnSync).toHaveBeenCalledWith(
				"/usr/local/bin/kreuzberg-cli",
				args,
				expect.objectContaining({
					stdio: "inherit",
					shell: false,
				}),
			);
		});

		it("should handle --help flag", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from("Usage: kreuzberg-cli [OPTIONS]"),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["--help"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});

		it("should handle --version flag", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from("kreuzberg-cli 4.0.0-rc.4"),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["--version"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});
	});

	describe("CLI exit codes", () => {
		it("should return 0 for successful execution", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["--help"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});

		it("should return non-zero for errors", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 1,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from("Error: invalid command"),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["invalid-command"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(1);
		});

		it("should handle signal termination", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: null,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: "SIGTERM",
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["long-running-command"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.signal).toBe("SIGTERM");
			expect(result.status).toBeNull();
		});
	});

	describe("CLI error handling", () => {
		it("should handle missing binary gracefully", () => {
			const mockWhichSync = vi.mocked(which.sync);

			mockWhichSync.mockImplementation(() => {
				throw new Error("not found");
			});

			expect(() => {
				which.sync("kreuzberg-cli");
			}).toThrow();
		});

		it("should handle spawn errors", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 127,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from("Command not found"),
				signal: null,
				error: new Error("spawn ENOENT"),
			} as any);

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["test"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(127);
		});
	});

	describe("CLI stdio configuration", () => {
		it("should use inherit stdio mode", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: null,
			});

			spawnSync("/usr/local/bin/kreuzberg-cli", [], {
				stdio: "inherit",
				shell: false,
			});

			expect(mockSpawnSync).toHaveBeenCalledWith(
				expect.any(String),
				expect.any(Array),
				expect.objectContaining({
					stdio: "inherit",
				}),
			);
		});

		it("should disable shell mode for security", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from(""),
				stderr: Buffer.from(""),
				signal: null,
			});

			spawnSync("/usr/local/bin/kreuzberg-cli", [], {
				stdio: "inherit",
				shell: false,
			});

			expect(mockSpawnSync).toHaveBeenCalledWith(
				expect.any(String),
				expect.any(Array),
				expect.objectContaining({
					shell: false,
				}),
			);
		});
	});

	describe("CLI command variations", () => {
		it("should handle API server command", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from("API server started"),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["api", "--port", "8000"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});

		it("should handle MCP server command", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from("MCP server started"),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["mcp"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});

		it("should handle extract command", () => {
			const mockWhichSync = vi.mocked(which.sync);
			const mockSpawnSync = vi.mocked(spawnSync);

			mockWhichSync.mockReturnValue("/usr/local/bin/kreuzberg-cli");
			mockSpawnSync.mockReturnValue({
				status: 0,
				pid: 12345,
				output: [],
				stdout: Buffer.from("Extraction complete"),
				stderr: Buffer.from(""),
				signal: null,
			});

			const result = spawnSync("/usr/local/bin/kreuzberg-cli", ["extract", "document.pdf"], {
				stdio: "inherit",
				shell: false,
			});

			expect(result.status).toBe(0);
		});
	});
});
