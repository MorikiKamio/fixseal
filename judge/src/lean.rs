use crate::{Reason, sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

fn timeout_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(120)
}

fn timeout() -> Duration {
    Duration::from_secs(timeout_seconds(
        std::env::var("FIXSEAL_LEAN_TIMEOUT_SECS").ok().as_deref(),
    ))
}

pub const SPEC_SHA256: &str = "f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b";
pub const PROTECTED_PATHS: [&str; 6] = [
    "Spec.lean",
    "Path.lean",
    "Main.lean",
    "Audit.lean",
    "lakefile.toml",
    "lean-toolchain",
];
const TRUSTED: [(&str, &[u8]); 6] = [
    ("Spec.lean", include_bytes!("../../lean/Spec.lean")),
    ("Path.lean", include_bytes!("../../lean/Path.lean")),
    ("Main.lean", include_bytes!("../../lean/Main.lean")),
    ("Audit.lean", include_bytes!("../../lean/Audit.lean")),
    ("lakefile.toml", include_bytes!("../../lean/lakefile.toml")),
    (
        "lean-toolchain",
        include_bytes!("../../lean/lean-toolchain"),
    ),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    #[default]
    Path,
    Spend,
}

impl Subject {
    pub fn is_path(&self) -> bool {
        *self == Self::Path
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Spend => "spend",
        }
    }

    pub fn spec_sha256(self) -> &'static str {
        match self {
            Self::Path => SPEC_SHA256,
            Self::Spend => "e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe",
        }
    }

    pub fn trusted(self) -> &'static [(&'static str, &'static [u8])] {
        match self {
            Self::Path => &TRUSTED,
            Self::Spend => &[
                (
                    "SpendSpec.lean",
                    include_bytes!("../../lean/targets/spend/SpendSpec.lean"),
                ),
                (
                    "Main.lean",
                    include_bytes!("../../lean/targets/spend/Main.lean"),
                ),
                (
                    "Audit.lean",
                    include_bytes!("../../lean/targets/spend/Audit.lean"),
                ),
                (
                    "TargetAudit.lean",
                    include_bytes!("../../lean/targets/spend/TargetAudit.lean"),
                ),
                (
                    "lakefile.toml",
                    include_bytes!("../../lean/targets/spend/lakefile.toml"),
                ),
                (
                    "lean-toolchain",
                    include_bytes!("../../lean/targets/spend/lean-toolchain"),
                ),
            ],
        }
    }

    fn spec_file(self) -> &'static str {
        match self {
            Self::Path => "Spec.lean",
            Self::Spend => "SpendSpec.lean",
        }
    }

    fn executable(self) -> &'static str {
        match self {
            Self::Path => "pathcheck",
            Self::Spend => "spendcheck",
        }
    }
}

type Tree = BTreeMap<String, Vec<u8>>;

// Lex executable tokens before invoking Lean; quoted identifiers and metaprogram syntax are excluded.
// Comments are nested and strings are single tokens, so executable directives cannot hide there.
fn tokens(bytes: &[u8]) -> Result<Vec<String>, Reason> {
    lex(bytes, false)
}

fn lex(bytes: &[u8], policy_only: bool) -> Result<Vec<String>, Reason> {
    let text = std::str::from_utf8(bytes).map_err(|_| Reason::ProofCheated)?;
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"--") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/-") {
            let mut depth = 1;
            i += 2;
            while depth > 0 && i < bytes.len() {
                if bytes[i..].starts_with(b"/-") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"-/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(Reason::ProofCheated);
            }
        } else if bytes[i] == b'"' {
            let start = i;
            i += 1;
            loop {
                match bytes.get(i) {
                    Some(b'\\') => i += 2,
                    Some(b'"') => {
                        i += 1;
                        break;
                    }
                    Some(_) => i += 1,
                    None => return Err(Reason::ProofCheated),
                }
            }
            let literal = &text[start..i];
            serde_json::from_str::<String>(literal).map_err(|_| Reason::ProofCheated)?;
            result.push(literal.into());
        } else if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'.' | b'\''))
            {
                i += 1;
            }
            result.push(text[start..i].into());
        } else if let Some(symbol) = [
            ":=", "=>", "==", "&&", "||", "≤", "≥", "↔", "→", "←", "¬", "∧", "∨", "⟨", "⟩", "∀",
            "∃", "≠", "<;>", "(", ")", ":", "[", "]", ",", "!", "·", "+", "-", "*", "/", "%", "^",
            "<", ">", "=", "|", ";", "{", "}", ".", "↑",
        ]
        .iter()
        .find(|s| bytes[i..].starts_with(s.as_bytes()))
        {
            result.push((*symbol).into());
            i += symbol.len();
        } else if policy_only {
            // Keep inspecting identifiers even in syntax that the executable lexer rejects.
            i += text[i..].chars().next().unwrap().len_utf8();
        } else {
            return Err(Reason::ProofCheated);
        }
    }
    Ok(result)
}

