//! Служебные задачи: `cargo xtask <команда>`.
//!
//! `third-party [--check]` — список сторонних библиотек в README и полные тексты их лицензий в
//! `THIRD_PARTY_LICENSES.md`. MIT, Apache-2.0 и BSD требуют прикладывать copyright и текст лицензии
//! к распространяемым копиям, поэтому оба файла генерируются из `cargo metadata` и проверяются в CI
//! (`--check` завершается с ошибкой, если файлы устарели).

// Консольная утилита разработчика: печать в stdout/stderr здесь уместна.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;

/// Целевая платформа сборки: только её зависимости попадают в релиз.
const TARGET: &str = "x86_64-pc-windows-msvc";
/// Крейты workspace, которые не распространяются (служебные).
const NOT_SHIPPED: &[&str] = &["xtask"];
const README: &str = "README.md";
const LICENSES_FILE: &str = "THIRD_PARTY_LICENSES.md";
const START: &str = "<!-- third-party:start -->";
const END: &str = "<!-- third-party:end -->";
/// Файлы в пакете, которые содержат лицензию, copyright или NOTICE.
const LICENSE_PREFIXES: &[&str] = &[
    "LICENSE",
    "LICENCE",
    "COPYING",
    "COPYRIGHT",
    "NOTICE",
    "UNLICENSE",
];

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    repository: Option<String>,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
}

#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    /// `null` — обычная зависимость, `"build"`, `"dev"`.
    kind: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("third-party") => third_party(args.iter().any(|a| a == "--check")),
        _ => Err("использование: cargo xtask third-party [--check]".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn metadata() -> Res<Metadata> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            TARGET,
        ])
        .output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}

/// Пакеты, достижимые из `roots` по рёбрам, для которых `follow(kind)` истинно.
fn reachable(
    graph: &HashMap<&str, &Node>,
    roots: &[&str],
    follow: impl Fn(Option<&str>, bool) -> bool,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<(&str, bool)> = roots.iter().map(|r| (*r, true)).collect();
    while let Some((id, is_root)) = stack.pop() {
        let Some(node) = graph.get(id) else { continue };
        for dep in &node.deps {
            if dep
                .dep_kinds
                .iter()
                .any(|k| follow(k.kind.as_deref(), is_root))
                && seen.insert(dep.pkg.clone())
            {
                stack.push((dep.pkg.as_str(), false));
            }
        }
    }
    seen
}

fn third_party(check: bool) -> Res<()> {
    let meta = metadata()?;
    let packages: HashMap<&str, &Package> =
        meta.packages.iter().map(|p| (p.id.as_str(), p)).collect();
    let graph: HashMap<&str, &Node> = meta
        .resolve
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();
    let members: BTreeSet<&str> = meta.workspace_members.iter().map(String::as_str).collect();
    let roots: Vec<&str> = members
        .iter()
        .copied()
        .filter(|id| {
            packages
                .get(id)
                .is_some_and(|p| !NOT_SHIPPED.contains(&p.name.as_str()))
        })
        .collect();

    // В сборку входят обычные и build-зависимости (включая proc-macro) всего дерева.
    let shipped: BTreeSet<String> = reachable(&graph, &roots, |kind, _| kind != Some("dev"))
        .into_iter()
        .filter(|id| !members.contains(id.as_str()))
        .collect();
    // Для разработки: dev-зависимости крейтов workspace и всё, что они тянут.
    let all: BTreeSet<String> = reachable(&graph, &roots, |kind, is_root| {
        is_root || kind != Some("dev")
    });
    let dev_only: BTreeSet<String> = all
        .into_iter()
        .filter(|id| !members.contains(id.as_str()) && !shipped.contains(id))
        .collect();

    let sorted = |ids: &BTreeSet<String>| {
        let mut v: Vec<&Package> = ids
            .iter()
            .filter_map(|id| packages.get(id.as_str()).copied())
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        v
    };
    let shipped = sorted(&shipped);
    let dev_only = sorted(&dev_only);

    let readme_path = meta.workspace_root.join(README);
    let readme = fs::read_to_string(&readme_path)?;
    let new_readme = replace_section(&readme, &readme_section(&shipped, &dev_only))?;
    let (licenses, missing) = licenses_file(&shipped)?;
    let licenses_path = meta.workspace_root.join(LICENSES_FILE);

    if check {
        let current = fs::read_to_string(&licenses_path).unwrap_or_default();
        if normalize(&new_readme) != normalize(&readme)
            || normalize(&licenses) != normalize(&current)
        {
            return Err(format!(
                "{README} или {LICENSES_FILE} устарели: запустите `cargo xtask third-party`"
            )
            .into());
        }
        println!("списки сторонних библиотек актуальны");
        return Ok(());
    }
    fs::write(&readme_path, new_readme)?;
    fs::write(&licenses_path, licenses)?;
    println!(
        "{README}: {} библиотек в сборке, {} только для разработки; {LICENSES_FILE} обновлён",
        shipped.len(),
        dev_only.len()
    );
    for name in &missing {
        eprintln!("внимание: в пакете {name} нет файла лицензии — нужен текст из репозитория");
    }
    Ok(())
}

