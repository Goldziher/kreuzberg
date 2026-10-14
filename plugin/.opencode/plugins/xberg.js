// AI-RULEZ :: GENERATED FILE — DO NOT EDIT
// Content-Hash: blake3:3fe1a33d721dfa8b49134cbe88659999cf9e481c558eb8e4b5ac81bc672c88db
// Source-Hash: blake3:e3f971402b546016b7336a1a078432a17b4e15efad115f1a5ead8136fe2182c6
// Schema-Version: v1

import { spawn } from "node:child_process";
import { registerBundledContent } from "../ai-rulez-content.js";

function hasValue(value) {
  return value !== undefined && value !== null && value !== "";
}

function pushOption(args, name, value) {
  if (hasValue(value)) {
    args.push(name, String(value));
  }
}

function validateJson(value, name) {
  if (!hasValue(value)) {
    return;
  }

  try {
    JSON.parse(value);
  } catch (error) {
    throw new Error(`${name} must be valid JSON: ${error.message}`, { cause: error });
  }
}

function runCli(args, context, directory) {
  return new Promise((resolve, reject) => {
    const child = spawn("xberg", args, {
      cwd: directory,
      env: process.env,
      signal: context?.signal,
      stdio: ["ignore", "pipe", "pipe"],
    });

    const stdout = [];
    const stderr = [];

    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => {
      if (error.code === "ENOENT") {
        resolve({
          content:
            "Install the xberg CLI with `brew install xberg-io/tap/xberg`, or run it via `npx -y @xberg-io/xberg-cli` / `uvx --from xberg-cli xberg`.",
          metadata: { exitCode: 127, command: "xberg", subcommand: args[0] },
        });
        return;
      }
      reject(error);
    });
    child.on("close", (exitCode, signal) => {
      const stdoutText = Buffer.concat(stdout).toString("utf8").trim();
      const stderrText = Buffer.concat(stderr).toString("utf8").trim();
      const output = [stdoutText, stderrText && `stderr:\n${stderrText}`].filter(Boolean).join("\n\n");

      resolve({
        content: output || "(no output)",
        metadata: {
          exitCode,
          signal,
          command: "xberg",
          subcommand: args[0],
        },
      });
    });
  });
}

const wireFormat = { type: "string", enum: ["text", "json", "toon"], default: "json" };
const contentFormat = { type: "string", enum: ["plain", "markdown", "djot", "html", "json"] };

function extractTool(directory) {
  return {
    name: "xberg_extract",
    description: "Extract text, tables, metadata, and images from a local document with the xberg CLI.",
    input: {
      type: "object",
      properties: {
        path: { type: "string", minLength: 1 },
        format: wireFormat,
        content_format: contentFormat,
        mime_type: { type: "string", minLength: 1 },
        config_json: { type: "string", minLength: 2 },
      },
      required: ["path"],
      additionalProperties: false,
    },
    execute(args, context) {
      validateJson(args.config_json, "config_json");
      const cliArgs = ["extract", args.path, "--format", args.format ?? "json"];
      pushOption(cliArgs, "--content-format", args.content_format);
      pushOption(cliArgs, "--mime-type", args.mime_type);
      pushOption(cliArgs, "--config-json", args.config_json);
      return runCli(cliArgs, context, directory);
    },
  };
}

function detectTool(directory) {
  return {
    name: "xberg_detect",
    description: "Detect the MIME type for a local file with the xberg CLI.",
    input: {
      type: "object",
      properties: { path: { type: "string", minLength: 1 }, format: wireFormat },
      required: ["path"],
      additionalProperties: false,
    },
    execute(args, context) {
      return runCli(["detect", args.path, "--format", args.format ?? "json"], context, directory);
    },
  };
}

function formatsTool(directory) {
  return {
    name: "xberg_formats",
    description: "List document formats supported by the xberg CLI.",
    input: { type: "object", properties: { format: wireFormat }, additionalProperties: false },
    execute(args, context) {
      return runCli(["formats", "--format", args.format ?? "json"], context, directory);
    },
  };
}

export default {
  id: "xberg",
  async setup(ctx) {
    await registerBundledContent(ctx);
    await ctx.tool.transform((editor) => {
      for (const tool of [
        extractTool(ctx.location.directory),
        detectTool(ctx.location.directory),
        formatsTool(ctx.location.directory),
      ]) {
        editor.add(tool);
      }
    });
  },
};
