//! Detector `vox/fs/hardcoded-abs-dir`: flags machine-local workstation paths and unanchored
//! absolute OS root directories across Rust, TypeScript/JS, Python, GDScript, and Vox.

use crate::diagnostics::catalog::FS_HARDCODED_ABS_DIR;
use crate::rules::{
    DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile,
    rust_byte_is_cfg_test, rust_byte_is_comment,
};
use regex::Regex;
use std::sync::LazyLock;

/// Synthetic fixture usernames permitted **only** inside test files / `#[cfg(test)]` blocks.
pub(crate) const SYNTHETIC_TEST_USERS: &[&str] = &[
    "alice",
    "bob",
    "ada",
    "user",
    "u",
    "me",
    "x",
    "someone",
    "secretuser",
    "dev",
    "vox",
    "owner",
    "default",
    "public",
    "shared",
    "test",
    "runner",
    "ubuntu",
    "a@b",
];

// Pattern 1: POSIX / MSYS / file-URI user home directory:
//   /Users/<user>/..., file:///Users/<user>/..., /home/<user>/..., file:///home/<user>/...,
//   /c/Users/<user>/..., C:/Users/<user>/...
// Matches a concrete username segment `[A-Za-z0-9._@-]+` followed by `/`, `\`, or string end quote.
static MACHINE_USER_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:file://)?(?:/[A-Za-z]:|[A-Za-z]:)?/(?:Users|home)/([A-Za-z0-9._@-]+)(?:/|\\|["'`]|$)"#,
    )
    .expect("valid machine user regex")
});

// Pattern 2: Windows backslash user home directory (C:\Users\<user>\... or \\?\C:\Users\<user>\...)
static WIN_USER_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:\\\\\?\\)?[A-Za-z]:\\+Users\\+([A-Za-z0-9._@-]+)(?:\\|/|["'`]|$)"#)
        .expect("valid win user regex")
});

// Pattern 3: Ephemeral IDE agent brain / macOS private temp dirs inside quoted strings.
// Split `.gemini/antigravity` to avoid self-matching outside suppressions.
static EPHEMERAL_DIR_RE: LazyLock<Regex> = LazyLock::new(|| {
    let brain = format!(r"\.gemini/{}/brain/", "antigravity");
    // toestub-ignore(vox/fs/hardcoded-abs-dir): detector pattern
    let pattern = format!(
        r#"["'`](?:[^"'`]*?(?:{}|/private/tmp/|/private/var/folders/)[^"'`]*)["'`]"#, // drift-allow(hardcoded-abs-dir): detector pattern
        regex::escape(&brain)
    );
    Regex::new(&pattern).expect("valid ephemeral dir regex")
});

// Pattern 4: Tier-2 general OS root directory in quoted string (non-test production code).
static ROOT_ABS_DIR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"["'`](?:(?:/(?:tmp|var|opt|etc|usr|proc|sys)/[^"'`\s]*)|(?:[A-Za-z]:\\+[^nrt"'`\s][^"'`\s]*))["'`]"#,
    )
    .expect("valid root abs dir regex")
});

/// Detects hardcoded machine-local home directories, ephemeral brain paths, and unanchored OS root paths.
pub struct HardcodedAbsDirDetector {
    langs: Vec<Language>,
}

impl Default for HardcodedAbsDirDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl HardcodedAbsDirDetector {
    pub fn new() -> Self {
        Self {
            langs: vec![
                Language::Rust,
                Language::TypeScript,
                Language::Python,
                Language::GDScript,
                Language::Vox,
            ],
        }
    }

    pub(crate) fn is_test_path(path: &str) -> bool {
        let p = path.replace('\\', "/");
        p.contains("/tests/")
            || p.contains("/e2e/")
            || p.ends_with("/tests.rs")
            || p.ends_with("_test.rs")
            || p.ends_with("_tests.rs")
            || p.ends_with(".test.ts")
            || p.ends_with(".test.tsx")
            || p.ends_with(".spec.ts")
            || p.ends_with(".spec.tsx")
            || p.ends_with("_test.py")
            || p.starts_with("tests/")
            || p.starts_with("e2e/")
    }

