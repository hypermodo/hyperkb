/**
 * HyperKB Plugin for OpenCode (Phase 12: Generic Lifecycle Hook)
 *
 * Provides turn-start context injection (<35 lines) and automated pre-tool risk guards
 * using the lightweight universal `hyperkb hook` CLI engine.
 *
 * Placement options:
 * - Local repo: .opencode/plugins/hyperkb.js (or .ts)
 * - Global: ~/.config/opencode/plugins/hyperkb.js
 */

import { execFileSync } from "node:child_process";

function runHyperkbHook(args: string[]): { stdout: string; error?: Error; status: number } {
  try {
    const stdout = execFileSync("hyperkb", ["hook", ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { stdout, status: 0 };
  } catch (err: any) {
    return {
      stdout: err.stdout?.toString() || "",
      error: err,
      status: err.status || 1,
    };
  }
}

export default async function hyperkbPlugin(ctx: any) {
  return {
    // 1. Session Start Context Injection
    event: async ({ event }: { event: { type: string; [key: string]: any } }) => {
      if (event.type === "session.created" || event.type === "session.started") {
        const res = runHyperkbHook(["session-start"]);
        if (res.stdout.trim()) {
          console.log(`\n${res.stdout.trim()}\n`);
        }
      }
    },

    // 2. Pre-Tool Execution Risk Interception
    "tool.execute.before": async (
      { tool }: { tool: string },
      { args }: { args: { path?: string; filePath?: string; file?: string; [key: string]: any } }
    ) => {
      // Intercept file-modifying tools
      if (["write", "edit", "patch"].includes(tool)) {
        const targetPath = args.path || args.filePath || args.file;
        if (targetPath) {
          const res = runHyperkbHook(["pre-tool-call", "--tool", tool, "--path", targetPath, "--strict"]);
          if (res.status !== 0) {
            throw new Error(
              `[HyperKB Risk Guard] Tool '${tool}' on '${targetPath}' was blocked by HyperKB:\n${res.error?.message || ""}`
            );
          }
        }
      }
    },
  };
}
