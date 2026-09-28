// Antigravity PreToolUse hook for vox (.agents/hooks.json runs `node scripts/agent-guard.mjs` with cwd = .agents/).
// Adapted from ~/dev/containment/.agents/scripts/agent-guard.mjs. Reads hook JSON on stdin, prints
// {"decision","reason"}. vox's working tree is shared with other sessions, so anything touching the git
// index/history, formatting the whole tree, or changing dependencies is refused mechanically.
// Self-check: node .agents/scripts/agent-guard.mjs --self-test
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

const INERT = /^(git\s+(log|diff|status|show|grep|ls-files|rev-parse|blame)\b|(\/usr\/bin\/env\s+)?(grep|rg)\b|echo\b|test\b|printf\b)/;
const TEMP = /^(\/tmp\/|\/private\/tmp\/|\$TMPDIR|"\$TMPDIR|\$\{TMPDIR|target\/)/;

const COMMAND_RULES = [
  [/^git\s+push\b/, "git push is owner-only"],
  [/^git\s+(reset|clean|rebase|stash|checkout|restore|switch|merge|cherry-pick|revert|tag|worktree|am|apply|branch\s+-[dD])\b/, "the working tree and index are shared with other sessions; Claude Code owns git state"],
  [/^git\s+(add|commit|rm|mv)\b/, "Claude Code stages and commits after verifying; the agent never touches the index"],
  [/^sudo\b/, "no sudo"],
  [/^(ssh|scp|rsync|curl|wget)\b/, "no network or remote hosts from the agent"],
  [/^node\s+(-e|--eval|-p|--print)\b|^python3?\s+-c\b/, "no inline code; write a file"],
  [/^(npm|pnpm|yarn)\s+((--dir|-C|--prefix|--filter)\s+\S+\s+)*(i|install|add|update|up|remove|rm)\b/, "no dependency changes"],
  [/^cargo\s+(add|update|remove|rm|install)\b/, "no dependency changes"],
  [/^cargo\s+fmt\b|scripts\/fmt\.vox/, "formatting the crate/tree also rewrites other sessions' files; use rustfmt --edition 2024 <your files>"],
  [/(\.cargo\/bin\/cargo|rustup\s+run)\b/, "call plain `cargo` so the build broker queues it"],
  [/^(vox\s+ci\s+pre-push|cargo\s+run\b.*\bci\s+pre-push)\b/, "the full gate is run by Claude Code, not the agent"],
];

const ENV_TOKEN = /(^|\/)\.env(\.[\w-]+)?$/;
const namesEnv = (s) => s.split(/[\s=<>|&;"'`()]+/).some((t) => ENV_TOKEN.test(t) && !t.endsWith(".env.example"));

function checkSegment(seg) {
  // Strip wrappers that run another command (`timeout 900s git add`, `nice -n 5 …`, `env X=1 …`, `command …`),
  // otherwise every ^-anchored rule below is bypassed by the `timeout` prefix rule 9 tells the agent to use.
  let s = seg.trim();
  for (let prev = ""; prev !== s; ) {
    prev = s;
    s = s
      .replace(/^(env\s+)?(\w+=\S+\s+)+/, "")
      .replace(/^env\s+/, "")
      .replace(/^(timeout|gtimeout)\s+(-[ks]\s+\S+\s+|-\S+\s+)*\S+\s+/, "")
      .replace(/^nice\s+(-n\s+\S+\s+|-\d+\s+)?/, "")
      .replace(/^(command|exec|nohup|time)\s+/, "")
      .trim();
  }
  s = s.replace(/^cd\s+\S+\s*$/, "");
  if (!s) return null;
  if (namesEnv(s)) return ".env is off limits";
  if (INERT.test(s) && !/\$\(|`/.test(s)) return null;
  const rm = s.match(/^rm\s+(-[a-zA-Z]*[rR][a-zA-Z]*)\s+(.+)$/);
  if (rm && !rm[2].split(/\s+/).every((p) => TEMP.test(p))) return "recursive rm outside a temp dir";
  for (const [re, why] of COMMAND_RULES) if (re.test(s)) return why;
  return null;
}

// Paths only the owner (or Claude Code with the owner's say-so) may write.
const PROTECTED = [
  [/(^|[\\/"])\.env(?!\.example)(\.[\w-]+)?$/, ".env is off limits"],
  [/(^|[\\/"])\.git[\\/]/, "never write inside .git/"],
  [/(^|[\\/"])\.agents[\\/]/, ".agents/ (rules, skills, this guard) is owner-edited only"],
  [/(^|[\\/"])\.planning[\\/]/, ".planning/ is maintained by Claude Code"],
  [/(^|[\\/])(docs[\\/]src[\\/])?archive[\\/]/, "archive/ is tombstoned"],
  [/contracts[\\/]ci[\\/](crate-edges\.allow|fan-in-snapshot)\.v1\.json$/, "crate-edge exceptions and fan-in budgets are user-authorized only"],
  [/docs[\\/]src[\\/]architecture[\\/]layers\.toml$/, "layer assignments and budgets are user-authorized only"],
  [/(^|[\\/])Cargo\.lock$/, "Cargo.lock is shared; dependency changes are out of scope"],
];

export const isDriven = () => existsSync(fileURLToPath(new URL("../.driven", import.meta.url)));

export function decide(input) {
  const name = input?.toolCall?.name ?? "";
  const args = input?.toolCall?.args ?? {};
  if (name === "run_command") {
    for (const seg of String(args.CommandLine ?? "").split(/&&|\|\||;|\||\n|\$\(|`/)) {
      const why = checkSegment(seg);
      if (why) return { decision: "deny", reason: `agent-guard: ${why}` };
    }
    return { decision: "allow", reason: "agent-guard: ok" };
  }
  if (/write|replace|edit|delete|remove|move|rename|create/i.test(name)) {
    const text = String(args.TargetFile ?? args.AbsolutePath ?? args.path ?? args.file ?? "");
    for (const [re, why] of PROTECTED) if (re.test(text)) return { decision: "deny", reason: `agent-guard: ${why}` };
  }
  return { decision: "allow", reason: "agent-guard: ok" };
}

function selfTest() {
  const run = (c) => decide({ toolCall: { name: "run_command", args: { CommandLine: c } } }).decision;
  const file = (n, p) => decide({ toolCall: { name: n, args: { TargetFile: p } } }).decision;
  const cases = [
    [run("cargo test -p vox-orchestrator tool_receipt 2>&1 | tail -20"), "allow"],
    [run("cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings"), "allow"],
    [run("rustfmt --edition 2024 crates/vox-orchestrator/src/tool_receipt.rs"), "allow"],
    [run("pnpm --dir crates/vox-gui/ui typecheck"), "allow"],
    [run("pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-receipts.spec.ts"), "allow"],
    [run("git status --short && git diff --stat"), "allow"],
    [run("cd /Users/brbrainerd/dev/vox && cargo check -p vox-crypto"), "allow"],
    [run("rm -rf target/agent-scratch /tmp/x"), "allow"],
    [run("RUST_LOG=debug cargo test -p vox-gui --bin vox-gui"), "allow"],
    [run("git push origin main"), "deny"],
    [run("git add -A"), "deny"],
    [run("cargo test && git commit -am x"), "deny"],
    [run("git stash"), "deny"],
    [run("git restore --staged Cargo.lock"), "deny"],
    [run("git checkout -- crates/"), "deny"],
    [run("git reset HEAD~1"), "deny"],
    [run("cargo fmt --all"), "deny"],
    [run("cargo fmt -p vox-orchestrator"), "deny"],
    [run("vox run scripts/fmt.vox"), "deny"],
    [run("cargo add blake3"), "deny"],
    [run("pnpm --dir crates/vox-gui/ui add left-pad"), "deny"],
    [run("~/.cargo/bin/cargo build"), "deny"],
    [run("vox ci pre-push --complete"), "deny"],
    [run("rm -rf crates/vox-orchestrator"), "deny"],
    [run("cat .env"), "deny"],
    [run("echo $(git push)"), "deny"],
    [run("node -e 'console.log(1)'"), "deny"],
    [run("timeout 1500s cargo run -q -p vox-cli -- ci pre-push"), "deny"],
    [run("timeout 30s git add -A"), "deny"],
    [run("timeout -k 5 60s git commit -m x"), "deny"],
    [run("cd /r && timeout 900s cargo fmt --all"), "deny"],
    [run("env FOO=1 timeout 10s git push"), "deny"],
    [run("nice -n 5 git stash"), "deny"],
    [run("timeout 1500s cargo test -p vox-orchestrator --lib models::"), "allow"],
    [run("timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run"), "allow"],
    [file("write_to_file", "/r/crates/vox-orchestrator/src/tool_receipt.rs"), "allow"],
    [file("replace_file_content", "/r/crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx"), "allow"],
    [file("write_to_file", "/r/.env"), "deny"],
    [file("write_to_file", "/r/.env.example"), "allow"],
    [file("replace_file_content", "/r/.agents/scripts/agent-guard.mjs"), "deny"],
    [file("replace_file_content", "/r/.planning/STATE.md"), "deny"],
    [file("replace_file_content", "/r/contracts/ci/crate-edges.allow.v1.json"), "deny"],
    [file("replace_file_content", "/r/docs/src/architecture/layers.toml"), "deny"],
    [file("replace_file_content", "/r/Cargo.lock"), "deny"],
    [file("write_to_file", "/r/docs/src/archive/x.md"), "deny"],
  ];
  const bad = cases.map(([got, want], i) => (got === want ? null : `case ${i}: got ${got}, want ${want}`)).filter(Boolean);
  if (bad.length) { console.error(bad.join("\n")); process.exit(1); }
  console.log(`agent-guard self-test: ${cases.length} cases ok`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.includes("--self-test")) selfTest();
  else {
    let raw = "";
    for await (const chunk of process.stdin) raw += chunk;
    let out;
    try { out = decide(JSON.parse(raw || "{}")); } catch { out = { decision: "deny", reason: "agent-guard: unreadable hook input" }; }
    process.stdout.write(JSON.stringify(out));
  }
}
