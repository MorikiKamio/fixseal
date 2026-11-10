use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    #[serde(default, skip_serializing_if = "crate::lean::Subject::is_path")]
    pub subject: crate::lean::Subject,
    pub version: String,
    pub baseline: BTreeMap<String, Vec<u8>>,
    pub candidate: BTreeMap<String, Vec<u8>>,
    pub corpus: Vec<u8>,
    pub policy: Vec<u8>,
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|_| "input file unavailable")?
        .file_type()
        .is_file()
    {
        return Err("input must be a regular file".into());
    }
    fs::read(path).map_err(|_| "cannot read input file".into())
}

fn read_tree(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
        if !fs::symlink_metadata(dir)
            .map_err(|_| "input directory unavailable")?
            .file_type()
            .is_dir()
        {
            return Err("input directory must not be a symlink".into());
        }
        for entry in fs::read_dir(dir).map_err(|_| "cannot read input directory")? {
            let path = entry.map_err(|_| "cannot read input entry")?.path();
            let kind = fs::symlink_metadata(&path)
                .map_err(|_| "cannot read input metadata")?
                .file_type();
            if kind.is_dir() {
                walk(root, &path, files)?;
            } else if kind.is_file() {
                let name = path
                    .strip_prefix(root)
                    .map_err(|_| "invalid relative path")?
                    .to_str()
                    .ok_or("input path is not UTF-8")?
                    .to_string();
                files.insert(name, read_file(&path)?);
            } else {
                return Err("symlinks and special files are not supported".into());
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files)?;
    Ok(files)
}

pub fn load_bundle(root: &Path, version: &str) -> Result<Bundle, String> {
    if !valid_path(version) || version.contains('/') {
        return Err("version must be a single relative path component".into());
    }
    if !fs::symlink_metadata(root)
        .map_err(|_| "target root unavailable")?
        .file_type()
        .is_dir()
    {
        return Err("target root must be a directory, not a symlink".into());
    }
    Ok(Bundle {
        subject: crate::lean::Subject::Path,
        version: version.into(),
        baseline: read_tree(&root.join("before"))?,
        candidate: read_tree(&root.join(version))?,
        corpus: read_file(&root.join("corpus.json"))?,
        policy: read_file(&root.join("policy.json"))?,
    })
}

pub fn load_lean_bundle(root: &Path, version: &str) -> Result<Bundle, String> {
    load_lean_subject_bundle(root, version, crate::lean::Subject::Path)
}

pub fn load_lean_subject_bundle(
    root: &Path,
    version: &str,
    subject: crate::lean::Subject,
) -> Result<Bundle, String> {
    let directory = match subject {
        crate::lean::Subject::Path => root.to_path_buf(),
        crate::lean::Subject::Spend => root.join("targets/spend"),
    };
    let mut bundle = load_bundle(&directory, version)?;
    bundle.subject = subject;
    for &(path, _) in subject.trusted() {
        let bytes = read_file(&directory.join(path))?;
        bundle.baseline.entry(path.into()).or_insert(bytes.clone());
        bundle.candidate.entry(path.into()).or_insert(bytes);
    }
    Ok(bundle)
}

pub fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0'])
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}