    #[inline]
    pub(crate) fn is_synthetic_test_user(user: &str) -> bool {
        let lower = user.to_ascii_lowercase();
        SYNTHETIC_TEST_USERS.contains(&lower.as_str())
    }

    #[inline]
    fn is_placeholder_ellipsis(user: &str) -> bool {
        !user.is_empty() && user.bytes().all(|b| b == b'.')
    }
}

/// Inspect a single source line or unquoted string literal for hardcoded machine-local home
/// directories (`/Users/<user>`, `/home/<user>`, `C:\Users\<user>`, `C:/Users/<user>`) or
/// ephemeral IDE brain / `/private/tmp` paths.
///
/// Used by both `HardcodedAbsDirDetector` (line-level) and `vox-drift-check`'s
/// `HardcodedAbsDirRule` (AST string-literal level) so rule semantics stay single-sourced.
pub fn find_machine_local_dir(
    text: &str,
    require_quotes_for_ephemeral: bool,
    is_test_ctx: bool,
) -> Option<(usize, String, &'static str)> {
    let maybe_ephemeral = text.contains(".gemini/")
        || text.contains("/private/tmp/") // toestub-ignore(vox/fs/hardcoded-abs-dir): detector pattern // drift-allow(hardcoded-abs-dir): detector pattern
        || text.contains("/private/var/folders/"); // toestub-ignore(vox/fs/hardcoded-abs-dir): detector pattern // drift-allow(hardcoded-abs-dir): detector pattern
    if maybe_ephemeral {
        if require_quotes_for_ephemeral {
            if let Some(m) = EPHEMERAL_DIR_RE.find(text) {
                return Some((
                    m.start(),
                    m.as_str().trim_matches(['"', '\'', '`']).to_string(),
                    "Hardcoded ephemeral IDE brain or /private/tmp directory path",
                ));
            }
        } else {
            let brain_marker = format!(".gemini/{}/brain/", "antigravity");
            let var_folders_marker = format!("/{}/var/folders/", "private");
            if let Some(pos) = text
                .find(&brain_marker)
                .or_else(|| text.find("/private/tmp/")) // toestub-ignore(vox/fs/hardcoded-abs-dir): detector pattern // drift-allow(hardcoded-abs-dir): detector pattern
                .or_else(|| text.find(&var_folders_marker))
            {
                return Some((
                    pos,
                    text.to_string(),
                    "Hardcoded ephemeral IDE brain or /private/tmp directory path",
                ));
            }
        }
    }

    if (text.contains("/Users/") || text.contains("/home/")) // vox-arch-check: allow abs-path
        && let Some(caps) = MACHINE_USER_PATH_RE.captures(text)
        && let Some(user_m) = caps.get(1)
    {
        let user = user_m.as_str();
        if !HardcodedAbsDirDetector::is_placeholder_ellipsis(user)
            && !(is_test_ctx && HardcodedAbsDirDetector::is_synthetic_test_user(user))
        {
            let full = caps.get(0).unwrap();
            return Some((
                full.start(),
                full.as_str().trim_end_matches(['"', '\'', '`']).to_string(),
                "Hardcoded machine-local home directory",
            ));
        }
    }

    if text.contains(":\\Users\\")
        && let Some(caps) = WIN_USER_PATH_RE.captures(text)
        && let Some(user_m) = caps.get(1)
    {
        let user = user_m.as_str();
        if !HardcodedAbsDirDetector::is_placeholder_ellipsis(user)
            && !(is_test_ctx && HardcodedAbsDirDetector::is_synthetic_test_user(user))
        {
            let full = caps.get(0).unwrap();
            return Some((
                full.start(),
                full.as_str().trim_end_matches(['"', '\'', '`']).to_string(),
                "Hardcoded Windows user directory",
            ));
        }
    }

    None
}

