//! Optional Mermaid CLI integration: fenced diagrams become vector PDF figures.
use crate::{figures::Figure, Block, Document};
use anyhow::{bail, ensure, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn is_fence(line: &str) -> bool {
    line.strip_prefix("```")
        .and_then(|s| s.strip_prefix("mermaid"))
        .is_some_and(|s| s.is_empty() || s.starts_with(char::is_whitespace) || s.starts_with('{'))
}
pub(super) fn figure(language: &str) -> Result<Figure> {
    let attrs = language
        .strip_prefix("mermaid")
        .context("expected mermaid fence")?
        .trim();
    let figure = crate::figures::parse(&format!("![Diagrama Mermaid](diagram.pdf){attrs}"))?;
    ensure!(
        figure.page == 1,
        "Mermaid diagrams have one page; page must be 1"
    );
    Ok(figure)
}
pub(super) fn validate(text: &str) -> Result<()> {
    ensure!(!text.trim().is_empty(), "Mermaid diagram is empty");
    ensure!(text.len() <= 50_000, "Mermaid diagram exceeds 50,000 bytes");
    ensure!(!text.contains("%%{") && !text.trim_start().starts_with("---"), "Mermaid init directives and YAML front matter are not supported; use @caption and fence attributes");
    Ok(())
}
fn renderer(explicit: Option<&Path>) -> Result<Command> {
    let chosen = explicit
        .map(Path::to_owned)
        .or_else(|| std::env::var_os("CARBON_MERMAID_CLI").map(PathBuf::from));
    let local = Path::new("tools/mermaid/node_modules/@mermaid-js/mermaid-cli/src/cli.js");
    let chosen = chosen.or_else(|| {
        if local.is_file() {
            return dunce::canonicalize(local).ok();
        }
        let exe = std::env::current_exe().ok()?;
        exe.ancestors()
            .skip(1)
            .map(|p| p.join(local))
            .find(|p| p.is_file())
    });
    if let Some(path) = chosen {
        let path = if path.components().count() > 1 || path.is_file() {
            // std::fs::canonicalize adds a Windows verbatim (\\?\) prefix.
            // Node 22 cannot reliably resolve that prefix on runner drive D:.
            dunce::canonicalize(&path)
                .with_context(|| format!("cannot locate Mermaid CLI {}", path.display()))?
        } else {
            path
        };
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("js") || e.eq_ignore_ascii_case("mjs"))
        {
            let mut command = Command::new("node");
            command.arg(path);
            Ok(command)
        } else {
            ensure!(!path.extension().is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat")), "use Mermaid's src/cli.js with --mermaid-cli on Windows instead of .cmd/.bat");
            Ok(Command::new(path))
        }
    } else if cfg!(windows) {
        bail!("Mermaid CLI not installed; run npm ci --prefix tools/mermaid, or pass --mermaid-cli PATH/cli.js")
    } else {
        Ok(Command::new("mmdc"))
    }
}
pub(super) fn stage(doc: &Document, destination: &Path, executable: Option<&Path>) -> Result<()> {
    if !doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::Mermaid { .. }))
    {
        return Ok(());
    }
    fs::create_dir_all(destination.join("assets"))?;
    fs::write(
        destination.join("mermaid-config.json"),
        r#"{"securityLevel":"strict","theme":"neutral","fontFamily":"Arial","maxTextSize":50000,"flowchart":{"htmlLabels":false}}"#,
    )?;
    for (index, block) in doc.blocks.iter().enumerate() {
        if let Block::Mermaid { text, figure } = block {
            validate(text)?;
            let input = format!("diagram-{index}.mmd");
            fs::write(destination.join(&input), text)?;
            let asset = figure.asset_name(index)?;
            let status = renderer(executable)?.current_dir(destination)
                .args(["--input", &input, "--output", &asset, "--outputFormat", "pdf", "--backgroundColor", "white", "--configFile", "mermaid-config.json"])
                .output().context("cannot start Mermaid CLI; install Node >=22.13 and run npm ci --prefix tools/mermaid")?;
            ensure!(
                status.status.success(),
                "Mermaid diagram {} failed to render:\n{}\n{}",
                index + 1,
                String::from_utf8_lossy(&status.stderr),
                String::from_utf8_lossy(&status.stdout)
            );
            let bytes =
                fs::read(destination.join(asset)).context("Mermaid CLI did not produce a PDF")?;
            let pdf = printpdf::lopdf::Document::load_mem(&bytes)
                .context("Mermaid CLI produced an invalid PDF")?;
            ensure!(
                pdf.get_pages().len() == 1,
                "Mermaid diagram must fit on one PDF page"
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fenced_diagrams_are_figures() {
        let doc = crate::parse("See @ref(fig:flow).\n\n@caption Pipeline\n```mermaid {#fig:flow width=90%}\nflowchart LR\nA --> B\n```\n\n```rust\nfn main() {}\n```").unwrap();
        let Block::Mermaid { text, figure } = &doc.blocks[1] else {
            panic!("expected diagram")
        };
        assert_eq!(text, "flowchart LR\nA --> B");
        assert_eq!(figure.caption, "Pipeline");
        assert_eq!(figure.id.as_deref(), Some("fig:flow"));
        let tex = crate::latex::source(&doc, crate::Paper::Letter).unwrap();
        assert!(tex.contains(r"\label{carbon:fig:flow}"));
        assert!(tex.contains(r"\ref{carbon:fig:flow}"));
        assert!(matches!(doc.blocks[2], Block::Code { .. }));
    }
    #[test]
    fn invalid_blocks_are_rejected() {
        for bad in [
            "```mermaid\n```",
            "```mermaid\nflowchart LR",
            "```mermaid {page=2}\nA-->B\n```",
            "```mermaid\n%%{init:{}}%%\nA-->B\n```",
            "```mermaid\n---\nconfig: {}\n---\nA-->B\n```",
        ] {
            assert!(crate::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    #[ignore = "requires Node.js"]
    fn node_entrypoint_works_from_another_directory() {
        let scripts = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let entry = scripts.path().join("renderer com espaco.mjs");
        fs::write(&entry, "console.log('carbon-path-ok');").unwrap();
        let mut command = renderer(Some(&entry)).unwrap();
        #[cfg(windows)]
        {
            let argument = command.get_args().next().unwrap().to_string_lossy();
            assert!(
                !argument.starts_with(r"\\?\"),
                "Node entry point uses a verbatim path: {argument}"
            );
        }
        let output = command.current_dir(work.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "carbon-path-ok"
        );
    }
    #[test]
    #[ignore = "requires Mermaid CLI, Chromium, and Tectonic"]
    fn mermaid_end_to_end_and_invalid_syntax() {
        let doc = crate::parse(include_str!("../examples/mermaid/diagramas.cb")).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/mermaid");
        let (bytes, pages) =
            crate::latex::render_at(&doc, None, crate::Paper::Letter, &base).unwrap();
        assert!(pages >= 1);
        assert!(bytes.starts_with(b"%PDF-"));
        let invalid = crate::parse("```mermaid\nthis is not a diagram\n```").unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(stage(&invalid, dir.path(), None).is_err());
    }
}