/// Сравнение без учёта CRLF/LF: git на Windows может менять окончания строк.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn license(p: &Package) -> &str {
    p.license.as_deref().unwrap_or("см. файл лицензии")
}

fn table(out: &mut String, pkgs: &[&Package]) {
    out.push_str("| Библиотека | Версия | Лицензия | Исходный код |\n|---|---|---|---|\n");
    for p in pkgs {
        let repo = p
            .repository
            .as_deref()
            .map_or_else(|| "—".to_owned(), |r| format!("<{r}>"));
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {repo} |",
            p.name,
            p.version,
            license(p)
        );
    }
}

fn readme_section(shipped: &[&Package], dev_only: &[&Package]) -> String {
    let mut s = String::new();
    s.push_str(START);
    s.push_str(
        "\n<!-- Генерируется командой `cargo xtask third-party`. Не редактируйте вручную. -->\n\n",
    );
    let _ = writeln!(
        s,
        "Проект использует следующие open source библиотеки. Полные тексты их лицензий и уведомления \
         об авторских правах — в [{LICENSES_FILE}]({LICENSES_FILE}); этот файл прикладывается к каждому релизу.\n"
    );
    let _ = writeln!(s, "### Входят в сборку ({})\n", shipped.len());
    table(&mut s, shipped);
    let _ = writeln!(
        s,
        "\n### Только для разработки и тестов ({})\n",
        dev_only.len()
    );
    s.push_str("Эти библиотеки не входят в распространяемые файлы.\n\n");
    table(&mut s, dev_only);
    s.push('\n');
    s.push_str(END);
    s
}

fn replace_section(readme: &str, section: &str) -> Res<String> {
    let (Some(start), Some(end)) = (readme.find(START), readme.find(END)) else {
        return Err(format!("в {README} нет маркеров {START} / {END}").into());
    };
    Ok(format!(
        "{}{section}{}",
        &readme[..start],
        &readme[end + END.len()..]
    ))
}

/// Файлы лицензий пакета: `license_file` из Cargo.toml и LICENSE*/COPYING*/NOTICE* в корне пакета.
fn license_texts(p: &Package) -> Res<Vec<(String, String)>> {
    let dir = p.manifest_path.parent().unwrap_or(Path::new("."));
    let mut files: BTreeMap<String, PathBuf> = BTreeMap::new();
    if let Some(f) = &p.license_file {
        files.insert(f.clone(), dir.join(f));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let upper = name.to_uppercase();
        if entry.file_type()?.is_file() && LICENSE_PREFIXES.iter().any(|pre| upper.starts_with(pre))
        {
            files.insert(name, entry.path());
        }
    }
    let mut out = Vec::new();
    for (name, path) in files {
        out.push((
            name,
            normalize(&fs::read_to_string(path)?).trim_end().to_owned(),
        ));
    }
    Ok(out)
}

fn licenses_file(shipped: &[&Package]) -> Res<(String, Vec<String>)> {
    // Одинаковые тексты (например, стандартный Apache-2.0) печатаются один раз со списком пакетов.
    let mut by_text: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut missing = Vec::new();
    for p in shipped {
        let texts = license_texts(p)?;
        if texts.is_empty() {
            missing.push(format!("{} {}", p.name, p.version));
        }
        for (file, text) in texts {
            by_text
                .entry(text)
                .or_default()
                .insert(format!("{} {} — {file}", p.name, p.version));
        }
    }
    let mut groups: Vec<(BTreeSet<String>, String)> = by_text
        .into_iter()
        .map(|(text, pkgs)| (pkgs, text))
        .collect();
    groups.sort();

    let mut s = String::from(
        "# Лицензии сторонних библиотек\n\n\
         <!-- Генерируется командой `cargo xtask third-party`. Не редактируйте вручную. -->\n\n\
         voice-to-text распространяется вместе со следующими библиотеками. Ниже — уведомления об авторских\n\
         правах и тексты лицензий в том виде, в каком они поставляются в пакетах crates.io.\n\n## Сводка\n\n",
    );
    s.push_str("| Библиотека | Версия | Лицензия |\n|---|---|---|\n");
    for p in shipped {
        let _ = writeln!(s, "| `{}` | {} | {} |", p.name, p.version, license(p));
    }
    if !missing.is_empty() {
        s.push_str("\n## Пакеты без файла лицензии\n\nВ этих пакетах crates.io нет файла лицензии; лицензия указана в их Cargo.toml (см. сводку), текст — в репозитории проекта.\n\n");
        for m in &missing {
            let _ = writeln!(s, "- {m}");
        }
    }
    s.push_str("\n## Тексты лицензий\n");
    for (pkgs, text) in &groups {
        s.push_str("\n---\n\n");
        for p in pkgs {
            let _ = writeln!(s, "- {p}");
        }
        let _ = writeln!(s, "\n```text\n{text}\n```");
    }
    Ok((s, missing))
}