impl HardcodedAbsDirDetector {
    #[inline]
    fn has_suppression(lines: &[String], idx: usize) -> bool {
        let check = |s: &str| {
            s.contains("toestub-ignore")
                || s.contains("vox-arch-check: allow abs-path")
                || s.contains("drift-allow(hardcoded-abs-dir)")
                || s.contains("drift-allow(drift/hardcoded-abs-dir)")
        };
        if check(&lines[idx]) {
            return true;
        }
        if idx > 0 {
            let prev = lines[idx - 1].trim_start();
            if (prev.starts_with("//") || prev.starts_with('#') || prev.starts_with("/*"))
                && check(prev)
            {
                return true;
            }
        }
        false
    }

    #[inline]
    fn is_comment_or_const_prefix(trimmed: &str) -> bool {
        trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("--")
    }

    #[inline]
    fn is_const_declaration(trimmed: &str) -> bool {
        trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("pub(crate) const ")
            || trimmed.starts_with("static ")
            || trimmed.starts_with("pub static ")
            || trimmed.starts_with("pub(crate) static ")
            || trimmed.starts_with("export const ")
    }

    fn make_finding(
        &self,
        file: &SourceFile,
        line_num: usize,
        col_0: usize,
        severity: Severity,
        message: String,
        suggestion: &str,
        confidence: FindingConfidence,
    ) -> Finding {
        Finding {
            rule_id: self.id().to_string(),
            diagnostic_id: Some(FS_HARDCODED_ABS_DIR.to_string()),
            rule_name: self.name().to_string(),
            severity,
            file: file.path.clone(),
            line: line_num,
            column: col_0 + 1,
            message,
            suggestion: Some(suggestion.to_string()),
            alternatives: vec![
                "Resolve via `std::env::current_dir()` / `vox_config` / `import.meta.dirname`."
                    .to_string(),
                "Pass via CLI argument or environment variable (`VOX_*`, `process.env.*`)."
                    .to_string(),
            ],
            rationale: Some(
                "Hardcoded workstation and machine-local directories break portability across OSes/contributors and leak local usernames into public artifacts."
                    .to_string(),
            ),
            context: file.context_around(line_num, 1),
            confidence: Some(confidence),
            evidence: None,
        }
    }
}

