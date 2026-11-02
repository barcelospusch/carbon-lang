//! Carbon AST -> LaTeX. Tectonic supplies TeX line breaking, Computer Modern,
//! hyphenation, and real mathematical composition instead of approximating them.
use super::{Block, Document, Paper};
use anyhow::{bail, ensure, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const GREEK: &[(&str, char)] = &[
    ("alpha", 'α'),
    ("beta", 'β'),
    ("gamma", 'γ'),
    ("delta", 'δ'),
    ("epsilon", 'ϵ'),
    ("varepsilon", 'ε'),
    ("zeta", 'ζ'),
    ("eta", 'η'),
    ("theta", 'θ'),
    ("vartheta", 'ϑ'),
    ("iota", 'ι'),
    ("kappa", 'κ'),
    ("varkappa", 'ϰ'),
    ("digamma", 'ϝ'),
    ("lambda", 'λ'),
    ("mu", 'μ'),
    ("nu", 'ν'),
    ("xi", 'ξ'),
    ("pi", 'π'),
    ("varpi", 'ϖ'),
    ("rho", 'ρ'),
    ("varrho", 'ϱ'),
    ("sigma", 'σ'),
    ("varsigma", 'ς'),
    ("tau", 'τ'),
    ("upsilon", 'υ'),
    ("phi", 'ϕ'),
    ("varphi", 'φ'),
    ("chi", 'χ'),
    ("psi", 'ψ'),
    ("omega", 'ω'),
    ("Gamma", 'Γ'),
    ("Delta", 'Δ'),
    ("Theta", 'Θ'),
    ("Lambda", 'Λ'),
    ("Xi", 'Ξ'),
    ("Pi", 'Π'),
    ("Sigma", 'Σ'),
    ("Upsilon", 'Υ'),
    ("Phi", 'Φ'),
    ("Psi", 'Ψ'),
    ("Omega", 'Ω'),
];
const COMMANDS: &[&str] = &[
    "frac",
    "dfrac",
    "tfrac",
    "sqrt",
    "sum",
    "prod",
    "int",
    "iint",
    "iiint",
    "oint",
    "infty",
    "partial",
    "nabla",
    "cdot",
    "times",
    "div",
    "pm",
    "mp",
    "le",
    "leq",
    "ge",
    "geq",
    "ne",
    "neq",
    "approx",
    "equiv",
    "sim",
    "simeq",
    "propto",
    "to",
    "rightarrow",
    "leftarrow",
    "Rightarrow",
    "Leftarrow",
    "leftrightarrow",
    "Leftrightarrow",
    "mapsto",
    "gets",
    "in",
    "notin",
    "subset",
    "subseteq",
    "supset",
    "supseteq",
    "cup",
    "cap",
    "emptyset",
    "forall",
    "exists",
    "neg",
    "land",
    "lor",
    "lnot",
    "wedge",
    "vee",
    "oplus",
    "otimes",
    "sin",
    "cos",
    "tan",
    "arcsin",
    "arccos",
    "arctan",
    "sinh",
    "cosh",
    "tanh",
    "log",
    "ln",
    "exp",
    "lim",
    "min",
    "max",
    "sup",
    "inf",
    "det",
    "gcd",
    "Pr",
    "arg",
    "hat",
    "widehat",
    "bar",
    "overline",
    "underline",
    "vec",
    "dot",
    "ddot",
    "tilde",
    "widetilde",
    "mathbf",
    "mathrm",
    "mathit",
    "mathsf",
    "mathtt",
    "mathcal",
    "mathbb",
    "boldsymbol",
    "text",
    "textrm",
    "textsf",
    "textit",
    "textbf",
    "operatorname",
    "left",
    "right",
    "middle",
    "big",
    "Big",
    "bigg",
    "Bigg",
    "bigl",
    "bigr",
    "Bigl",
    "Bigr",
    "biggl",
    "biggr",
    "langle",
    "rangle",
    "lvert",
    "rvert",
    "lVert",
    "rVert",
    "vert",
    "Vert",
    "lfloor",
    "rfloor",
    "lceil",
    "rceil",
    "ldots",
    "cdots",
    "vdots",
    "ddots",
    "dots",
    "quad",
    "qquad",
    "enspace",
    "enskip",
    "displaystyle",
    "textstyle",
    "scriptstyle",
    "scriptscriptstyle",
    "limits",
    "nolimits",
    "underbrace",
    "overbrace",
    "overset",
    "underset",
    "binom",
    "dbinom",
    "tbinom",
    "substack",
    "hbar",
    "ell",
    "Re",
    "Im",
    "aleph",
    "angle",
    "degree",
    "prime",
    "circ",
    "top",
    "bot",
    "shortparallel",
    "parallel",
    "perp",
    "mathring",
    "not",
    "colon",
    "backslash",
    "setminus",
];
fn escape(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        result.push_str(match c {
            '\\' => "\\textbackslash{}",
            '{' => "\\{",
            '}' => "\\}",
            '$' => "\\$",
            '&' => "\\&",
            '#' => "\\#",
            '%' => "\\%",
            '_' => "\\_",
            '^' => "\\textasciicircum{}",
            '~' => "\\textasciitilde{}",
            'á' => "\\'{a}",
            'à' => "\\`{a}",
            'â' => "\\^{a}",
            'ã' => "\\~{a}",
            'ä' => "\\\"{a}",
            'é' => "\\'{e}",
            'è' => "\\`{e}",
            'ê' => "\\^{e}",
            'ë' => "\\\"{e}",
            'í' => "\\'{i}",
            'ì' => "\\`{i}",
            'î' => "\\^{i}",
            'ï' => "\\\"{i}",
            'ó' => "\\'{o}",
            'ò' => "\\`{o}",
            'ô' => "\\^{o}",
            'õ' => "\\~{o}",
            'ö' => "\\\"{o}",
            'ú' => "\\'{u}",
            'ù' => "\\`{u}",
            'û' => "\\^{u}",
            'ü' => "\\\"{u}",
            'ç' => "\\c{c}",
            'ñ' => "\\~{n}",
            'Á' => "\\'{A}",
            'À' => "\\`{A}",
            'Â' => "\\^{A}",
            'Ã' => "\\~{A}",
            'Ä' => "\\\"{A}",
            'É' => "\\'{E}",
            'È' => "\\`{E}",
            'Ê' => "\\^{E}",
            'Ë' => "\\\"{E}",
            'Í' => "\\'{I}",
            'Ì' => "\\`{I}",
            'Î' => "\\^{I}",
            'Ï' => "\\\"{I}",
            'Ó' => "\\'{O}",
            'Ò' => "\\`{O}",
            'Ô' => "\\^{O}",
            'Õ' => "\\~{O}",
            'Ö' => "\\\"{O}",
            'Ú' => "\\'{U}",
            'Ù' => "\\`{U}",
            'Û' => "\\^{U}",
            'Ü' => "\\\"{U}",
            'Ç' => "\\c{C}",
            'Ñ' => "\\~{N}",
            '–' => "--",
            '—' => "---",
            '“' => "``",
            '”' => "''",
            '‘' => "`",
            '’' => "'",
            _ => {
                result.push(c);
                continue;
            }
        });
    }
    result
}
fn inline_tex(text: &str) -> Result<String> {
    fn scan(text: &str, depth: usize) -> Result<String> {
        ensure!(depth <= 32, "inline nesting exceeds 32 levels");
        let mut result = String::new();
        let mut i = 0;
        while i < text.len() {
            let tail = &text[i..];
            if let Some(after) = tail.strip_prefix('\\') {
                if let Some(c) = after.chars().next().filter(|c| "*$\\@".contains(*c)) {
                    result.push_str(&escape(&c.to_string()));
                    i += 1 + c.len_utf8();
                    continue;
                }
            }
            if let Some((command, id, consumed)) = crate::figures::reference_at(tail)? {
                result.push_str(&format!("\\{command}{{carbon:{id}}}"));
                i += consumed;
                continue;
            }
            let marker = ["***", "**", "*", "$"]
                .into_iter()
                .find(|m| tail.starts_with(m));
            if let Some(marker) = marker {
                let after = &tail[marker.len()..];
                let end = after
                    .match_indices(marker)
                    .find(|(n, _)| {
                        *n > 0
                            && after[..*n]
                                .bytes()
                                .rev()
                                .take_while(|b| *b == b'\\')
                                .count()
                                % 2
                                == 0
                    })
                    .map(|(n, _)| {
                        if marker == "**" && after[n..].starts_with("***") {
                            n + 1
                        } else {
                            n
                        }
                    });
                if let Some(end) = end {
                    let content = &after[..end];
                    let part = if marker == "$" {
                        format!("${}$", math(content)?)
                    } else {
                        let content = scan(content, depth + 1)?;
                        match marker {
                            "**" => format!("\\textbf{{{content}}}"),
                            "***" => format!("\\textbf{{\\textit{{{content}}}}}"),
                            _ => format!("\\textit{{{content}}}"),
                        }
                    };
                    result.push_str(&part);
                    i += marker.len() * 2 + end;
                    continue;
                }
            }
            let c = tail.chars().next().unwrap();
            result.push_str(&escape(&c.to_string()));
            i += c.len_utf8();
        }
        Ok(result)
    }
    scan(text, 0)
}
/// Accept a documented mathematical subset, never arbitrary TeX I/O/macros.
fn math(source: &str) -> Result<String> {
    let mut result = String::new();
    let mut chars = source.chars().peekable();
    let mut depth = 0usize;
    let mut environments: Vec<String> = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let mut name = String::new();
                while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    name.push(chars.next().unwrap());
                }
                if name.is_empty() {
                    let symbol = chars.next().context("dangling backslash in mathematics")?;
                    ensure!(
                        ",;:! {}|_&#%\\".contains(symbol),
                        "unsupported math escape \\{symbol}"
                    );
                    result.push('\\');
                    result.push(symbol);
                } else if name == "begin" || name == "end" {
                    while chars.peek().is_some_and(|c| c.is_whitespace()) {
                        chars.next();
                    }
                    ensure!(
                        chars.next() == Some('{'),
                        "expected environment name after \\{name}"
                    );
                    let mut env = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(c) => env.push(c),
                            None => bail!("unclosed math environment"),
                        }
                    }
                    ensure!(
                        [
                            "aligned",
                            "gathered",
                            "matrix",
                            "pmatrix",
                            "bmatrix",
                            "Bmatrix",
                            "vmatrix",
                            "Vmatrix",
                            "cases",
                            "smallmatrix"
                        ]
                        .contains(&env.as_str()),
                        "unsupported math environment {env}"
                    );
                    if name == "begin" {
                        environments.push(env.clone());
                    } else {
                        ensure!(
                            environments.pop().as_deref() == Some(env.as_str()),
                            "mismatched math environment {env}"
                        );
                    }
                    result.push_str(&format!("\\{name}{{{env}}}"));
                } else {
                    ensure!(
                        COMMANDS.contains(&name.as_str()) || GREEK.iter().any(|(n, _)| *n == name),
                        "unsupported math command \\{name}"
                    );
                    result.push('\\');
                    result.push_str(&name);
                    result.push(' ');
                }
            }
            '{' => {
                depth += 1;
                ensure!(depth <= 128, "math nesting exceeds 128 levels");
                result.push(c);
            }
            '}' => {
                ensure!(depth > 0, "unmatched closing brace in mathematics");
                depth -= 1;
                result.push(c);
            }
            '$' | '%' | '#' => bail!("escape {c:?} inside mathematics"),
            '&' => {
                ensure!(
                    !environments.is_empty(),
                    "alignment & requires an aligned/matrix/cases environment"
                );
                result.push(c);
            }
            c if c.is_ascii() => {
                ensure!(
                    !c.is_control() || c.is_whitespace(),
                    "control character in mathematics"
                );
                result.push(c);
            }
            c => {
                if let Some((name, _)) = GREEK.iter().find(|(_, greek)| *greek == c) {
                    result.push_str(&format!("\\{name} "));
                } else if let Some(name) = match c {
                    'Α' => Some("mathrm{A}"),
                    'Β' => Some("mathrm{B}"),
                    'Ε' => Some("mathrm{E}"),
                    'Ζ' => Some("mathrm{Z}"),
                    'Η' => Some("mathrm{H}"),
                    'Ι' => Some("mathrm{I}"),
                    'Κ' => Some("mathrm{K}"),
                    'Μ' => Some("mathrm{M}"),
                    'Ν' => Some("mathrm{N}"),
                    'Ο' => Some("mathrm{O}"),
                    'Ρ' => Some("mathrm{P}"),
                    'Τ' => Some("mathrm{T}"),
                    'Χ' => Some("mathrm{X}"),
                    'ο' => Some("mathrm{o}"),
                    '∞' => Some("infty"),
                    '∑' => Some("sum"),
                    '∫' => Some("int"),
                    '∂' => Some("partial"),
                    '∇' => Some("nabla"),
                    '±' => Some("pm"),
                    '×' => Some("times"),
                    '·' => Some("cdot"),
                    '≤' => Some("leq"),
                    '≥' => Some("geq"),
                    '≠' => Some("neq"),
                    '≈' => Some("approx"),
                    _ => None,
                } {
                    result.push_str(&format!("\\{name} "));
                } else {
                    result.push(c);
                }
            }
        }
    }
    ensure!(depth == 0, "unclosed brace in mathematics");
    ensure!(environments.is_empty(), "unclosed math environment");
    Ok(result)
}

