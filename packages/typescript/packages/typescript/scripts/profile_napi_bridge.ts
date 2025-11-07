#!/usr/bin/env node

/**
 * Profile NAPI-RS bridge for memory leaks and overhead.
 */

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { performance } from "node:perf_hooks";
import { extractFileSync } from "../dist/index.js";

interface ProfileResult {
	label: string;
	iterations: number;
	totalDuration: number;
	avgTimeMs: number;
	startMemMb: number;
	endMemMb: number;
	peakMemMb: number;
	deltaMemMb: number;
	afterGcMemMb: number;
	leakMb: number;
}

function getMemoryMb(): number {
	const memUsage = process.memoryUsage();
	return memUsage.rss / 1024 / 1024;
}

function forceGc() {
	if (global.gc) {
		global.gc();
	}
}

function profileExtraction(
	func: (path: string) => any,
	filePath: string,
	iterations: number,
	label: string,
): ProfileResult {
	console.log(`\n${"=".repeat(70)}`);
	console.log(`Profiling: ${label}`);
	console.log(`File: ${basename(filePath)}`);
	console.log(`Iterations: ${iterations}`);
	console.log("=".repeat(70));

	// Warm up
	func(filePath);
	forceGc();

	// Start profiling
	const startMem = getMemoryMb();
	const startTime = performance.now();

	const memSamples: number[] = [];
	for (let i = 0; i < iterations; i++) {
		func(filePath);
		if (i % 10 === 0) {
			memSamples.push(getMemoryMb());
		}
	}

	const endTime = performance.now();
	const endMem = getMemoryMb();

	// Force GC
	forceGc();
	const gcMem = getMemoryMb();

	const duration = (endTime - startTime) / 1000;
	const avgTime = duration / iterations;

	const results: ProfileResult = {
		label,
		iterations,
		totalDuration: duration,
		avgTimeMs: avgTime * 1000,
		startMemMb: startMem,
		endMemMb: endMem,
		peakMemMb: Math.max(...memSamples, endMem),
		deltaMemMb: endMem - startMem,
		afterGcMemMb: gcMem,
		leakMb: gcMem - startMem,
	};

	console.log("\nResults:");
	console.log(`  Duration: ${duration.toFixed(3)}s (${(avgTime * 1000).toFixed(3)}ms/iter)`);
	console.log(`  Memory: ${startMem.toFixed(1)}MB → ${endMem.toFixed(1)}MB (Δ${results.deltaMemMb.toFixed(1)}MB)`);
	console.log(`  Peak: ${results.peakMemMb.toFixed(1)}MB`);
	console.log(`  After GC: ${gcMem.toFixed(1)}MB (leak: ${results.leakMb.toFixed(1)}MB)`);

	return results;
}

const testFiles = [
	"../../test_documents/documents/fake.docx",
	"../../test_documents/documents/lorem_ipsum.docx",
].filter((f) => existsSync(f));

if (testFiles.length === 0) {
	console.log("❌ No test files found!");
	process.exit(1);
}

console.log("NAPI-RS Bridge Profiling");
console.log("=".repeat(70));
console.log(`Found ${testFiles.length} test files\n`);

for (const testFile of testFiles) {
	const results = profileExtraction(extractFileSync, testFile, 50, "NAPI-RS Binding");
	const outputDir = "../../results/memory_profile";
	mkdirSync(outputDir, { recursive: true });
	const outputFile = join(outputDir, `napi_bridge_${basename(testFile, ".docx")}.json`);
	writeFileSync(outputFile, JSON.stringify(results, null, 2));
	console.log(`\n💾 Results saved to ${outputFile}`);
}