impl DetectionRule for HardcodedAbsDirDetector {
    fn id(&self) -> &'static str {
        FS_HARDCODED_ABS_DIR
    }

    fn diagnostic_id(&self) -> Option<&'static str> {
        Some(FS_HARDCODED_ABS_DIR)
    }

    fn name(&self) -> &'static str {
        "Hardcoded Absolute Directory Detector"
    }

    fn description(&self) -> &'static str {
        "Detects machine-local workstation paths (/Users/<user>, /home/<user>, C:\\Users\\<user>, .gemini/antigravity/brain, /private/tmp) and unanchored OS root paths"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &self.langs
    }

    fn minimal_repro(&self) -> Option<&'static str> {
        Some(
            "// VIOLATION — machine-local absolute path (toestub-ignore: doc example)\n\
             let root = \"/Users/dev/workspace/project\";\n\
             \n\
             // FIX — use workspace-relative or env-resolved path\n\
             let root = std::env::current_dir()?;",
        )
    }

    fn detect(
        &self,
        file: &SourceFile,
        rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        // File-level O(1) fast skip: no path separator means zero possible absolute directories.
        if !file.content.contains('/') && !file.content.contains('\\') {
            return Vec::new();
        }
        // Fast check if file has any quoted string or backtick literal.
        if !file.content.contains('"')
            && !file.content.contains('\'')
            && !file.content.contains('`')
        {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let path_str = file.path.to_string_lossy();
        let file_is_test = Self::is_test_path(&path_str);
        let mut in_test_block = false;

        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim_start();

            if !file_is_test
                && (trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("#[test]"))
            {
                in_test_block = true;
            } else if in_test_block
                && !line.starts_with(char::is_whitespace)
                && (trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub ")
                    || trimmed.starts_with("mod "))
                && !trimmed.contains("test")
            {
                in_test_block = false;
            }

            // O(1) line pre-filters before touching regexes.
            if (!line.contains('/') && !line.contains('\\'))
                || (!line.contains('"') && !line.contains('\'') && !line.contains('`'))
            {
                continue;
            }
            if Self::is_comment_or_const_prefix(trimmed)
                || rust_byte_is_comment(file, line_num, 0, rust_ctx)
                || Self::has_suppression(&file.lines, idx)
            {
                continue;
            }

            let is_test_ctx =
                file_is_test || in_test_block || rust_byte_is_cfg_test(file, line_num, 0, rust_ctx);

            if let Some((col, matched, kind)) = find_machine_local_dir(line, true, is_test_ctx) {
                let suggestion = if kind.contains("ephemeral") {
                    "Use workspace-relative paths or `std::env::temp_dir()` instead of IDE-private brain or macOS `/private/tmp` paths."
                } else {
                    "Use a repository-relative path, `dirs`/`vox_config`, or an environment variable."
                };
                findings.push(self.make_finding(
                    file,
                    line_num,
                    col,
                    Severity::Error,
                    format!("{kind} (`{matched}`) breaks portability and leaks usernames"),
                    suggestion,
                    FindingConfidence::High,
                ));
                continue;
            }

            // Tier 2: General unanchored OS root directory in non-test production code.
            if !is_test_ctx
                && !Self::is_const_declaration(trimmed)
                && !line.contains("Regex::new")
                && !line.contains("starts_with(\"/")
                && !line.contains("starts_with('/")
                && !line.contains("strip_prefix(\"/")
                && let Some(m) = ROOT_ABS_DIR_RE.find(line)
            {
                findings.push(self.make_finding(
                    file,
                    line_num,
                    m.start(),
                    Severity::Warning,
                    format!(
                        "Unanchored absolute OS directory literal ({}) in production code",
                        m.as_str()
                    ),
                    "Prefer `std::env::temp_dir()`, `vox_config`, or a configurable/workspace-relative path.",
                    FindingConfidence::Medium,
                ));
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scan_file(path: &str, code: &str) -> Vec<Finding> {
        let d = HardcodedAbsDirDetector::new();
        let f = SourceFile::new(PathBuf::from(path), code.to_string());
        d.detect(&f, None)
    }

    #[test]
    fn flags_machine_local_paths_across_languages() {
        let real = "realdev";
        for (path, code) in [
            (
                "crates/vox-cli/src/lib.rs",
                format!(r#"let root = "/Users/{real}/dev/vox";"#),
            ),
            (
                "crates/vox-gui/ui/e2e/review/flywheel.spec.ts",
                format!(r#"const repo = "/Users/{real}/dev/vox";"#),
            ),
            (
                "scripts/migrate.py",
                format!(r#"out = '/Users/{real}/artifacts/run.json'"#),
            ),
            (
                "examples/app.vox",
                format!(r#"let p = "/Users/{real}/dev/vox""#),
            ),
        ] {
            let findings = scan_file(path, &code);
            assert_eq!(
                findings.len(),
                1,
                "expected 1 Error finding for {path}, got {findings:?}"
            );
            assert_eq!(findings[0].severity, Severity::Error);
            assert_eq!(findings[0].rule_id, FS_HARDCODED_ABS_DIR);
        }
    }

    #[test]
    fn flags_linux_windows_file_uri_and_brain_paths() {
        let real = "realdev";
        let cases = [
            format!(r#"let p = "/home/{real}/project";"#),
            format!(r#"let p = r"C:\Users\{real}\project";"#),
            format!(r#"let p = "C:/Users/{real}/project";"#),
            format!(r#"let p = "file:///Users/{real}/foo";"#),
            format!(
                r#"let p = "/Users/someone/.gemini/{}/brain/0e2bd81e/out.txt";"#,
                "antigravity"
            ),
            format!(r#"let p = "{}/tmp/vox-out";"#, "/private"),
        ];
        for code in cases {
            let findings = scan_file("crates/vox-foo/src/lib.rs", &code);
            assert!(
                !findings.is_empty() && findings[0].severity == Severity::Error,
                "expected Error finding for `{code}`, got {findings:?}"
            );
        }
    }

    #[test]
    fn exempts_synthetic_users_in_tests_only_not_real_workstation_or_brain_paths() {
        let real = "realdev";
        let synthetic_in_unit_test = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn parses_home() {
        let a = "/home/alice/.vox";
        let b = "/Users/someone/project";
        let c = r"C:\Users\Owner\vox";
        let d = "/home/u/.cargo/bin";
    }
}
"#;
        assert!(
            scan_file("crates/vox-config/src/paths.rs", synthetic_in_unit_test).is_empty(),
            "synthetic usernames in #[cfg(test)] should be allowed"
        );
        assert!(
            scan_file(
                "crates/vox-gui/ui/src/lib/foo.test.ts",
                r#"const doc = '/Users/me/dev/vox/docs/a.md';"#,
            )
            .is_empty(),
            "synthetic `me` in *.test.ts should be allowed"
        );

        // Real workstation username or brain path MUST still error inside E2E / test files!
        let e2e_real = format!(
            r#"const out = "/Users/{real}/.gemini/{}/brain/abc/out.json";"#,
            "antigravity"
        );
        let findings = scan_file("crates/vox-gui/ui/e2e/review/cv.spec.ts", &e2e_real);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);

        let e2e_home = format!(r#"const repo = "/Users/{real}/dev/vox";"#);
        let findings2 = scan_file("crates/vox-gui/ui/e2e/review/cv.spec.ts", &e2e_home);
        assert_eq!(findings2.len(), 1);
        assert_eq!(findings2[0].severity, Severity::Error);
    }

    #[test]
    fn flags_synthetic_user_in_non_test_production_code() {
        let user = "alice";
        let prod = format!(r#"pub fn default_home() -> &'static str {{ "/Users/{user}/foo" }}"#);
        let findings = scan_file("crates/vox-config/src/paths.rs", &prod);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn tier2_os_root_dir_warns_in_prod_skips_const_and_tests() {
        let prod = r#"let p = "/tmp/my_cache";"#;
        let findings = scan_file("crates/vox-cli/src/cache.rs", prod);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);

        let in_const = r#"const DEFAULT_CACHE: &str = "/tmp/my_cache";"#;
        assert!(scan_file("crates/vox-cli/src/cache.rs", in_const).is_empty());

        let in_test = "#[cfg(test)]\nmod tests {\n    fn t() { let p = \"/tmp/my_cache\"; }\n}\n";
        assert!(scan_file("crates/vox-cli/src/cache.rs", in_test).is_empty());
    }

    #[test]
    fn honors_inline_and_above_suppressions() {
        let real = "realdev";
        let code = format!(
            r#"
let a = "/Users/{real}/a"; // toestub-ignore(vox/fs/hardcoded-abs-dir): fixture
// vox-arch-check: allow abs-path
let b = "/Users/{real}/b";
// drift-allow(hardcoded-abs-dir): test fixture
let c = "/Users/{real}/c";
"#
        );
        assert!(scan_file("crates/vox-foo/src/lib.rs", &code).is_empty());
    }

    #[test]
    fn skips_bare_prefixes_and_regexes() {
        let code = r#"
if s.starts_with("/Users/") || s.starts_with("/home/") { return true; }
let re = Regex::new(r"(?:/home/[^/\s]+|[A-Za-z]:\\Users\\[^\\]+)").unwrap();
"#;
        assert!(scan_file("crates/vox-redact/src/lib.rs", code).is_empty());
    }
}
