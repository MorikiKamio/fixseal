pub const ROOT: &str = "/sandbox/files";

#[derive(Debug)]
enum Op {
    DecodePercent,
    JoinRoot,
    Normalize,
    RequireWithinRoot,
    RejectExact(String),
    RejectAll,
    ReturnPath,
}

pub struct Program(Vec<Op>);

impl Program {
    pub fn parse(source: &[u8]) -> Result<Self, String> {
        let source = std::str::from_utf8(source).map_err(|_| "source is not UTF-8")?;
        let mut ops = Vec::new();
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if matches!(ops.last(), Some(Op::ReturnPath)) {
                return Err("instructions after return_path".into());
            }
            ops.push(match line {
                "decode_percent" => Op::DecodePercent,
                "join_root" => Op::JoinRoot,
                "normalize" => Op::Normalize,
                "require_within_root" => Op::RequireWithinRoot,
                "reject_all" => Op::RejectAll,
                "return_path" => Op::ReturnPath,
                _ => match line.strip_prefix("reject_exact ") {
                    Some(value) if !value.is_empty() => Op::RejectExact(value.into()),
                    _ => return Err("unknown instruction".into()),
                },
            });
        }
        if ops.len() > 64 || !matches!(ops.last(), Some(Op::ReturnPath)) {
            return Err("expected at most 64 instructions ending in return_path".into());
        }
        Ok(Self(ops))
    }

    pub fn run(&self, input: &str) -> Option<String> {
        let mut path = input.to_string();
        for op in &self.0 {
            match op {
                Op::DecodePercent => path = decode_percent(&path)?,
                Op::JoinRoot => {
                    if !path.starts_with('/') {
                        path = format!("{ROOT}/{path}");
                    }
                }
                Op::Normalize => path = normalize(&path),
                Op::RequireWithinRoot => {
                    if !within_root(&normalize(&path)) {
                        return None;
                    }
                }
                Op::RejectExact(value) if path == *value => return None,
                Op::RejectExact(_) => {}
                Op::RejectAll => return None,
                Op::ReturnPath => return Some(path),
            }
        }
        None
    }
}

pub fn decode_percent(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = (*bytes.get(i + 1)? as char).to_digit(16)?;
            let lo = (*bytes.get(i + 2)? as char).to_digit(16)?;
            decoded.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    if decoded.contains(&0) {
        return None;
    }
    String::from_utf8(decoded).ok()
}

pub fn normalize(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

pub fn within_root(path: &str) -> bool {
    path == ROOT || path.starts_with(&format!("{ROOT}/"))
}

pub fn attack_works(program: &Program, input: &str) -> bool {
    program
        .run(input)
        .is_some_and(|path| !within_root(&normalize(&path)))
}