fn body<'a>(source: &'a [String], header: &str, footer: &str) -> Result<&'a [String], Reason> {
    let header: Vec<_> = header.split_whitespace().collect();
    let footer: Vec<_> = footer.split_whitespace().collect();
    if source.len() < header.len() + footer.len()
        || !source.iter().zip(&header).all(|(a, b)| a == b)
        || !source[source.len() - footer.len()..]
            .iter()
            .zip(&footer)
            .all(|(a, b)| a == b)
    {
        return Err(Reason::ProofCheated);
    }
    Ok(&source[header.len()..source.len() - footer.len()])
}

fn string_list(tokens: &[String]) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut i = 1; // The caller starts at an opening bracket.
    loop {
        if tokens.get(i)? == "]" {
            return Some(parts);
        }
        let mut parentheses = 0;
        while tokens.get(i)? == "(" {
            parentheses += 1;
            i += 1;
        }
        parts.push(serde_json::from_str::<String>(tokens.get(i)?).ok()?);
        i += 1;
        for _ in 0..parentheses {
            if tokens.get(i)? != ")" {
                return None;
            }
            i += 1;
        }
        match tokens.get(i)?.as_str() {
            "]" => return Some(parts),
            "," => i += 1,
            _ => return None,
        }
    }
}

pub(crate) fn contains_attack_literal(source: &[u8], attacks: &[&str]) -> bool {
    let Ok(tokens) = tokens(source) else {
        return false; // Invalid syntax is rejected separately by inspect.
    };
    let lists: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.as_str() == "[")
        .filter_map(|(i, _)| string_list(&tokens[i..]))
        .collect();
    attacks.iter().any(|attack| {
        let decoded = crate::vm::decode_percent(attack);
        [Some(*attack), decoded.as_deref()]
            .into_iter()
            .flatten()
            .any(|attack| {
                if tokens.iter().any(|token| {
                    serde_json::from_str::<String>(token)
                        .is_ok_and(|literal| literal.contains(attack))
                }) {
                    return true;
                }
                let parts: Vec<_> = attack.split('/').collect();
                lists.iter().any(|list| list == &parts)
            })
    })
}

pub(crate) fn contains_spend_literal(source: &[u8], attacks: &[&str]) -> bool {
    let Ok(tokens) = tokens(source) else {
        return false;
    };
    // Match a full numeric attack triple; shared boundary constants alone are legitimate.
    let numbers: Vec<u64> = tokens
        .iter()
        .filter_map(|token| {
            for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
                if let Some(digits) = token.strip_prefix(prefix) {
                    return u64::from_str_radix(digits, radix).ok();
                }
            }
            token.parse().ok()
        })
        .collect();
    attacks.iter().any(|attack| {
        crate::spend_request(attack)
            .is_some_and(|(l, s, a)| numbers.windows(3).any(|values| values == [l, s, a]))
    })
}

fn toolchain_bin() -> Result<PathBuf, Reason> {
    // Only direct toolchain binaries are used; never fall back to an elan proxy.
    if let Some(bin) = std::env::var_os("FIXSEAL_LEAN_BIN") {
        return Ok(PathBuf::from(bin));
    }
    let home = std::env::var_os("HOME").ok_or(Reason::ProofRejected)?;
    Ok(PathBuf::from(home).join(".elan/toolchains/leanprover--lean4---v4.34.1/bin"))
}

pub(crate) fn target_policy_violation(source: &[u8], forbidden: &[String]) -> Option<String> {
    // Invalid lexical syntax is rejected by inspect; comments and strings are not identifiers.
    let tokens = lex(source, true).ok()?;
    for (i, token) in tokens.iter().enumerate() {
        if token.starts_with('"') {
            continue;
        }
        for part in token.split('.') {
            if forbidden.iter().any(|name| name == part) {
                return Some(format!("forbidden identifier in Target.lean: {part}"));
            }
        }
        // Numeric projections and constructor patterns can unpack UInt64 without naming a cast.
        if token == "⟨"
            || (token.contains('.') && token.split('.').skip(1).any(|p| p.parse::<u64>().is_ok()))
            || (token == "." && tokens.get(i + 1).is_some_and(|p| p.parse::<u64>().is_ok()))
        {
            return Some("forbidden representation access in Target.lean".into());
        }
    }
    None
}

