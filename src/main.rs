use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, ValueEnum};
mod figures;
mod latex;
mod mermaid;
use printpdf::{
    IndirectFontRef, Line, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference, Point,
};
use std::{
    fs,
    io::{BufWriter, Cursor, Write},
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(version, about = "Compile Carbon markup with LaTeX typography")]
struct Cli {
    input: PathBuf,
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Directory containing regular.ttf, bold.ttf, italic.ttf, bold-italic.ttf, mono.ttf
    #[arg(long)]
    font_dir: Option<PathBuf>,
    /// LaTeX composition (default) or the standalone printpdf renderer
    #[arg(long, value_enum, default_value_t = Backend::Latex)]
    backend: Backend,
    /// Tectonic executable; otherwise CARBON_TECTONIC, local tools, or PATH
    #[arg(long)]
    tectonic: Option<PathBuf>,
    /// Mermaid CLI executable or cli.js entry point (only needed for diagrams)
    #[arg(long)]
    mermaid_cli: Option<PathBuf>,
    /// Paper size; the reference PDF uses Letter
    #[arg(long, value_enum, default_value_t = Paper::Letter)]
    paper: Paper,
}
#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
enum Backend {
    Latex,
    Native,
}
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Paper {
    Letter,
    A4,
}
#[derive(Debug, Default, PartialEq)]
struct Document {
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    blocks: Vec<Block>,
}
#[derive(Debug, PartialEq)]
enum Block {
    Figure(figures::Figure),
    Mermaid {
        text: String,
        figure: figures::Figure,
    },
    Heading(u8, String),
    Paragraph(String),
    Equation(String),
    Code {
        language: String,
        text: String,
    },
    Table {
        caption: Option<String>,
        layout: Option<TableLayout>,
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}
impl Block {
    fn figure(&self) -> Option<&figures::Figure> {
        match self {
            Self::Figure(fig) | Self::Mermaid { figure: fig, .. } => Some(fig),
            _ => None,
        }
    }
}
#[derive(Debug, PartialEq)]
struct TableLayout {
    width: f32,
    columns: Vec<Option<f32>>,
    align: String,
}
fn table_layout(text: &str) -> Result<TableLayout> {
    let mut layout = TableLayout {
        width: 1.,
        columns: Vec::new(),
        align: String::new(),
    };
    let mut keys = std::collections::HashSet::new();
    for item in text.split_whitespace() {
        let (key, value) = item
            .split_once('=')
            .context("@table expects width=1 columns=0.48,* align=lc")?;
        ensure!(keys.insert(key), "duplicate @table option {key}");
        match key {
            "width" => layout.width = value.parse().context("invalid table width")?,
            "columns" => {
                layout.columns = value
                    .split(',')
                    .map(|v| {
                        if v == "*" {
                            Ok(None)
                        } else {
                            Ok(Some(v.parse::<f32>().context("invalid column width")?))
                        }
                    })
                    .collect::<Result<_>>()?
            }
            "align" => layout.align = value.into(),
            _ => bail!("unknown @table option {key}"),
        }
    }
    ensure!(
        layout.width.is_finite() && layout.width > 0. && layout.width <= 1.,
        "table width must be in (0,1]"
    );
    ensure!(
        !layout.columns.is_empty() && layout.columns.len() <= 12,
        "@table requires 1..12 columns"
    );
    ensure!(
        layout
            .columns
            .iter()
            .flatten()
            .all(|v| v.is_finite() && *v > 0. && *v < 1.),
        "column fractions must be in (0,1)"
    );
    ensure!(
        layout.columns.iter().filter(|v| v.is_none()).count() == 1,
        "@table requires one flexible * column"
    );
    ensure!(
        layout.align.len() == layout.columns.len()
            && layout.align.chars().all(|c| "lcr".contains(c)),
        "align must have one l/c/r per column"
    );
    let fixed: f32 = layout.columns.iter().flatten().sum();
    ensure!(
        fixed + layout.columns.len() as f32 * 12. / 498.6 < layout.width,
        "column widths and padding exceed table width"
    );
    Ok(layout)
}
fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|s| s.trim().to_owned())
        .collect()
}
fn separator(s: &str) -> bool {
    s.contains('|')
        && cells(s).iter().all(|c| {
            let c = c.trim_matches(':');
            c.len() >= 3 && c.chars().all(|x| x == '-')
        })
}
fn heading(s: &str) -> Option<(u8, &str)> {
    for n in (1..=3).rev() {
        let prefix = format!("{} ", "#".repeat(n));
        if let Some(text) = s.strip_prefix(&prefix) {
            return Some((n as u8, text.trim()));
        }
    }
    None
}
fn special(lines: &[&str], i: usize) -> bool {
    let s = lines[i];
    s.trim().is_empty()
        || heading(s).is_some()
        || s.starts_with("```")
        || s.starts_with("$$")
        || s.starts_with("![")
        || s.starts_with("    ")
        || s.starts_with('\t')
        || (i + 1 < lines.len() && separator(lines[i + 1]))
}
fn parse(source: &str) -> Result<Document> {
    ensure!(
        !source
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')),
        "input contains unsupported control characters"
    );
    let source = source.trim_start_matches('\u{feff}');
    let lines: Vec<_> = source.lines().collect();
    let mut doc = Document::default();
    let mut i = 0;
    let mut body = false;
    let mut caption = None;
    let mut layout = None;
    while i < lines.len() {
        let s = lines[i];
        if s.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(value) = s.strip_prefix("@caption ") {
            ensure!(
                caption.is_none() && !value.trim().is_empty(),
                "line {}: invalid or duplicate caption",
                i + 1
            );
            caption = Some(value.trim().to_owned());
            body = true;
            i += 1;
            continue;
        }
        if let Some(value) = s.strip_prefix("@table ") {
            ensure!(layout.is_none(), "line {}: duplicate @table", i + 1);
            layout = Some(table_layout(value).with_context(|| format!("line {}", i + 1))?);
            body = true;
            i += 1;
            continue;
        }
        if layout.is_some() || (caption.is_some() && !s.starts_with("![") && !mermaid::is_fence(s))
        {
            ensure!(
                i + 1 < lines.len() && separator(lines[i + 1]),
                "line {}: @caption/@table must be followed by a figure/table as appropriate",
                i + 1
            );
        }
        if s.starts_with('@') && figures::reference_at(s)?.is_none() {
            ensure!(
                !body,
                "line {}: metadata must precede the document body",
                i + 1
            );
            let (key, value) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
            ensure!(
                !value.trim().is_empty(),
                "line {}: empty metadata value",
                i + 1
            );
            let field = match key {
                "@title" => &mut doc.title,
                "@author" => &mut doc.author,
                "@date" => &mut doc.date,
                _ => bail!("line {}: unknown metadata {key}", i + 1),
            };
            ensure!(field.is_none(), "line {}: duplicate metadata {key}", i + 1);
            *field = Some(value.trim().to_owned());
            i += 1;
            continue;
        }
        body = true;
        if s.starts_with("![") {
            let mut figure = figures::parse(s).with_context(|| format!("line {}", i + 1))?;
            if let Some(caption) = caption.take() {
                figure.caption = caption;
            }
            ensure!(
                !figure.caption.trim().is_empty(),
                "line {}: figure requires a caption or @caption",
                i + 1
            );
            doc.blocks.push(Block::Figure(figure));
            i += 1;
        } else if let Some((level, text)) = heading(s) {
            ensure!(!text.is_empty(), "line {}: empty heading", i + 1);
            doc.blocks.push(Block::Heading(level, text.to_owned()));
            i += 1;
        } else if let Some(language) = s.strip_prefix("```") {
            let start = i;
            i += 1;
            let mut code = Vec::new();
            while i < lines.len() && lines[i].trim() != "```" {
                code.push(lines[i]);
                i += 1;
            }
            ensure!(i < lines.len(), "line {}: unclosed code fence", start + 1);
            if mermaid::is_fence(s) {
                let mut figure = mermaid::figure(language)?;
                if let Some(value) = caption.take() {
                    figure.caption = value;
                }
                let text = code.join("\n");
                mermaid::validate(&text)
                    .with_context(|| format!("line {}: invalid Mermaid block", start + 1))?;
                doc.blocks.push(Block::Mermaid { text, figure });
            } else {
                doc.blocks.push(Block::Code {
                    language: language.trim().to_owned(),
                    text: code.join("\n"),
                });
            }
            i += 1;
        } else if let Some(rest) = s.strip_prefix("$$") {
            let start = i;
            let equation = if let Some(text) = rest.strip_suffix("$$") {
                i += 1;
                text.to_owned()
            } else {
                let mut parts = vec![rest];
                i += 1;
                while i < lines.len() && lines[i].trim() != "$$" {
                    parts.push(lines[i]);
                    i += 1;
                }
                ensure!(i < lines.len(), "line {}: unclosed equation", start + 1);
                i += 1;
                parts.join("\n")
            };
            ensure!(
                !equation.trim().is_empty(),
                "line {}: empty equation",
                start + 1
            );
            doc.blocks.push(Block::Equation(equation.trim().to_owned()));
        } else if s.starts_with("    ") || s.starts_with('\t') {
            let mut code = Vec::new();
            while i < lines.len() {
                if let Some(v) = lines[i]
                    .strip_prefix("    ")
                    .or_else(|| lines[i].strip_prefix('\t'))
                {
                    code.push(v);
                    i += 1;
                } else {
                    break;
                }
            }
            doc.blocks.push(Block::Code {
                language: String::new(),
                text: code.join("\n"),
            });
        } else if i + 1 < lines.len() && separator(lines[i + 1]) {
            let header = cells(s);
            if let Some(options) = &layout {
                ensure!(
                    options.columns.len() == header.len(),
                    "line {}: @table column count differs from header",
                    i + 1
                );
            }
            ensure!(
                header.len() == cells(lines[i + 1]).len(),
                "line {}: table separator has wrong column count",
                i + 2
            );
            ensure!(
                header.len() <= 12,
                "line {}: tables support at most 12 columns",
                i + 1
            );
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].contains('|') && !lines[i].trim().is_empty() {
                let row = cells(lines[i]);
                ensure!(
                    row.len() == header.len(),
                    "line {}: expected {} table columns, got {}",
                    i + 1,
                    header.len(),
                    row.len()
                );
                rows.push(row);
                i += 1;
            }
            doc.blocks.push(Block::Table {
                caption: caption.take(),
                layout: layout.take(),
                header,
                rows,
            });
        } else {
            let mut text = vec![s.trim()];
            i += 1;
            while i < lines.len()
                && !special(&lines, i)
                && (!lines[i].starts_with('@') || figures::reference_at(lines[i])?.is_some())
            {
                text.push(lines[i].trim());
                i += 1;
            }
            doc.blocks.push(Block::Paragraph(text.join(" ")));
        }
    }
    ensure!(caption.is_none(), "@caption has no following figure/table");
    ensure!(layout.is_none(), "@table has no following table");
    ensure!(
        doc.title.is_some() || !doc.blocks.is_empty(),
        "document is empty"
    );
    Ok(doc)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    mono: bool,
}
impl Style {
    fn index(self) -> usize {
        if self.mono {
            4
        } else {
            self.bold as usize + 2 * self.italic as usize
        }
    }
}
#[derive(Clone, Debug)]
struct Run {
    text: String,
    style: Style,
}
fn inline(text: &str, base: Style) -> Vec<Run> {
    fn scan(text: &str, style: Style, out: &mut Vec<Run>, depth: usize) {
        if depth > 32 {
            out.push(Run {
                text: text.into(),
                style,
            });
            return;
        }
        let mut pos = 0;
        let mut plain = String::new();
        while pos < text.len() {
            let rest = &text[pos..];
            if let Some(escaped) = rest.strip_prefix('\\') {
                if let Some(c) = escaped.chars().next().filter(|c| "*$\\".contains(*c)) {
                    plain.push(c);
                    pos += 1 + c.len_utf8();
                    continue;
                }
            }
            let marker = if rest.starts_with("***") {
                "***"
            } else if rest.starts_with("**") {
                "**"
            } else if rest.starts_with('*') {
                "*"
            } else if rest.starts_with('$') {
                "$"
            } else {
                ""
            };
            if !marker.is_empty() {
                let tail = &rest[marker.len()..];
                let end = tail
                    .match_indices(marker)
                    .find(|(n, _)| {
                        let backslashes =
                            tail[..*n].chars().rev().take_while(|c| *c == '\\').count();
                        backslashes % 2 == 0 && *n > 0
                    })
                    .map(|(n, _)| {
                        // In **bold *italic*** the first of the final three stars
                        // belongs to the nested italic span.
                        if marker == "**" && tail[n..].starts_with("***") {
                            n + 1
                        } else {
                            n
                        }
                    });
                if let Some(end) = end {
                    if !plain.is_empty() {
                        out.push(Run {
                            text: std::mem::take(&mut plain),
                            style,
                        });
                    }
                    let mut next = style;
                    match marker {
                        "***" => {
                            next.bold = !next.bold;
                            next.italic = !next.italic;
                        }
                        "**" => next.bold = !next.bold,
                        _ => next.italic = true,
                    }
                    if marker == "$" {
                        out.push(Run {
                            text: tail[..end].into(),
                            style: next,
                        });
                    } else {
                        scan(&tail[..end], next, out, depth + 1);
                    }
                    pos += marker.len() * 2 + end;
                    continue;
                }
            }
            let c = rest.chars().next().expect("nonempty UTF-8 suffix");
            plain.push(c);
            pos += c.len_utf8();
        }
        if !plain.is_empty() {
            out.push(Run { text: plain, style });
        }
    }
    let mut runs = Vec::new();
    scan(text, base, &mut runs, 0);
    runs
}
struct Fonts {
    data: Vec<Vec<u8>>,
    refs: Vec<IndirectFontRef>,
}
impl Fonts {
    fn load(pdf: &PdfDocumentReference, custom: Option<&Path>) -> Result<Self> {
        let candidates: Vec<Vec<PathBuf>> = if let Some(dir) = custom {
            vec![[
                "regular.ttf",
                "bold.ttf",
                "italic.ttf",
                "bold-italic.ttf",
                "mono.ttf",
            ]
            .iter()
            .map(|n| dir.join(n))
            .collect()]
        } else {
            vec![
                [
                    "times.ttf",
                    "timesbd.ttf",
                    "timesi.ttf",
                    "timesbi.ttf",
                    "cour.ttf",
                ]
                .iter()
                .map(|n| {
                    PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:/Windows".into()))
                        .join("Fonts")
                        .join(n)
                })
                .collect(),
                [
                    "LiberationSerif-Regular.ttf",
                    "LiberationSerif-Bold.ttf",
                    "LiberationSerif-Italic.ttf",
                    "LiberationSerif-BoldItalic.ttf",
                    "LiberationMono-Regular.ttf",
                ]
                .iter()
                .map(|n| PathBuf::from("/usr/share/fonts/truetype/liberation2").join(n))
                .collect(),
                [
                    "Times New Roman.ttf",
                    "Times New Roman Bold.ttf",
                    "Times New Roman Italic.ttf",
                    "Times New Roman Bold Italic.ttf",
                    "Courier New.ttf",
                ]
                .iter()
                .map(|n| PathBuf::from("/System/Library/Fonts/Supplemental").join(n))
                .collect(),
            ]
        };
        let paths = candidates.into_iter().find(|set| set.iter().all(|p| p.is_file())).context("fonts not found; use --font-dir with regular.ttf, bold.ttf, italic.ttf, bold-italic.ttf, mono.ttf")?;
        let mut data = Vec::new();
        let mut refs = Vec::new();
        for path in paths {
            let bytes =
                fs::read(&path).with_context(|| format!("cannot read font {}", path.display()))?;
            let face = ttf_parser::Face::parse(&bytes, 0)
                .map_err(|e| anyhow::anyhow!("invalid font {}: {e:?}", path.display()))?;
            ensure!(
                face.is_regular() || face.number_of_glyphs() > 0,
                "font has no glyphs"
            );
            refs.push(
                pdf.add_external_font(Cursor::new(&bytes))
                    .context("cannot embed font")?,
            );
            data.push(bytes);
        }
        Ok(Self { data, refs })
    }
    fn width(&self, c: char, style: Style, size: f32) -> Result<f32> {
        let face = ttf_parser::Face::parse(&self.data[style.index()], 0)
            .map_err(|e| anyhow::anyhow!("font parse: {e:?}"))?;
        let glyph = face.glyph_index(c).with_context(|| {
            format!(
                "font lacks glyph {c:?} (U+{:04X}); choose another --font-dir",
                c as u32
            )
        })?;
        let advance = face
            .glyph_hor_advance(glyph)
            .context("font lacks horizontal metrics")?;
        Ok(advance as f32 / face.units_per_em() as f32 * size)
    }
}
#[derive(Clone, Debug)]
struct Glyph {
    c: char,
    style: Style,
    width: f32,
}
type TextLine = Vec<Glyph>;
fn wrap(
    fonts: &Fonts,
    runs: Vec<Run>,
    size: f32,
    width: f32,
    literal: bool,
) -> Result<Vec<TextLine>> {
    let mut result = Vec::new();
    let mut line = Vec::new();
    let mut used = 0.;
    // Buffer whole words across style boundaries, then split only words wider than a column.
    let mut word: Vec<Glyph> = Vec::new();
    let flush = |word: &mut Vec<Glyph>,
                 line: &mut Vec<Glyph>,
                 used: &mut f32,
                 result: &mut Vec<TextLine>| {
        let w: f32 = word.iter().map(|g| g.width).sum();
        if *used + w > width && !line.is_empty() {
            while line.last().is_some_and(|g| g.c == ' ') {
                line.pop();
            }
            result.push(std::mem::take(line));
            *used = 0.;
        }
        for glyph in word.drain(..) {
            if *used + glyph.width > width && !line.is_empty() {
                result.push(std::mem::take(line));
                *used = 0.;
            }
            *used += glyph.width;
            line.push(glyph);
        }
    };
    for run in runs {
        for c in run.text.replace('\t', "    ").chars() {
            if c == '\n' {
                flush(&mut word, &mut line, &mut used, &mut result);
                result.push(std::mem::take(&mut line));
                used = 0.;
                continue;
            }
            let c = if c.is_whitespace() { ' ' } else { c };
            let glyph = Glyph {
                c,
                style: run.style,
                width: fonts.width(c, run.style, size)?,
            };
            ensure!(glyph.width <= width, "column too narrow for glyph {c:?}");
            if literal {
                if used + glyph.width > width && !line.is_empty() {
                    result.push(std::mem::take(&mut line));
                    used = 0.;
                }
                used += glyph.width;
                line.push(glyph);
            } else if c == ' ' {
                flush(&mut word, &mut line, &mut used, &mut result);
                if !line.is_empty() && line.last().is_none_or(|g| g.c != ' ') {
                    used += glyph.width;
                    line.push(glyph);
                }
            } else {
                word.push(glyph);
            }
        }
    }
    flush(&mut word, &mut line, &mut used, &mut result);
    if !line.is_empty() || result.is_empty() {
        result.push(line);
    }
    Ok(result)
}
const PAGE_W: f32 = 595.2756;
const PAGE_H: f32 = 841.8898;
const MARGIN: f32 = 56.6929;
const BOTTOM: f32 = PAGE_H - MARGIN;
const WIDTH: f32 = PAGE_W - MARGIN * 2.;
fn mm(pt: f32) -> Mm {
    Mm(pt * 25.4 / 72.)
}
struct Renderer {
    pdf: PdfDocumentReference,
    layer: PdfLayerReference,
    fonts: Fonts,
    y: f32,
    pages: usize,
}
impl Renderer {
    fn new(title: &str, font_dir: Option<&Path>) -> Result<Self> {
        let (pdf, page, layer) = PdfDocument::new(title, Mm(210.), Mm(297.), "Carbon");
        let fonts = Fonts::load(&pdf, font_dir)?;
        let layer = pdf.get_page(page).get_layer(layer);
        let r = Self {
            pdf,
            layer,
            fonts,
            y: MARGIN,
            pages: 1,
        };
        r.footer()?;
        Ok(r)
    }
    fn footer(&self) -> Result<()> {
        let text = self.pages.to_string();
        let style = Style::default();
        let w: f32 = text
            .chars()
            .map(|c| self.fonts.width(c, style, 9.))
            .collect::<Result<Vec<_>>>()?
            .iter()
            .sum();
        self.layer.use_text(
            text,
            9.,
            mm((PAGE_W - w) / 2.),
            mm(28.),
            &self.fonts.refs[0],
        );
        Ok(())
    }
    fn next_page(&mut self) -> Result<()> {
        let (page, layer) = self.pdf.add_page(Mm(210.), Mm(297.), "Carbon");
        self.layer = self.pdf.get_page(page).get_layer(layer);
        self.y = MARGIN;
        self.pages += 1;
        self.footer()
    }
    fn ensure_space(&mut self, height: f32) -> Result<()> {
        ensure!(height <= BOTTOM - MARGIN, "layout unit exceeds page height");
        if self.y + height > BOTTOM {
            self.next_page()?;
        }
        Ok(())
    }
    fn draw(&self, line: &TextLine, x: f32, y: f32, size: f32) {
        let mut x = x;
        let mut i = 0;
        while i < line.len() {
            let style = line[i].style;
            let start = i;
            let mut text = String::new();
            while i < line.len() && line[i].style == style {
                text.push(line[i].c);
                i += 1;
            }
            self.layer.use_text(
                text,
                size,
                mm(x),
                mm(PAGE_H - y - size),
                &self.fonts.refs[style.index()],
            );
            x += line[start..i].iter().map(|g| g.width).sum::<f32>();
        }
    }
    fn text(
        &mut self,
        runs: Vec<Run>,
        size: f32,
        leading: f32,
        center: bool,
        literal: bool,
    ) -> Result<()> {
        let lines = wrap(&self.fonts, runs, size, WIDTH, literal)?;
        let count = lines.len();
        for (i, line) in lines.into_iter().enumerate() {
            // Avoid a single first/last paragraph line at a page boundary.
            let reserve = if (i == 0 && count > 1) || count - i == 2 {
                leading * 2.
            } else {
                leading
            };
            self.ensure_space(reserve)?;
            let width: f32 = line.iter().map(|g| g.width).sum();
            self.draw(
                &line,
                MARGIN + if center { (WIDTH - width) / 2. } else { 0. },
                self.y,
                size,
            );
            self.y += leading;
        }
        Ok(())
    }
    fn rule(&self, y: f32, thickness: f32) {
        self.layer.set_outline_thickness(thickness);
        self.layer.add_line(Line {
            points: vec![
                (Point::new(mm(MARGIN), mm(PAGE_H - y)), false),
                (Point::new(mm(PAGE_W - MARGIN), mm(PAGE_H - y)), false),
            ],
            is_closed: false,
        });
    }
    fn table(&mut self, header: &[String], rows: &[Vec<String>]) -> Result<()> {
        let col = WIDTH / header.len() as f32;
        let leading = 13.;
        let prepare = |cells: &[String], bold: bool| -> Result<Vec<Vec<TextLine>>> {
            cells
                .iter()
                .map(|s| {
                    wrap(
                        &self.fonts,
                        inline(
                            s,
                            Style {
                                bold,
                                ..Style::default()
                            },
                        ),
                        10.,
                        col - 12.,
                        false,
                    )
                })
                .collect()
        };
        let head = prepare(header, true)?;
        let body = rows
            .iter()
            .map(|r| prepare(r, false))
            .collect::<Result<Vec<_>>>()?;
        let height = |r: &Vec<Vec<TextLine>>| {
            r.iter().map(Vec::len).max().unwrap_or(1) as f32 * leading + 12.
        };
        let hh = height(&head);
        ensure!(
            hh + leading + 12. <= BOTTOM - MARGIN,
            "table header too tall"
        );
        for row in &body {
            ensure!(
                height(row) + hh <= BOTTOM - MARGIN,
                "table row too tall; split the row in the source"
            );
        }
        self.ensure_space(hh + body.first().map(height).unwrap_or(0.))?;
        self.rule(self.y, 0.8);
        self.table_row(&head, leading);
        self.rule(self.y, 0.4);
        for row in &body {
            if self.y + height(row) > BOTTOM {
                self.rule(self.y, 0.8);
                self.next_page()?;
                self.rule(self.y, 0.8);
                self.table_row(&head, leading);
                self.rule(self.y, 0.4);
            }
            self.table_row(row, leading);
        }
        self.rule(self.y, 0.8);
        self.y += 12.;
        Ok(())
    }
    fn table_row(&mut self, row: &[Vec<TextLine>], leading: f32) {
        let count = row.iter().map(Vec::len).max().unwrap_or(1);
        for (column, lines) in row.iter().enumerate() {
            for (n, line) in lines.iter().enumerate() {
                self.draw(
                    line,
                    MARGIN + column as f32 * WIDTH / row.len() as f32 + 6.,
                    self.y + 6. + n as f32 * leading,
                    10.,
                );
            }
        }
        self.y += count as f32 * leading + 12.;
    }
    fn render(mut self, doc: &Document) -> Result<(Vec<u8>, usize)> {
        if let Some(title) = &doc.title {
            self.text(
                inline(
                    title,
                    Style {
                        bold: true,
                        ..Style::default()
                    },
                ),
                23.,
                29.,
                true,
                false,
            )?;
            self.y += 10.;
        }
        for value in [&doc.author, &doc.date].into_iter().flatten() {
            self.text(inline(value, Style::default()), 11., 16., true, false)?;
        }
        if doc.title.is_some() || doc.author.is_some() || doc.date.is_some() {
            self.y += 20.;
        }
        for block in &doc.blocks {
            match block {
                Block::Figure(_) | Block::Mermaid { .. } => {
                    bail!("figures and Mermaid diagrams require --backend latex")
                }
                Block::Heading(level, text) => {
                    let size = match level {
                        1 => 18.,
                        2 => 15.,
                        _ => 12.5,
                    };
                    let lines = wrap(
                        &self.fonts,
                        inline(
                            text,
                            Style {
                                bold: true,
                                ..Style::default()
                            },
                        ),
                        size,
                        WIDTH,
                        false,
                    )?;
                    let h = lines.len() as f32 * size * 1.3;
                    ensure!(h + 44. <= BOTTOM - MARGIN, "heading too tall");
                    self.ensure_space(h + 44.)?;
                    self.y += 12.;
                    for line in lines {
                        self.draw(&line, MARGIN, self.y, size);
                        self.y += size * 1.3;
                    }
                    self.y += 7.;
                }
                Block::Paragraph(text) => {
                    self.text(inline(text, Style::default()), 11., 16., false, false)?;
                    self.y += 8.;
                }
                Block::Equation(text) => {
                    self.y += 8.;
                    self.text(
                        vec![Run {
                            text: text.clone(),
                            style: Style {
                                italic: true,
                                ..Style::default()
                            },
                        }],
                        12.,
                        18.,
                        true,
                        true,
                    )?;
                    self.y += 12.;
                }
                Block::Code { language, text } => {
                    self.ensure_space(42.)?;
                    if !language.is_empty() {
                        self.text(
                            vec![Run {
                                text: language.clone(),
                                style: Style {
                                    bold: true,
                                    ..Style::default()
                                },
                            }],
                            9.,
                            13.,
                            false,
                            true,
                        )?;
                    }
                    self.text(
                        vec![Run {
                            text: text.clone(),
                            style: Style {
                                mono: true,
                                ..Style::default()
                            },
                        }],
                        9.,
                        13.,
                        false,
                        true,
                    )?;
                    self.y += 12.;
                }
                Block::Table {
                    caption,
                    layout,
                    header,
                    rows,
                } => {
                    ensure!(
                        layout.is_none(),
                        "@table layout options require --backend latex"
                    );
                    if let Some(caption) = caption {
                        self.ensure_space(60.)?;
                        self.text(inline(caption, Style::default()), 10., 14., true, false)?;
                    }
                    self.table(header, rows)?;
                }
            }
        }
        let bytes = self
            .pdf
            .save_to_bytes()
            .context("PDF serialization failed")?;
        // printpdf 0.7 writes UTF-8 into PDF strings. PDF metadata requires
        // PDFDocEncoding or BOM-prefixed UTF-16BE instead.
        let mut pdf = printpdf::lopdf::Document::load_mem(&bytes)?;
        let info_id = pdf.trailer.get(b"Info")?.as_reference()?;
        let info = pdf.get_object_mut(info_id)?.as_dict_mut()?;
        let pdf_string = |s: &str| {
            let mut bytes = vec![0xFE, 0xFF];
            for unit in s.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            printpdf::lopdf::Object::String(bytes, printpdf::lopdf::StringFormat::Hexadecimal)
        };
        info.set(
            "Title",
            pdf_string(doc.title.as_deref().unwrap_or("Carbon document")),
        );
        if let Some(author) = &doc.author {
            info.set("Author", pdf_string(author));
        }
        if let Some(date) = &doc.date {
            info.set("CarbonDate", pdf_string(date));
        }
        info.set(
            "Creator",
            pdf_string(concat!("Carbon ", env!("CARGO_PKG_VERSION"))),
        );
        // This compiler does not claim certified archival/prepress conformance.
        info.remove(b"GTS_PDFXVersion");
        info.remove(b"GTS_PDFXConformance");
        pdf.compress();
        let mut output = Vec::new();
        pdf.save_to(&mut output)?;
        Ok((output, self.pages))
    }
}
fn run(cli: Cli) -> Result<()> {
    let output = cli
        .output
        .unwrap_or_else(|| cli.input.with_extension("pdf"));
    ensure!(output != cli.input, "input and output must differ");
    if output.exists() {
        ensure!(
            fs::canonicalize(&output)? != fs::canonicalize(&cli.input)?,
            "input and output resolve to the same file"
        );
    }
    let source = fs::read_to_string(&cli.input)
        .with_context(|| format!("cannot read UTF-8 input {}", cli.input.display()))?;
    let doc = parse(&source).context("Carbon parsing failed")?;
    let input = fs::canonicalize(&cli.input)?;
    let base = input.parent().context("input has no parent directory")?;
    if output.exists() {
        let destination = fs::canonicalize(&output)?;
        for block in &doc.blocks {
            if let Block::Figure(fig) = block {
                if let Ok(asset) = fs::canonicalize(base.join(&fig.path)) {
                    ensure!(
                        asset != destination,
                        "output must not overwrite a figure asset: {}",
                        fig.path.display()
                    );
                }
            }
        }
    }
    let title = doc.title.as_deref().unwrap_or("Carbon document");
    let (bytes, pages) = match cli.backend {
        Backend::Latex => {
            ensure!(
                cli.font_dir.is_none(),
                "--font-dir is only supported by --backend native"
            );
            latex::render_with_mermaid(
                &doc,
                cli.tectonic.as_deref(),
                cli.paper,
                base,
                cli.mermaid_cli.as_deref(),
            )?
        }
        Backend::Native => Renderer::new(title, cli.font_dir.as_deref())?.render(&doc)?,
    };
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).context("cannot create output temporary file")?;
    {
        let mut writer = BufWriter::new(temporary.as_file_mut());
        writer.write_all(&bytes)?;
        writer.flush()?;
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist(&output)
        .map_err(|e| anyhow::anyhow!("cannot save {}: {}", output.display(), e.error))?;
    println!("{} ({pages} pages)", output.display());
    Ok(())
}
fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("carbon: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_scientific_blocks() {
        let d = parse("@title Ciência\n@author Ana\n@date 2026\n\n# Intro\nTexto **forte** e $x$.\n\n$$\nE=mc^2\n$$\n```rust\n  let x = 1;\n```\n\n| A | B |\n| --- | :---: |\n| 1 | 2 |\n").unwrap();
        assert_eq!(d.blocks.len(), 5);
        assert!(matches!(&d.blocks[2], Block::Equation(s) if s == "E=mc^2"));
        assert!(matches!(&d.blocks[3], Block::Code {text, ..} if text.starts_with("  let")));
    }
    #[test]
    fn diagnostics() {
        for s in [
            "",
            "```rust\nx",
            "$$\nx",
            "@title A\n@title B",
            "# A\n@author B",
            "| A | B |\n| --- | --- |\n| x |",
            "@unknown A",
        ] {
            assert!(parse(s).is_err(), "{s}");
        }
    }
    #[test]
    fn unicode_and_escapes() {
        let runs = inline(
            r"ação **forte *ênfase*** e \*literal\* $α + β$",
            Style::default(),
        );
        assert!(runs.iter().any(|r| r.style.bold));
        assert!(runs.iter().any(|r| r.style.italic));
        assert!(runs
            .iter()
            .any(|r| r.text == "ênfase" && r.style.bold && r.style.italic));
        assert!(runs.iter().any(|r| r.text.contains("literal")));
        assert_eq!(inline("**aberto", Style::default())[0].text, "**aberto");
    }
    #[test]
    fn bom_crlf_and_indented_code() {
        let d = parse("\u{feff}@title T\r\n\r\n    a\r\n    b\r\n").unwrap();
        assert_eq!(
            d.blocks,
            vec![Block::Code {
                language: String::new(),
                text: "a\nb".into()
            }]
        );
    }

    #[test]
    #[ignore = "requires the documented system TrueType fonts"]
    fn measured_layout_and_pagination() {
        let renderer = Renderer::new("Layout test", None).unwrap();
        let text = format!("ação **negrito** {}", "x".repeat(300));
        let lines = wrap(
            &renderer.fonts,
            inline(&text, Style::default()),
            11.,
            80.,
            false,
        )
        .unwrap();
        assert!(lines.len() > 10);
        for line in &lines {
            assert!(line.iter().map(|g| g.width).sum::<f32>() <= 80.01);
        }
        let literal = wrap(
            &renderer.fonts,
            vec![Run {
                text: "    fn main() {}".into(),
                style: Style {
                    mono: true,
                    ..Style::default()
                },
            }],
            9.,
            WIDTH,
            true,
        )
        .unwrap();
        assert_eq!(
            literal[0].iter().take(4).map(|g| g.c).collect::<String>(),
            "    "
        );
        assert!(renderer
            .fonts
            .width('\u{10FFFF}', Style::default(), 11.)
            .is_err());
        let mut source = String::from("@title Pagination\n\n# Text\n");
        for _ in 0..80 {
            source.push_str("Paragraph **bold** with *italics*, ação and $α + β$.\n\n");
        }
        source.push_str("## Table\n| Name | Value |\n| --- | --- |\n");
        for _ in 0..100 {
            source.push_str("| Test | Value wrapped over multiple lines in its column |\n");
        }
        let (pdf, pages) = renderer.render(&parse(&source).unwrap()).unwrap();
        assert!(pages >= 5);
        assert!(pdf.starts_with(b"%PDF-"));
        let parsed = printpdf::lopdf::Document::load_mem(&pdf).unwrap();
        assert_eq!(parsed.get_pages().len(), pages);
        let info = parsed
            .get_dictionary(parsed.trailer.get(b"Info").unwrap().as_reference().unwrap())
            .unwrap();
        assert!(info
            .get(b"Title")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with(&[0xFE, 0xFF]));
        assert!(info.get(b"GTS_PDFXVersion").is_err());
    }

    #[test]
    #[ignore = "requires the documented system TrueType fonts"]
    fn failed_compilation_preserves_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("broken.cb");
        let output = dir.path().join("existing.pdf");
        fs::write(&input, "```rust\nunclosed").unwrap();
        fs::write(&output, b"original").unwrap();
        assert!(run(Cli {
            input,
            output: Some(output.clone()),
            font_dir: None,
            backend: Backend::Native,
            tectonic: None,
            mermaid_cli: None,
            paper: Paper::A4
        })
        .is_err());
        assert_eq!(fs::read(output).unwrap(), b"original");
    }
}
