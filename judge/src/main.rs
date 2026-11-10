use fixseal_judge::bundle::{self, Evidence, Mode};
use fixseal_judge::input::{Bundle, load_bundle, load_lean_subject_bundle};
use fixseal_judge::{judge, judge_lean};
use std::io::{IsTerminal, Read, Write};
use std::path::Path;

fn print_summary(report: &serde_json::Value) {
    let reasons = report["reason_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reason| reason.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
    eprintln!(
        "{}  {}  attacks {}/{}  regressions {}/{}",
        report["verdict"].as_str().unwrap().to_uppercase(),
        if reasons.is_empty() { "[]" } else { &reasons },
        report["checks"]["candidate_attacks_working"],
        report["checks"]["attacks_total"],
        report["checks"]["regressions_passed"],
        report["checks"]["regressions_total"],
    );
}

fn run() -> Result<bool, String> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let show_summary =
        std::io::stderr().is_terminal() || args.first().is_some_and(|arg| arg == "--summary");
    if args.first().is_some_and(|arg| arg == "--summary") {
        args.remove(0);
    }
    if let [command, path] = &args[..]
        && command == "verify"
    {
        let bytes = std::fs::read(path).map_err(|_| "cannot read evidence bundle")?;
        let evidence: Evidence = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid evidence bundle JSON: {error}"))?;
        let (valid, lines) = bundle::verify(&evidence);
        for line in lines {
            println!("{line}");
        }
        println!(
            "{}",
            if valid {
                "VERIFIED"
            } else {
                "VERIFICATION FAILED"
            }
        );
        return Ok(valid);
    }
    let export = args.first().is_some_and(|arg| arg == "bundle");
    let args = if export { &args[1..] } else { &args[..] };
    let lean = args.first().is_some_and(|arg| arg == "--lean");
    let args = if lean { &args[1..] } else { args };
    let subject_explicit = args.first().is_some_and(|arg| arg == "--subject");
    let (subject, args) = match args {
        [flag, name, rest @ ..] if flag == "--subject" => {
            if !lean {
                return Err("--subject requires --lean".into());
            }
            let subject = match name.as_str() {
                "path" => fixseal_judge::lean::Subject::Path,
                "spend" => fixseal_judge::lean::Subject::Spend,
                _ => return Err("unknown subject; expected path or spend".into()),
            };
            (subject, rest)
        }
        _ => (fixseal_judge::lean::Subject::Path, args),
    };
    let bundle = match args {
        [flag] if flag == "--stdin" => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .read_to_end(&mut bytes)
                .map_err(|_| "cannot read stdin")?;
            let bundle =
                serde_json::from_slice::<Bundle>(&bytes).map_err(|_| "invalid bundle JSON")?;
            if subject_explicit && bundle.subject != subject {
                return Err("stdin subject differs from --subject".into());
            }
            bundle
        }
        [root, version] => {
            if lean {
                load_lean_subject_bundle(Path::new(root), version, subject)?
            } else {
                load_bundle(Path::new(root), version)?
            }
        }
        _ => {
            return Err(
                "usage: fixseal-judge [--summary] [bundle] [--lean [--subject path|spend]] TARGET_ROOT VERSION | [--lean] --stdin | verify BUNDLE.json".into(),
            );
        }
    };
    if export {
        let evidence = bundle::create(bundle, if lean { Mode::Lean } else { Mode::Vm });
        if show_summary {
            print_summary(&evidence.report);
        }
        serde_json::to_writer(std::io::stdout(), &evidence)
            .map_err(|_| "cannot write evidence bundle")?;
        println!();
        return Ok(true);
    }
    let report = if lean {
        judge_lean(&bundle)
    } else {
        judge(&bundle)
    };
    if show_summary {
        print_summary(&serde_json::to_value(&report).unwrap());
    }
    std::io::stdout()
        .write_all(&report.to_json())
        .map_err(|_| "cannot write report")?;
    Ok(report.verdict == "accept")
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}