fn target_allowed(source: &[u8]) -> Result<(), Reason> {
    let tokens = tokens(source)?;
    let body = body(
        &tokens,
        "import Spec namespace Target def join ( root : Spec.Path ) ( request : Spec.Request ) : Option Spec.Path :=",
        "end Target",
    )?;
    expression_allowed(body)?;
    if body
        .iter()
        .enumerate()
        .any(|(i, t)| t == "[" && string_list(&body[i..]).is_none())
    {
        return Err(Reason::ProofCheated);
    }
    Ok(())
}

// No declarations, environment mutation, metaprogram execution or alternate runtimes.
// Identifiers for core lemmas and local hypotheses are otherwise unrestricted.
fn expression_allowed(body: &[String]) -> Result<(), Reason> {
    let forbidden = [
        "sorry",
        "admit",
        "sorryAx",
        "native_decide",
        "bv_decide",
        "native",
        "unsafe",
        "unsafeCast",
        "implemented_by",
        "extern",
        "opaque",
        "axiom",
        "set_option",
        "import",
        "namespace",
        "end",
        "def",
        "theorem",
        "lemma",
        "abbrev",
        "constant",
        "constants",
        "example",
        "instance",
        "structure",
        "class",
        "inductive",
        "mutual",
        "partial",
        "noncomputable",
        "initialize",
        "builtin_initialize",
        "macro",
        "macro_rules",
        "syntax",
        "elab",
        "elab_rules",
        "run_tac",
        "run_elab",
        "eval",
        "IO",
        "EIO",
        "ST",
        "Lean",
        "Lake",
        "attribute",
        "local",
        "scoped",
        "open",
        "export",
        "include",
        "omit",
        "section",
        "universe",
        "variable",
        "variables",
        "deriving",
        "termination_by",
        "decreasing_by",
        "declaration",
        "ofReduceBool",
        "ofReduceNat",
    ];
    if body.iter().any(|token| {
        !token.starts_with('"') && token.split('.').any(|part| forbidden.contains(&part))
    }) {
        return Err(Reason::ProofCheated);
    }
    Ok(())
}

fn proof_allowed_for(subject: Subject, source: &[u8]) -> Result<(), Reason> {
    let tokens = tokens(source)?;
    let header = match subject {
        Subject::Path => "import Target theorem patch_sound : Spec.Correct Target.join := by",
        Subject::Spend => {
            "import Target theorem patch_sound : SpendSpec.Correct Target.authorize := by"
        }
    };
    expression_allowed(body(&tokens, header, "")?)
}

fn target_allowed_for(subject: Subject, source: &[u8]) -> Result<(), Reason> {
    if subject == Subject::Path {
        return target_allowed(source);
    }
    let tokens = tokens(source)?;
    expression_allowed(body(
        &tokens,
        "import SpendSpec namespace Target def authorize ( limit spent amount : UInt64 ) : Bool :=",
        "end Target",
    )?)
}

pub fn inspect(baseline: &Tree, candidate: &Tree) -> Option<Reason> {
    inspect_for(Subject::Path, baseline, candidate)
}

pub fn inspect_for(subject: Subject, baseline: &Tree, candidate: &Tree) -> Option<Reason> {
    for tree in [baseline, candidate] {
        if tree
            .get(subject.spec_file())
            .is_none_or(|bytes| sha256(bytes) != subject.spec_sha256())
        {
            return Some(Reason::ProofCheated);
        }
        for &(path, bytes) in subject.trusted() {
            if tree.get(path).map(Vec::as_slice) != Some(bytes) {
                return Some(Reason::ProofCheated);
            }
        }
        if let Some(source) = tree.get("Target.lean")
            && target_allowed_for(subject, source).is_err()
        {
            return Some(Reason::ProofCheated);
        }
    }
    if !candidate.contains_key("Target.lean") {
        return Some(Reason::ProofMissing);
    }
    match candidate.get("Proof.lean") {
        Some(proof) => proof_allowed_for(subject, proof).err(),
        None => Some(Reason::ProofMissing),
    }
}

struct Project(PathBuf);

