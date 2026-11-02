//! Figure syntax and asset staging. Original paths never enter generated TeX.
use crate::{Block, Document};
use anyhow::{bail, ensure, Context, Result};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq)]
pub(super) struct Figure {
    pub description: String,
    pub caption: String,
    pub path: PathBuf,
    pub id: Option<String>,
    pub width: f32,
    pub placement: String,
    pub page: u32,
}
pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id.starts_with(|c: char| c.is_ascii_alphabetic())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ":._-".contains(c))
}
pub(super) fn parse(line: &str) -> Result<Figure> {
    let rest = line
        .strip_prefix("![")
        .context("expected ![description](path)")?;
    let (description, rest) = rest
        .split_once("](")
        .context("expected ]( after figure description")?;
    let close = rest.rfind(')').context("unclosed figure path")?;
    let path = rest[..close].trim();
    let path = path
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .unwrap_or(path);
    ensure!(
        !path.is_empty() && !path.contains("://"),
        "figure requires a local file path"
    );
    let mut fig = Figure {
        description: description.into(),
        caption: description.into(),
        path: path.into(),
        id: None,
        width: 0.75,
        placement: "htbp".into(),
        page: 1,
    };
    let attrs = rest[close + 1..].trim();
    if !attrs.is_empty() {
        let attrs = attrs
            .strip_prefix('{')
            .and_then(|v| v.strip_suffix('}'))
            .context("figure attributes must be inside {...}")?;
        let mut seen = std::collections::HashSet::new();
        for attr in attrs.split_whitespace() {
            let (key, value) = if let Some(id) = attr.strip_prefix('#') {
                ("id", id)
            } else {
                attr.split_once('=')
                    .context("expected #id, width=75%, placement=htbp or page=1")?
            };
            ensure!(seen.insert(key), "duplicate figure option {key}");
            match key {
                "id" => {
                    ensure!(valid_id(value), "invalid figure identifier {value:?}");
                    fig.id = Some(value.into());
                }
                "width" => {
                    fig.width = if let Some(v) = value.strip_suffix('%') {
                        v.parse::<f32>()? / 100.
                    } else {
                        value.parse()?
                    };
                }
                "placement" => {
                    ensure!(
                        !value.is_empty()
                            && value.chars().all(|c| "htbp!".contains(c))
                            && value.chars().any(|c| "htbp".contains(c)),
                        "invalid figure placement"
                    );
                    fig.placement = value.into();
                }
                "page" => fig.page = value.parse().context("invalid PDF page")?,
                _ => bail!("unknown figure option {key}"),
            }
        }
    }
    ensure!(
        fig.width.is_finite() && fig.width > 0. && fig.width <= 1.,
        "figure width must be in (0,1] or (0%,100%]"
    );
    ensure!(fig.page > 0, "PDF page starts at 1");
    Ok(fig)
}
/// Return command, label and consumed bytes, when an inline reference starts here.
pub(super) fn reference_at(text: &str) -> Result<Option<(&'static str, &str, usize)>> {
    for (prefix, command) in [
        ("@ref(", "ref"),
        ("@pageref(", "pageref"),
        ("@autoref(", "autoref"),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            let end = rest.find(')').context("unclosed figure reference")?;
            let id = &rest[..end];
            ensure!(valid_id(id), "invalid figure reference {id:?}");
            return Ok(Some((command, id, prefix.len() + end + 1)));
        }
    }
    Ok(None)
}
impl Figure {
    fn extension(&self) -> Result<String> {
        let ext = self
            .path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        ensure!(
            ["png", "jpg", "jpeg", "pdf"].contains(&ext.as_str()),
            "unsupported figure format {}; use PNG, JPEG or PDF",
            self.path.display()
        );
        ensure!(
            ext == "pdf" || self.page == 1,
            "page option is only supported for PDF figures"
        );
        Ok(ext)
    }
    pub fn asset_name(&self, index: usize) -> Result<String> {
        Ok(format!("assets/figure-{index}.{}", self.extension()?))
    }
}
pub(super) fn stage(doc: &Document, base: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination.join("assets"))?;
    for (index, block) in doc.blocks.iter().enumerate() {
        if let Block::Figure(fig) = block {
            let file = base.join(&fig.path);
            let resolved = fs::canonicalize(&file).with_context(|| {
                format!(
                    "cannot find figure {} (relative to {})",
                    fig.path.display(),
                    base.display()
                )
            })?;
            let mut input = fs::File::open(&resolved)
                .with_context(|| format!("cannot open figure {}", resolved.display()))?;
            ensure!(
                input.metadata()?.is_file(),
                "figure is not a regular file: {}",
                file.display()
            );
            let mut bytes = Vec::new();
            input
                .by_ref()
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= 64 * 1024 * 1024,
                "figure exceeds 64 MiB: {}",
                file.display()
            );
            let ext = fig.extension()?;
            if ext == "pdf" {
                let pdf = printpdf::lopdf::Document::load_mem(&bytes)
                    .with_context(|| format!("invalid PDF figure {}", file.display()))?;
                ensure!(
                    !pdf.is_encrypted(),
                    "encrypted PDF figure is not supported: {}",
                    file.display()
                );
                ensure!(
                    (fig.page as usize) <= pdf.get_pages().len(),
                    "figure {} has no page {}",
                    file.display(),
                    fig.page
                );
            } else {
                let mut reader =
                    image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format()?;
                let expected = if ext == "png" {
                    image::ImageFormat::Png
                } else {
                    image::ImageFormat::Jpeg
                };
                ensure!(
                    reader.format() == Some(expected),
                    "figure extension and file contents differ: {}",
                    file.display()
                );
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(20000);
                limits.max_image_height = Some(20000);
                limits.max_alloc = Some(256 * 1024 * 1024);
                reader.limits(limits);
                reader
                    .decode()
                    .with_context(|| format!("invalid or oversized image {}", file.display()))?;
            }
            fs::write(destination.join(fig.asset_name(index)?), &bytes)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn figure_options_and_paths() {
        let f = parse("![A **curve**](figuras com espaços/curva (final).PNG){#fig:curve width=80% placement=!ht page=1}").unwrap();
        assert_eq!(f.width, 0.8);
        assert_eq!(f.id.as_deref(), Some("fig:curve"));
        assert_eq!(f.asset_name(3).unwrap(), "assets/figure-3.png");
        for bad in [
            "![x](x.png){width=NaN}",
            "![x](x.png){width=101%}",
            "![x](x.png){page=0}",
            "![x](x.png){#bad\\input}",
            "![x](x.png){width=1 width=1}",
            "![x](https://a/b.png)",
            "![x](x.png){placement=H}",
            "![x](x.png) garbage",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn missing_and_corrupt_assets() {
        let dir = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let doc = crate::parse("![caption](missing.png)").unwrap();
        assert!(stage(&doc, dir.path(), dest.path()).is_err());
        fs::write(dir.path().join("missing.png"), b"broken").unwrap();
        assert!(stage(&doc, dir.path(), dest.path()).is_err());
        let image = image::RgbImage::new(20, 10);
        image.save(dir.path().join("missing.png")).unwrap();
        stage(&doc, dir.path(), dest.path()).unwrap();
        assert!(dest.path().join("assets/figure-0.png").is_file());
    }
    #[test]
    fn invalid_pdf_pages_and_failed_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("vector.pdf"),
            include_bytes!("../examples/imagens/figuras/vetores/diagrama.pdf"),
        )
        .unwrap();
        let valid = crate::parse("![Vector](vector.pdf){page=2}").unwrap();
        stage(&valid, dir.path(), dest.path()).unwrap();
        let invalid = crate::parse("![Vector](vector.pdf){page=3}").unwrap();
        assert!(stage(&invalid, dir.path(), dest.path()).is_err());
        let input = dir.path().join("document.cb");
        let output = dir.path().join("result.pdf");
        fs::write(&input, "![Missing](not-found.png)").unwrap();
        fs::write(&output, b"original").unwrap();
        let cli = crate::Cli {
            input: input.clone(),
            output: Some(output.clone()),
            font_dir: None,
            backend: crate::Backend::Latex,
            tectonic: None,
            mermaid_cli: None,
            paper: crate::Paper::Letter,
        };
        assert!(crate::run(cli).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"original");
        fs::write(&input, "![Vector](vector.pdf)").unwrap();
        let original = fs::read(dir.path().join("vector.pdf")).unwrap();
        let cli = crate::Cli {
            input,
            output: Some(dir.path().join("vector.pdf")),
            font_dir: None,
            backend: crate::Backend::Latex,
            tectonic: None,
            mermaid_cli: None,
            paper: crate::Paper::Letter,
        };
        assert!(crate::run(cli).is_err());
        assert_eq!(fs::read(dir.path().join("vector.pdf")).unwrap(), original);
    }
}
