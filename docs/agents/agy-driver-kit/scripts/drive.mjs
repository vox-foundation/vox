// Claude Code's driver for one GSD plan task via headless agy (vox). Usage:
//   node drive.mjs <plan-id e.g. 05-01> <task-number> [--model m] [--note "extra guidance"]
// Runs agy, saves the JSON log under .agents/runs/, prints the summary, and lists changed paths
// against the task's <files>. Verification and commit are done by Claude Code afterwards.
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
const sh = (c, a, o = {}) => spawnSync(c, a, { encoding: "utf8", maxBuffer: 256 << 20, ...o });
// plan: a Phase 5 id like 05-03, or a repo-relative plan path ending in .md
const [plan, task] = process.argv.slice(2);
const opt = (k, d) => (process.argv.includes(k) ? process.argv[process.argv.indexOf(k) + 1] : d);
const model = opt("--model", "gemini-3.8-flash-high");
const note = opt("--note", "");
const root = "/Users/brbrainerd/dev/vox";
const planPath = plan.endsWith(".md") ? plan : `.planning/phases/05-multi-agent-coordination-trust-hardening/${plan}-PLAN.md`;
const text = readFileSync(`${root}/${planPath}`, "utf8");
let files;
if (text.includes("<task")) {
  const block = text.split(/<task\b/).slice(1).find((b) => new RegExp(`<name>Task ${task}:`).test(b)) ?? "";
  files = (block.match(/<files>([\s\S]*?)<\/files>/)?.[1] ?? "").split(",").map((s) => s.trim().replace(/\s*\(.*\)$/, "")).filter(Boolean);
} else {
  // writing-plans format: "### Task N: ..." section with a "**Files:**" list of `path[:lines]`.
  const section = text.split(/^### Task /m).find((s) => s.startsWith(`${task}:`)) ?? "";
  const filesBlock = section.split("**Files:**")[1]?.split(/\n\*\*|\n- \[ \]/)[0] ?? "";
  files = [...filesBlock.matchAll(/`([^`\s]+)`/g)].map((m) => m[1].replace(/:[\d~-]+(,[\d~-]+)*$/, "")).filter((p) => p.includes("/"));
}
const before = new Set(sh("git", ["status", "--porcelain", "-uall"], { cwd: root }).stdout.split("\n").filter(Boolean));
const head = sh("git", ["rev-parse", "HEAD"], { cwd: root }).stdout.trim();
const prompt = `/drive-task ${planPath} ${task}${note ? `\n\nDriver note from Claude Code: ${note}` : ""}`;
const t0 = Date.now();
const res = sh("agy", ["-p", prompt, "--model", model, "--dangerously-skip-permissions", "--output-format", "json", "--print-timeout", "60m"], { cwd: root });
mkdirSync(`${root}/.agents/runs`, { recursive: true });
const log = `${root}/.agents/runs/${plan}-t${task}-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
writeFileSync(log, res.stdout || res.stderr || "");
let out = {};
try { out = JSON.parse(res.stdout.slice(res.stdout.indexOf("{"))); } catch {}
const resp = String(out.response ?? "");
const after = sh("git", ["status", "--porcelain", "-uall"], { cwd: root }).stdout.split("\n").filter(Boolean);
const changed = after.filter((l) => !before.has(l)).map((l) => l.slice(3));
const outOfScope = changed.filter((p) => !files.some((f) => p === f || p.startsWith(f.endsWith("/") ? f : `${f}/`)) && !p.startsWith("target/"));
console.log(JSON.stringify({
  plan, task, model, status: out.status, exit: res.status,
  minutes: +((Date.now() - t0) / 60000).toFixed(1), tokens: out.usage?.total_tokens,
  headMoved: sh("git", ["rev-parse", "HEAD"], { cwd: root }).stdout.trim() !== head,
  last: resp.trim().split("\n").at(-1), changed, outOfScope, log,
  // Big numbers here on a file the task only appends to mean a rewrite, not an edit.
  numstat: sh("git", ["diff", "--numstat", "--", ...changed], { cwd: root }).stdout.trim().split("\n").filter((l) => { const [a, d] = l.split("\t").map(Number); return a + d > 400; }),
}, null, 1));
console.log("---- response tail ----\n" + resp.slice(-3000));