impl Project {
    fn new(subject: Subject, source: &[u8], proof: Option<&[u8]>) -> Result<Self, Reason> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "fixseal-lean-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).map_err(|_| Reason::ProofRejected)?;
        let project = Self(root);
        fs::create_dir(project.0.join("after_honest")).map_err(|_| Reason::ProofRejected)?;
        for &(path, bytes) in subject.trusted() {
            fs::write(project.0.join(path), bytes).map_err(|_| Reason::ProofRejected)?;
        }
        fs::write(project.0.join("after_honest/Target.lean"), source)
            .map_err(|_| Reason::ProofRejected)?;
        if let Some(proof) = proof {
            fs::write(project.0.join("after_honest/Proof.lean"), proof)
                .map_err(|_| Reason::ProofRejected)?;
        }
        Ok(project)
    }

    fn lake(&self, args: &[&str]) -> Result<std::process::Output, Reason> {
        // Use the installed toolchain directly: an elan proxy could attempt a download.
        let bin = toolchain_bin()?;
        self.output(
            Command::new(bin.join("lake"))
                .current_dir(&self.0)
                .env_remove("LEAN_PATH")
                .env_remove("LEAN_SRC_PATH")
                .env_remove("LEAN_SYSROOT")
                .env("ELAN_TOOLCHAIN", "leanprover/lean4:v4.34.1")
                .stdin(Stdio::null())
                .args(args),
        )
    }

    fn audit_target(&self) -> Result<String, Reason> {
        let audit = self.lake(&["env", "lean", "TargetAudit.lean"])?;
        if !audit.status.success() || !audit.stderr.is_empty() {
            return Err(Reason::ProofRejected);
        }
        let output = String::from_utf8_lossy(&audit.stdout);
        if output.contains("FIXSEAL_TARGET_POLICY_VIOLATION:") {
            return Err(Reason::PolicyViolation);
        }
        if !output
            .lines()
            .any(|line| line == "FIXSEAL_TARGET_POLICY_OK")
        {
            return Err(Reason::ProofRejected);
        }
        let exports: Vec<_> = output
            .lines()
            .filter_map(|line| line.strip_prefix("FIXSEAL_POLICY_EXPR:"))
            .collect();
        if exports.len() != 1 {
            return Err(Reason::ProofRejected);
        }
        let expr =
            crate::policy_expr::BoolExpr::parse(exports[0]).map_err(|_| Reason::PolicyViolation)?;
        Ok(expr.canonical())
    }

    fn output(&self, command: &mut Command) -> Result<std::process::Output, Reason> {
        self.output_with_timeout(command, timeout())
    }

    fn output_with_timeout(
        &self,
        command: &mut Command,
        limit: Duration,
    ) -> Result<std::process::Output, Reason> {
        // Files avoid pipe backpressure and readers blocked by a timed-out descendant.
        let stdout = self.0.join("stdout.log");
        let stderr = self.0.join("stderr.log");
        command
            .stdout(Stdio::from(
                fs::File::create(&stdout).map_err(|_| Reason::ProofRejected)?,
            ))
            .stderr(Stdio::from(
                fs::File::create(&stderr).map_err(|_| Reason::ProofRejected)?,
            ));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let started = Instant::now();
        let mut child = command.spawn().map_err(|_| Reason::ProofRejected)?;
        loop {
            if let Some(status) = child.try_wait().map_err(|_| Reason::ProofRejected)? {
                return Ok(std::process::Output {
                    status,
                    stdout: fs::read(stdout).map_err(|_| Reason::ProofRejected)?,
                    stderr: fs::read(stderr).map_err(|_| Reason::ProofRejected)?,
                });
            }
            if started.elapsed() >= limit {
                // Lake starts Lean/C compiler descendants; terminate the whole group on Unix.
                #[cfg(unix)]
                let _ = Command::new("/bin/kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                let _ = child.kill();
                let _ = child.wait();
                return Err(Reason::ProofTimeout);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Execution {
    pub outputs: Vec<Option<String>>,
    pub policy_expr: Option<String>,
}

type RunResult = Result<Execution, Reason>;

pub fn run(source: &[u8], inputs: &[&str]) -> Result<Vec<Option<String>>, Reason> {
    run_for(Subject::Path, source, inputs)
}

pub fn run_for(
    subject: Subject,
    source: &[u8],
    inputs: &[&str],
) -> Result<Vec<Option<String>>, Reason> {
    run_export_for(subject, source, inputs).map(|execution| execution.outputs)
}

pub fn run_export_for(subject: Subject, source: &[u8], inputs: &[&str]) -> RunResult {
    target_allowed_for(subject, source)?;
    static CACHE: OnceLock<Mutex<BTreeMap<String, RunResult>>> = OnceLock::new();
    let key = sha256(
        &serde_json::to_vec(&(subject, toolchain_bin()?, timeout(), source, inputs))
            .expect("serializable inputs"),
    );
    let mut cache = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .expect("run cache poisoned");
    if let Some(result) = cache.get(&key) {
        return result.clone();
    }
    let result = (|| {
        let project = Project::new(subject, source, None)?;
        if !project
            .lake(&["build", subject.executable()])?
            .status
            .success()
        {
            return Err(Reason::ProofRejected);
        }
        let policy_expr = if subject == Subject::Spend {
            Some(project.audit_target()?)
        } else {
            None
        };
        let mut requests = Vec::new();
        for input in inputs {
            writeln!(
                requests,
                "{}",
                serde_json::to_string(input).expect("serializable string")
            )
            .map_err(|_| Reason::ProofRejected)?;
        }
        let input_path = project.0.join("requests.jsonl");
        fs::write(&input_path, requests).map_err(|_| Reason::ProofRejected)?;
        let stdin = fs::File::open(input_path).map_err(|_| Reason::ProofRejected)?;
        let output = project.output(
            Command::new(project.0.join(".lake/build/bin").join(subject.executable()))
                .stdin(Stdio::from(stdin)),
        )?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err(Reason::ProofRejected);
        }
        let text = std::str::from_utf8(&output.stdout).map_err(|_| Reason::ProofRejected)?;
        let results: Vec<_> = text
            .lines()
            .map(|line| {
                serde_json::from_str::<Option<String>>(line).map_err(|_| Reason::ProofRejected)
            })
            .collect::<Result<_, _>>()?;
        if results.len() != inputs.len() {
            return Err(Reason::ProofRejected);
        }
        Ok(Execution {
            outputs: results,
            policy_expr,
        })
    })();
    if result != Err(Reason::ProofTimeout) {
        cache.insert(key, result.clone());
    }
    result
}

pub fn verify(candidate: &Tree) -> Result<(), Reason> {
    verify_for(Subject::Path, candidate)
}

pub fn verify_for(subject: Subject, candidate: &Tree) -> Result<(), Reason> {
    let source = candidate.get("Target.lean").ok_or(Reason::ProofMissing)?;
    let proof = candidate.get("Proof.lean").ok_or(Reason::ProofMissing)?;
    if let Some(reason) = inspect_for(subject, candidate, candidate) {
        return Err(reason);
    }
    static CACHE: OnceLock<Mutex<BTreeMap<String, Result<(), Reason>>>> = OnceLock::new();
    let key = sha256(
        &serde_json::to_vec(&(subject, toolchain_bin()?, timeout(), source, proof))
            .expect("serializable sources"),
    );
    let mut cache = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .expect("proof cache poisoned");
    if let Some(result) = cache.get(&key) {
        return result.clone();
    }
    let result = (|| {
        let project = Project::new(subject, source, Some(proof))?;
        if !project.lake(&["build"])?.status.success() {
            return Err(Reason::ProofRejected);
        }
        if subject == Subject::Spend {
            project.audit_target()?;
        }
        let audit = project.lake(&["env", "lean", "Audit.lean"])?;
        if !audit.status.success() {
            return Err(Reason::ProofRejected);
        }
        let output = String::from_utf8_lossy(&audit.stdout);
        if !output.lines().any(|line| {
            line == "'checked_patch' does not depend on any axioms"
                || line
                    .strip_prefix("'checked_patch' depends on axioms: [")
                    .and_then(|s| s.strip_suffix(']'))
                    .is_some_and(|axioms| {
                        axioms.split(", ").all(|axiom| {
                            ["propext", "Quot.sound", "Classical.choice"].contains(&axiom)
                        })
                    })
        }) {
            return Err(Reason::ProofCheated);
        }
        Ok(())
    })();
    if result != Err(Reason::ProofTimeout) {
        cache.insert(key, result.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_configuration_requires_positive_whole_seconds() {
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("-1"),
            Some("1.5"),
            Some("invalid"),
        ] {
            assert_eq!(timeout_seconds(value), 120);
        }
        assert_eq!(timeout_seconds(Some("7")), 7);
    }

    #[cfg(unix)]
    #[test]
    fn process_output_is_drained_and_timeout_kills_descendants() {
        let project = Project::new(Subject::Path, b"", None).unwrap();
        let output = project
            .output_with_timeout(
                Command::new("/bin/sh").args(["-c", "printf out; printf err >&2; exit 3"]),
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
        let started = Instant::now();
        let result = project.output_with_timeout(
            Command::new("/bin/sh")
                .current_dir(&project.0)
                .args(["-c", "(sleep 2; touch survived) & wait"]),
            Duration::from_millis(100),
        );
        assert_eq!(result.unwrap_err(), Reason::ProofTimeout);
        assert!(started.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(2200));
        assert!(!project.0.join("survived").exists());
    }
}
