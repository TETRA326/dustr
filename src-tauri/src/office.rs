// Built-in previews for .docx / .xlsx / .pptx that need no Office install.
// These files are zip archives of XML; we pull out the text (and for
// spreadsheets, a grid of the first sheet). Used on Windows when no preview
// handler is available, and on every other platform.
use base64::Engine;
use serde::Serialize;
use std::fs::File;
use std::io::Read;

const MAX_TEXT: usize = 64 * 1024;
const MAX_ROWS: usize = 40;
const MAX_COLS: usize = 12;

#[derive(Serialize, Default)]
pub struct OfficePreview {
    pub text: Option<String>,
    pub rows: Option<Vec<Vec<String>>>,
    pub image: Option<String>,
}

// ---------- minimal XML scanning ----------
enum Tok<'a> {
    Open { name: &'a str, attrs: &'a str, empty: bool },
    Close(&'a str),
    Text(&'a str),
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn tokens(xml: &str) -> Vec<Tok<'_>> {
    let mut out = Vec::new();
    let mut rest = xml;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push(Tok::Text(rest));
            break;
        };
        if lt > 0 {
            out.push(Tok::Text(&rest[..lt]));
        }
        rest = &rest[lt..];
        if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r.find("]]>").unwrap_or(r.len());
            out.push(Tok::Text(&r[..end]));
            rest = r.get(end + 3..).unwrap_or("");
            continue;
        }
        if rest.starts_with("<!--") {
            rest = rest.find("-->").and_then(|e| rest.get(e + 3..)).unwrap_or("");
            continue;
        }
        let Some(gt) = rest.find('>') else { break };
        let inner = &rest[1..gt];
        rest = &rest[gt + 1..];
        if inner.starts_with('?') || inner.starts_with('!') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            out.push(Tok::Close(local(name.trim())));
            continue;
        }
        let empty = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let (name, attrs) = match inner.find(char::is_whitespace) {
            Some(i) => (&inner[..i], &inner[i..]),
            None => (inner, ""),
        };
        out.push(Tok::Open { name: local(name), attrs, empty });
    }
    out
}

fn attr<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let mut search = attrs;
    loop {
        let i = search.find(key)?;
        let before_ok = i == 0 || search.as_bytes()[i - 1].is_ascii_whitespace();
        let after = &search[i + key.len()..];
        if before_ok {
            if let Some(v) = after.trim_start().strip_prefix('=') {
                let v = v.trim_start();
                let q = v.chars().next()?;
                if q == '"' || q == '\'' {
                    let end = v[1..].find(q)?;
                    return Some(&v[1..1 + end]);
                }
            }
        }
        search = after;
    }
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(semi) = rest.find(';') else { break };
        let ent = &rest[1..semi];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------- zip helpers ----------
type Archive = zip::ZipArchive<File>;

fn read_entry(zip: &mut Archive, name: &str) -> Option<String> {
    let mut f = zip.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn read_bytes(zip: &mut Archive, name: &str) -> Option<Vec<u8>> {
    let mut f = zip.by_name(name).ok()?;
    let mut b = Vec::new();
    f.read_to_end(&mut b).ok()?;
    Some(b)
}

fn names(zip: &Archive) -> Vec<String> {
    zip.file_names().filter_map(|n| n.ok().map(|n| n.to_string())).collect()
}

fn trailing_number(s: &str) -> u32 {
    let stem = s.trim_end_matches(".xml");
    let digits: String = stem.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    digits.chars().rev().collect::<String>().parse().unwrap_or(0)
}

/// Text of runs (`<x:t>`), with paragraph breaks.
fn run_text(xml: &str, out: &mut String) {
    let mut in_t = false;
    for tok in tokens(xml) {
        if out.len() > MAX_TEXT {
            return;
        }
        match tok {
            Tok::Open { name, empty, .. } => match name {
                "t" if !empty => in_t = true,
                "tab" => out.push('\t'),
                "br" | "cr" => out.push('\n'),
                _ => {}
            },
            Tok::Close(name) => match name {
                "t" => in_t = false,
                "p" => out.push('\n'),
                _ => {}
            },
            Tok::Text(t) if in_t => out.push_str(&unescape(t)),
            _ => {}
        }
    }
}

// ---------- formats ----------
fn docx(zip: &mut Archive) -> OfficePreview {
    let mut text = String::new();
    if let Some(xml) = read_entry(zip, "word/document.xml") {
        run_text(&xml, &mut text);
    }
    OfficePreview { text: Some(text.trim().to_string()), ..Default::default() }
}

fn pptx(zip: &mut Archive) -> OfficePreview {
    let mut slides: Vec<String> = names(zip)
        .into_iter()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slides.sort_by_key(|n| trailing_number(n));
    let mut text = String::new();
    for (i, s) in slides.iter().enumerate() {
        if text.len() > MAX_TEXT {
            break;
        }
        if let Some(xml) = read_entry(zip, s) {
            text.push_str(&format!("— Slide {} —\n", i + 1));
            run_text(&xml, &mut text);
            text.push('\n');
        }
    }
    let image = ["docProps/thumbnail.jpeg", "docProps/thumbnail.jpg", "docProps/thumbnail.png"]
        .iter()
        .find_map(|n| read_bytes(zip, n).map(|b| (n, b)))
        .map(|(n, b)| {
            let mime = if n.ends_with("png") { "image/png" } else { "image/jpeg" };
            format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(b))
        });
    OfficePreview { text: Some(text.trim().to_string()), image, ..Default::default() }
}