pub(super) fn source(doc: &Document, paper: Paper) -> Result<String> {
    let mut ids = std::collections::HashSet::new();
    for block in &doc.blocks {
        if let Some(fig) = block.figure() {
            ensure!(
                !fig.caption.trim().is_empty(),
                "figure requires a caption: {}",
                fig.path.display()
            );
            if let Some(id) = &fig.id {
                ensure!(ids.insert(id.as_str()), "duplicate figure identifier {id}");
            }
        }
    }
    let inline_tex = |text: &str| -> Result<String> {
        let tex = inline_tex(text)?;
        for command in ["ref", "pageref", "autoref"] {
            let prefix = format!("\\{command}{{carbon:");
            for part in tex.split(&prefix).skip(1) {
                let id = part.split('}').next().unwrap();
                ensure!(ids.contains(id), "undefined figure reference {id}");
            }
        }
        Ok(tex)
    };
    let paper = match paper {
        Paper::Letter => "letterpaper",
        Paper::A4 => "a4paper",
    };
    let mut out = format!(
        r#"\documentclass[10pt,{paper}]{{article}}
\usepackage[margin=2cm]{{geometry}}
\usepackage[OT1]{{fontenc}}
\usepackage{{indentfirst}}
\renewcommand{{\rmdefault}}{{cmr}}
\renewcommand{{\sfdefault}}{{cmss}}
\renewcommand{{\ttdefault}}{{cmtt}}
\usepackage{{amsmath,amssymb,booktabs,longtable,array,tabularx}}
\usepackage{{listings}}
\usepackage{{graphicx}}
\usepackage[hidelinks]{{hyperref}}
\lstset{{basicstyle=\ttfamily\small,breaklines=true,keepspaces=true,columns=fullflexible,showstringspaces=false}}
\lstset{{literate={{á}}{{{{\'a}}}}1 {{à}}{{{{\`a}}}}1 {{â}}{{{{\^a}}}}1 {{ã}}{{{{\~a}}}}1 {{é}}{{{{\'e}}}}1 {{ê}}{{{{\^e}}}}1 {{í}}{{{{\'i}}}}1 {{ó}}{{{{\'o}}}}1 {{ô}}{{{{\^o}}}}1 {{õ}}{{{{\~o}}}}1 {{ú}}{{{{\'u}}}}1 {{ü}}{{{{\"u}}}}1 {{ç}}{{{{\c c}}}}1 {{Á}}{{{{\'A}}}}1 {{Ã}}{{{{\~A}}}}1 {{É}}{{{{\'E}}}}1 {{Ç}}{{{{\c C}}}}1}}
\begin{{document}}
"#
    );
    if doc.title.is_some() || doc.author.is_some() || doc.date.is_some() {
        out.push_str(&format!(
            "\\title{{{}}}\n\\author{{{}}}\n\\date{{{}}}\n\\maketitle\n",
            inline_tex(doc.title.as_deref().unwrap_or(""))?,
            inline_tex(doc.author.as_deref().unwrap_or(""))?,
            inline_tex(doc.date.as_deref().unwrap_or(""))?
        ));
    }
    for (index, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Figure(fig) | Block::Mermaid { figure: fig, .. } => {
                let asset = fig.asset_name(index)?;
                let page = if fig
                    .path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
                {
                    format!(",page={}", fig.page)
                } else {
                    String::new()
                };
                out.push_str(&format!("\\begin{{figure}}[{}]\n\\centering\n\\includegraphics[width={:.6}\\linewidth,height=0.65\\textheight,keepaspectratio{page}]{{{asset}}}\n\\caption{{{}}}\n", fig.placement, fig.width, inline_tex(&fig.caption)?));
                if let Some(id) = &fig.id {
                    out.push_str(&format!("\\label{{carbon:{id}}}\n"));
                }
                out.push_str("\\end{figure}\n");
            }
            Block::Heading(level, text) => {
                let command = match level {
                    1 => "section",
                    2 => "subsection",
                    _ => "subsubsection",
                };
                out.push_str(&format!("\\{command}{{{}}}\n", inline_tex(text)?));
            }
            Block::Paragraph(text) => {
                out.push_str(&inline_tex(text)?);
                out.push_str("\n\n");
            }
            Block::Equation(text) => {
                out.push_str("\\[\n");
                out.push_str(&math(text)?);
                out.push_str("\n\\]\n");
            }
            Block::Code { text, .. } => {
                // Prevent a code sample from terminating its generated environment.
                ensure!(
                    !text.contains("\\end{lstlisting}"),
                    "code contains reserved listing terminator"
                );
                out.push_str("\\begin{lstlisting}\n");
                out.push_str(text);
                out.push_str("\n\\end{lstlisting}\n");
            }
            Block::Table {
                caption,
                layout,
                header,
                rows,
            } => {
                if let Some(layout) = layout {
                    out.push_str("\\begin{table}[htbp]\n\\centering\n");
                    if let Some(caption) = caption {
                        out.push_str(&format!("\\caption{{{}}}\n", inline_tex(caption)?));
                    }
                    let spec = layout
                        .columns
                        .iter()
                        .zip(layout.align.chars())
                        .map(|(width, align)| {
                            let alignment = match align {
                                'l' => "raggedright",
                                'r' => "raggedleft",
                                _ => "centering",
                            };
                            let column = width
                                .map(|v| format!("p{{{v:.6}\\textwidth}}"))
                                .unwrap_or_else(|| "X".into());
                            format!(">{{\\{alignment}\\arraybackslash}}{column}")
                        })
                        .collect::<String>();
                    out.push_str(&format!(
                        "\\begin{{tabularx}}{{{:.6}\\textwidth}}{{{spec}}}\n\\toprule\n",
                        layout.width
                    ));
                    out.push_str(
                        &header
                            .iter()
                            .map(|s| Ok(format!("\\textbf{{{}}}", inline_tex(s)?)))
                            .collect::<Result<Vec<_>>>()?
                            .join(" & "),
                    );
                    out.push_str(" \\\\\n\\midrule\n");
                    for row in rows {
                        out.push_str(
                            &row.iter()
                                .map(|s| inline_tex(s))
                                .collect::<Result<Vec<_>>>()?
                                .join(" & "),
                        );
                        out.push_str(" \\\\\n");
                    }
                    out.push_str("\\bottomrule\n\\end{tabularx}\n\\end{table}\n");
                    continue;
                }
                // Fit columns using paragraph cells; longtable handles pagination and
                // repeats booktabs rules and the heading without splitting rows.
                let count = header.len();
                let cell = format!(
                    "p{{\\dimexpr(\\linewidth-{}\\tabcolsep)/{}\\relax}}",
                    2 * (count - 1),
                    count
                );
                let spec = format!(
                    "@{{}}{}{}@{{}}",
                    cell,
                    format!(">{{\\centering\\arraybackslash}}{cell}").repeat(count - 1)
                );
                out.push_str(&format!("\\begin{{longtable}}{{{spec}}}\n"));
                if let Some(caption) = caption {
                    out.push_str(&format!("\\caption{{{}}} \\\\\n", inline_tex(caption)?));
                }
                out.push_str("\\toprule\n");
                let head = header
                    .iter()
                    .map(|s| Ok(format!("\\textbf{{{}}}", inline_tex(s)?)))
                    .collect::<Result<Vec<_>>>()?
                    .join(" & ");
                out.push_str(&format!("{head} \\\\\n\\midrule\n\\endfirsthead\n\\toprule\n{head} \\\\\n\\midrule\n\\endhead\n\\bottomrule\n\\endfoot\n\\bottomrule\n\\endlastfoot\n"));
                for row in rows {
                    out.push_str(
                        &row.iter()
                            .map(|s| inline_tex(s))
                            .collect::<Result<Vec<_>>>()?
                            .join(" & "),
                    );
                    out.push_str(" \\\\\n");
                }
                out.push_str("\\end{longtable}\n");
            }
        }
    }
    out.push_str("\\end{document}\n");
    Ok(out)
}
fn executable(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_owned();
    }
    if let Some(p) = std::env::var_os("CARBON_TECTONIC") {
        return p.into();
    }
    let name = if cfg!(windows) {
        "tectonic.exe"
    } else {
        "tectonic"
    };
    let local = Path::new("tools").join("tectonic").join(name);
    if local.is_file() {
        return fs::canonicalize(&local).unwrap_or(local);
    }
    if let Ok(exe) = std::env::current_exe() {
        for parent in exe.ancestors().skip(1) {
            let local = parent.join("tools").join("tectonic").join(name);
            if local.is_file() {
                return local;
            }
        }
    }
    PathBuf::from(name)
}
#[cfg(test)]
pub(super) fn render(
    doc: &Document,
    engine: Option<&Path>,
    paper: Paper,
) -> Result<(Vec<u8>, usize)> {
    render_at(doc, engine, paper, &std::env::current_dir()?)
}
#[cfg(test)]
pub(super) fn render_at(
    doc: &Document,
    engine: Option<&Path>,
    paper: Paper,
    base: &Path,
) -> Result<(Vec<u8>, usize)> {
    render_with_mermaid(doc, engine, paper, base, None)
}
pub(super) fn render_with_mermaid(
    doc: &Document,
    engine: Option<&Path>,
    paper: Paper,
    base: &Path,
    mermaid_cli: Option<&Path>,
) -> Result<(Vec<u8>, usize)> {
    let tex = source(doc, paper)?;
    let dir = tempfile::tempdir().context("cannot create TeX working directory")?;
    crate::figures::stage(doc, base, dir.path())?;
    crate::mermaid::stage(doc, dir.path(), mermaid_cli)?;
    fs::write(dir.path().join("document.tex"), tex)?;
    let engine = executable(engine);
    let engine = if engine.components().count() > 1 {
        fs::canonicalize(&engine)
            .with_context(|| format!("cannot locate Tectonic {}", engine.display()))?
    } else {
        engine
    };
    let status = Command::new(&engine).current_dir(dir.path())
        .env("TECTONIC_UNTRUSTED_MODE", "1")
        .args(["--untrusted", "--keep-logs", "--chatter", "minimal", "document.tex"])
        .output().with_context(|| format!("cannot start Tectonic {}; install Tectonic, specify --tectonic PATH, or use --backend native", engine.display()))?;
    if !status.status.success() {
        let log = fs::read_to_string(dir.path().join("document.log")).unwrap_or_default();
        let log_tail = log
            .lines()
            .rev()
            .take(35)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "LaTeX compilation failed:\n{}\n{}\n{log_tail}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        );
    }
    let log = fs::read_to_string(dir.path().join("document.log")).unwrap_or_default();
    ensure!(!log.contains("Missing character:"), "LaTeX font lacks a requested character; use a supported mathematical command or a Latin text character");
    ensure!(
        !log.contains("undefined references") && !log.contains("multiply defined"),
        "LaTeX could not resolve figure references"
    );
    ensure!(
        !log.contains("Float too large for page"),
        "figure and caption exceed page height; shorten the caption or reduce the image width"
    );
    let bytes =
        fs::read(dir.path().join("document.pdf")).context("Tectonic did not produce a PDF")?;
    let mut pdf = printpdf::lopdf::Document::load_mem(&bytes)?;
    let pages = pdf.get_pages().len();
    // Preserve searchable, correctly encoded document metadata as in the native backend.
    let mut info = printpdf::lopdf::Dictionary::new();
    let string = |text: &str| {
        let mut bytes = vec![0xfe, 0xff];
        for c in text.encode_utf16() {
            bytes.extend_from_slice(&c.to_be_bytes());
        }
        printpdf::lopdf::Object::String(bytes, printpdf::lopdf::StringFormat::Hexadecimal)
    };
    info.set(
        "Title",
        string(doc.title.as_deref().unwrap_or("Carbon document")),
    );
    info.set("Author", string(doc.author.as_deref().unwrap_or("")));
    info.set("Creator", string("Carbon / Tectonic"));
    let id = pdf.add_object(info);
    pdf.trailer.set("Info", id);
    let mut result = Vec::new();
    pdf.save_to(&mut result)?;
    Ok((result, pages))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn greek_commands_and_unicode() {
        for (name, glyph) in GREEK {
            assert_eq!(math(&glyph.to_string()).unwrap(), format!("\\{name} "));
            assert!(math(&format!("\\{name}")).is_ok());
        }
        assert!(math(r"\frac{\alpha+\beta}{\Gamma} + x^2 + \sqrt{y}")
            .unwrap()
            .contains(r"\frac"));
        assert!(math(r"\begin{aligned} a &= b \\ c &= \gamma \end{aligned}").is_ok());
    }
    #[test]
    fn rejects_tex_injection() {
        for s in [
            r"\input{secret}",
            r"\write18{cmd}",
            r"\def\x{y}",
            r"\end{document}",
            r"\begin{document}",
            "x}",
            "{x",
            "x%",
            "x$",
            r"\csname input\endcsname",
        ] {
            assert!(math(s).is_err(), "{s}");
        }
        assert_eq!(
            inline_tex(r"plain \input{secret} $\alpha$").unwrap(),
            r"plain \textbackslash{}input\{secret\} $\alpha $"
        );
    }
    #[test]
    fn generates_reference_style() {
        let doc = super::super::parse(
            "@title Test\n\n# Intro\nA **bold** and $α + β$.\n\n$$\\frac{1}{2}$$",
        )
        .unwrap();
        let tex = source(&doc, Paper::Letter).unwrap();
        for expected in [
            "10pt,letterpaper",
            "margin=2cm",
            "10pt,letterpaper",
            r"\section{Intro}",
            r"\textbf{bold}",
            r"$\alpha  + \beta $",
        ] {
            assert!(tex.contains(expected), "{expected}\n{tex}");
        }
    }
    #[test]
    fn nested_math_and_table_captions() {
        assert_eq!(
            inline_tex(r"**bold $\alpha$** and \$literal").unwrap(),
            r"\textbf{bold $\alpha $} and \$literal"
        );
        let doc =
            super::super::parse("@caption Values\n| A | B |\n| --- | --- |\n| $\\beta$ | 2 |")
                .unwrap();
        let tex = source(&doc, Paper::Letter).unwrap();
        assert!(tex.contains(r"\caption{Values}"));
        assert!(tex.contains(r"\endfirsthead"));
        for bad in ["@caption Orphan", "@caption Orphan\n# Heading"] {
            assert!(super::super::parse(bad).is_err());
        }
    }
    #[test]
    fn configured_table_layout_options() {
        // Keep this unit-test input self-contained: private reference documents
        // and examples may be removed without breaking `cargo test` or Clippy.
        let doc = super::super::parse(
            "@caption Full width\n\
             @table width=1 columns=0.48,* align=lc\n\
             | Parameter | Value |\n\
             | --- | --- |\n\
             | Sample | 1 |\n\n\
             @caption Reduced width\n\
             @table width=0.75 columns=0.30,* align=lc\n\
             | Parameter | Value |\n\
             | --- | --- |\n\
             | Sample | 2 |\n\n\
             @caption Multiple columns\n\
             @table width=1 columns=0.15,0.24,0.12,0.12,0.15,* align=cccccc\n\
             | A | B | C | D | E | F |\n\
             | --- | --- | --- | --- | --- | --- |\n\
             | 1 | 2 | 3 | 4 | 5 | 6 |",
        )
        .unwrap();
        let tex = source(&doc, Paper::Letter).unwrap();
        assert_eq!(tex.matches("\\begin{tabularx}").count(), 3);
        assert!(tex.contains(r"\begin{tabularx}{0.750000\textwidth}"));
        for bad in [
            "width=NaN columns=0.4,* align=lc",
            "width=1 columns=0.99,* align=lc",
            "width=1 columns=0.4,* align=zz",
            "width=1 columns=0.4,0.4 align=lc",
            "width=1 columns=0.4,* align=lc input=x",
        ] {
            assert!(super::super::table_layout(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn figure_labels_and_forward_references() {
        let doc = super::super::parse("See **Figure @ref(fig:x)** on page @pageref(fig:x).\n![A $\\alpha$ curve](figures/curve.png){#fig:x width=50%}").unwrap();
        let tex = source(&doc, Paper::Letter).unwrap();
        assert!(tex.contains(r"\textbf{Figure \ref{carbon:fig:x}}"));
        assert!(tex.contains(r"\pageref{carbon:fig:x}"));
        assert!(tex.contains(r"\label{carbon:fig:x}"));
        assert!(tex.contains(r"{assets/figure-1.png}"));
        assert!(!tex.contains("figures/curve.png"));
        for bad in [
            "See @ref(fig:missing)",
            "![A](a.png){#fig:x}\n![B](b.png){#fig:x}",
            "See @ref(fig:x",
            "See @ref(bad id)",
        ] {
            let result = super::super::parse(bad).and_then(|d| source(&d, Paper::Letter));
            assert!(result.is_err(), "{bad}");
        }
        let literal = super::super::parse(r"Escaped \@ref(fig:unknown)").unwrap();
        assert!(source(&literal, Paper::Letter).is_ok());
        let doc = super::super::parse(
            "@caption A detailed **caption**\n![Short description](a.png){#fig:x}",
        )
        .unwrap();
        let super::super::Block::Figure(fig) = &doc.blocks[0] else {
            panic!()
        };
        assert_eq!(fig.description, "Short description");
        assert_eq!(fig.caption, "A detailed **caption**");
    }

    #[test]
    #[ignore = "requires Tectonic and its cached TeX bundle"]
    fn figures_end_to_end() {
        let doc = super::super::parse(include_str!("../examples/imagens/figuras.cb")).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/imagens");
        let (bytes, pages) = render_at(&doc, None, Paper::Letter, &base).unwrap();
        assert!(pages >= 1);
        let pdf = printpdf::lopdf::Document::load_mem(&bytes).unwrap();
        assert!(pdf
            .objects
            .values()
            .filter_map(|v| v.as_dict().ok())
            .any(|d| d.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Link")));
    }

    #[test]
    #[ignore = "requires Tectonic and its cached TeX bundle"]
    fn tectonic_end_to_end() {
        let doc =
            super::super::parse(include_str!("../examples/matematica/matematica.cb")).unwrap();
        let (bytes, pages) = render(&doc, None, Paper::Letter).unwrap();
        assert!(pages >= 1);
        let pdf = printpdf::lopdf::Document::load_mem(&bytes).unwrap();
        let first = *pdf.get_pages().values().next().unwrap();
        let mut page = pdf.get_dictionary(first).unwrap();
        // MediaBox is inheritable through the PDF page tree.
        while page.get(b"MediaBox").is_err() {
            let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
            page = pdf.get_dictionary(parent).unwrap();
        }
        let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
        assert_eq!(media[2].as_float().unwrap(), 612.);
        assert_eq!(media[3].as_float().unwrap(), 792.);
    }
}