fn col_index(cell_ref: &str) -> Option<(usize, usize)> {
    let letters: String = cell_ref.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let row: usize = cell_ref[letters.len()..].parse().ok()?;
    let col = letters
        .chars()
        .fold(0usize, |acc, c| acc * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1));
    Some((row.checked_sub(1)?, col.checked_sub(1)?))
}

fn xlsx(zip: &mut Archive) -> OfficePreview {
    // Shared strings
    let mut shared = Vec::new();
    if let Some(xml) = read_entry(zip, "xl/sharedStrings.xml") {
        let mut cur = String::new();
        let mut in_t = false;
        for tok in tokens(&xml) {
            match tok {
                Tok::Open { name: "si", .. } => cur.clear(),
                Tok::Open { name: "t", empty: false, .. } => in_t = true,
                Tok::Close("t") => in_t = false,
                Tok::Close("si") => shared.push(std::mem::take(&mut cur)),
                Tok::Text(t) if in_t => cur.push_str(&unescape(t)),
                _ => {}
            }
        }
    }
    // First worksheet
    let mut sheets: Vec<String> = names(zip)
        .into_iter()
        .filter(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
        .collect();
    sheets.sort_by_key(|n| trailing_number(n));
    let Some(xml) = sheets.first().and_then(|s| read_entry(zip, s)) else {
        return OfficePreview::default();
    };

    let mut grid: Vec<Vec<String>> = Vec::new();
    let mut pos: Option<(usize, usize)> = None;
    let mut kind = String::new();
    let mut value = String::new();
    let mut in_value = false;
    for tok in tokens(&xml) {
        match tok {
            Tok::Open { name: "c", attrs, empty } => {
                pos = attr(attrs, "r").and_then(col_index);
                kind = attr(attrs, "t").unwrap_or("n").to_string();
                value.clear();
                if empty {
                    pos = None;
                }
            }
            Tok::Open { name: "v" | "t", empty: false, .. } if pos.is_some() => in_value = true,
            Tok::Close("v" | "t") => in_value = false,
            Tok::Text(t) if in_value => value.push_str(&unescape(t)),
            Tok::Close("c") => {
                if let Some((r, c)) = pos.take() {
                    if r < MAX_ROWS && c < MAX_COLS {
                        let shown = match kind.as_str() {
                            "s" => value.trim().parse::<usize>().ok().and_then(|i| shared.get(i).cloned()).unwrap_or_default(),
                            "b" => if value.trim() == "1" { "TRUE".into() } else { "FALSE".into() },
                            _ => value.clone(),
                        };
                        if grid.len() <= r {
                            grid.resize(r + 1, Vec::new());
                        }
                        if grid[r].len() <= c {
                            grid[r].resize(c + 1, String::new());
                        }
                        grid[r][c] = shown;
                    }
                }
            }
            Tok::Close("sheetData") => break,
            _ => {}
        }
    }
    let width = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    for r in grid.iter_mut() {
        r.resize(width, String::new());
    }
    OfficePreview { rows: Some(grid), ..Default::default() }
}

#[tauri::command]
pub async fn office_preview(path: String) -> Result<OfficePreview, String> {
    let ext = std::path::Path::new(&path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "Not a readable Office file".to_string())?;
    Ok(match ext.as_str() {
        "docx" | "docm" | "dotx" => docx(&mut zip),
        "pptx" | "pptm" | "ppsx" => pptx(&mut zip),
        "xlsx" | "xlsm" | "xltx" => xlsx(&mut zip),
        _ => return Err("Unsupported format".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_and_unescape() {
        let mut s = String::new();
        run_text(r#"<w:p><w:r><w:t xml:space="preserve">A &amp; B</w:t></w:r></w:p><w:p><w:r><w:t>C&#233;</w:t></w:r></w:p>"#, &mut s);
        assert_eq!(s, "A & B\nCé\n");
        assert_eq!(col_index("AB12"), Some((11, 27)));
        assert_eq!(attr(r#" r="B3" t="s""#, "t"), Some("s"));
        assert_eq!(attr(r#" r="B3" t="s""#, "r"), Some("B3"));
    }
}
