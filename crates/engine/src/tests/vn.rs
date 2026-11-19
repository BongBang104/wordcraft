//! Personal fork: spreadsheet cells pasted as plain text become a table, and the Vietnamese
//! administrative document format (Nghị định 30/2020/NĐ-CP).

use serde_json::json;
use wordcraft_doc::{Block, Document};

use crate::Session;
use crate::cmd::paste::parse_tsv;

fn run(s: &mut Session, id: &str, v: serde_json::Value) -> serde_json::Value {
    s.run(id, &v).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn new() -> Session {
    Session::new(Document::new())
}

/// The first table in the body: its cells' text, row by row.
fn first_table(s: &Session) -> Option<Vec<Vec<String>>> {
    s.doc.body.iter().find_map(|b| match &**b {
        Block::Table(t) => Some(
            t.rows
                .iter()
                .map(|r| {
                    r.cells
                        .iter()
                        .map(|c| {
                            c.blocks
                                .iter()
                                .filter_map(|b| match &**b {
                                    Block::Para(p) => Some(p.text.clone()),
                                    Block::Table(_) => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .collect()
                })
                .collect(),
        ),
        Block::Para(_) => None,
    })
}

#[test]
fn tsv_parses_excel_cells() {
    let rows = parse_tsv("Họ tên\tĐiểm\r\nAn\t9\r\nBình\t8,5\r\n").unwrap();
    assert_eq!(rows, vec![vec!["Họ tên", "Điểm"], vec!["An", "9"], vec!["Bình", "8,5"]]);
    // Quoted fields: a tab, a line break and a quote inside a cell.
    let rows = parse_tsv("\"a\tb\"\t\"line1\nline2\"\n\"say \"\"hi\"\"\"\t\n").unwrap();
    assert_eq!(rows, vec![vec!["a\tb", "line1\nline2"], vec!["say \"hi\"", ""]]);
    // Not cells: no tab, ragged rows, an unterminated quote.
    assert_eq!(parse_tsv("just text"), None);
    assert_eq!(parse_tsv("a\tb\nc"), None);
    assert_eq!(parse_tsv("\"a\tb"), None);
    assert_eq!(parse_tsv(""), None);
    // Too many columns.
    assert_eq!(parse_tsv(&"x\t".repeat(70)), None);
}

#[test]
fn pasting_tab_separated_text_makes_a_table() {
    let mut s = new();
    run(&mut s, "edit.paste", json!({"text": "Họ tên\tĐơn vị\tĐiểm\r\nNguyễn Văn An\tKíp 1\t9\r\nTrần Bình\tKíp 2\t8,5\r\n"}));
    let t = first_table(&s).expect("a table");
    assert_eq!(t.len(), 3);
    assert!(t.iter().all(|r| r.len() == 3), "{t:?}");
    assert_eq!(t[0], vec!["Họ tên", "Đơn vị", "Điểm"]);
    assert_eq!(t[2], vec!["Trần Bình", "Kíp 2", "8,5"]);
    // A table style with a grid (Table Grid), like a table inserted here.
    let has_borders = s.doc.body.iter().any(|b| match &**b {
        Block::Table(t) => t.props.style.is_some(),
        Block::Para(_) => false,
    });
    assert!(has_borders);
    // Undo takes the whole paste back.
    run(&mut s, "edit.undo", json!({}));
    assert!(first_table(&s).is_none());
}

#[test]
fn text_that_is_not_cells_stays_text() {
    let mut s = new();
    run(&mut s, "edit.paste", json!({"text": "one line\nanother line"}));
    assert!(first_table(&s).is_none());
    run(&mut s, "edit.paste", json!({"text": "a\tb\nc"}));
    assert!(first_table(&s).is_none());
}

#[test]
fn keep_text_only_keeps_tab_separated_text_as_text() {
    let mut s = new();
    run(&mut s, "edit.pasteText", json!({"text": "a\tb\nc\td"}));
    assert!(first_table(&s).is_none());
    let mut s = new();
    run(&mut s, "edit.pasteSpecial", json!({"as": "text", "text": "a\tb\nc\td"}));
    assert!(first_table(&s).is_none());
}

#[test]
fn html_still_wins_over_tab_separated_text() {
    let mut s = new();
    run(&mut s, "edit.paste", json!({"text": "a\tb", "html": "<table><tr><td><b>x</b></td><td>y</td></tr></table>"}));
    let t = first_table(&s).expect("a table");
    assert_eq!(t, vec![vec!["x", "y"]]);
}

#[test]
fn hostile_tab_separated_text_never_panics() {
    for text in ["\t", "\t\t\n\t\t", "\"", "\"\"\t\"", "\"a\"b\tc", "\r\r\r\t", "<script>\t&amp;\n</td>\t\"<tr>\"", "\u{0}\t\u{feff}"] {
        let mut s = new();
        let _ = s.run("edit.paste", &json!({"text": text}));
        let _ = s.run("edit.pasteMerge", &json!({"text": text}));
    }
    let big = "x\ty\n".repeat(2000);
    let mut s = new();
    run(&mut s, "edit.paste", json!({"text": big}));
    assert!(first_table(&s).is_none(), "more rows than the cap stay text");
}

fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn nd30_template_has_the_decrees_page_and_font() {
    let mut s = new();
    run(&mut s, "file.new", json!({"template": "nd30"}));
    let x = &s.doc.last_section;
    assert!(close(x.page_w, 595.28) && close(x.page_h, 841.89), "A4");
    assert!(close(x.margin_top, mm(20.0)) && close(x.margin_bottom, mm(20.0)));
    assert!(close(x.margin_left, mm(30.0)) && close(x.margin_right, mm(15.0)));
    assert!(x.title_page, "no page number on the first page");
    let normal = s.doc.styles.get("Normal").expect("Normal");
    assert_eq!(normal.chr.font.as_deref(), Some("Times New Roman"));
    assert_eq!(normal.chr.size, Some(14.0));
    let body = s.doc.plain_text(wordcraft_doc::StoryRef::Body);
    for want in ["CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM", "Độc lập - Tự do - Hạnh phúc", "TÊN LOẠI VĂN BẢN", "Nơi nhận:", "Họ và tên"]
    {
        assert!(body.contains(want), "{want} in {body}");
    }
    // It lays out, and saves and reopens as DOCX with the font and margins.
    let l = wordcraft_layout::layout(&s.doc, &mut wordcraft_layout::LayoutCache::new(), &Default::default());
    assert!(!l.pages.is_empty());
    let bytes = crate::io::save_bytes("x.docx", &s.doc).expect("save");
    let back = crate::io::open_bytes("x.docx", &bytes).expect("reopen");
    assert!(close(back.last_section.margin_left, mm(30.0)));
    assert_eq!(back.styles.get("Normal").and_then(|n| n.chr.font.clone()).as_deref(), Some("Times New Roman"));
}

#[test]
fn apply_nd30_sets_page_and_normal_style_and_undoes() {
    let mut s = new();
    run(&mut s, "document.setText", json!({"text": "Kính gửi: Ban Giám đốc"}));
    let before = s.doc.last_section.clone();
    let r = run(&mut s, "layout.applyNd30", json!({}));
    assert_eq!(r["size"], json!(14.0));
    let x = &s.doc.last_section;
    assert!(close(x.page_w, 595.28) && close(x.margin_left, mm(30.0)) && close(x.margin_right, mm(15.0)));
    let normal = s.doc.styles.get("Normal").expect("Normal");
    assert_eq!(normal.para.align, Some(wordcraft_doc::props::Align::Justify));
    assert_eq!(normal.para.indent_first, Some(36.0));
    run(&mut s, "edit.undo", json!({}));
    assert!(close(s.doc.last_section.margin_left, before.margin_left));
    assert!(close(s.doc.last_section.page_w, before.page_w));
    // Size 13, and sizes outside the decree's range refused.
    run(&mut s, "layout.applyNd30", json!({"size": 13}));
    assert_eq!(s.doc.styles.get("Normal").and_then(|n| n.chr.size), Some(13.0));
    for bad in [json!({"size": 40}), json!({"size": -1}), json!({"size": "x"}), json!({"size": 1e9})] {
        assert!(s.run("layout.applyNd30", &bad).is_err() || bad["size"].is_string(), "{bad}");
    }
}

#[test]
fn apply_nd30_reaches_every_section() {
    let mut s = new();
    run(&mut s, "document.setText", json!({"text": "Một\nHai"}));
    run(&mut s, "caret.docEnd", json!({}));
    run(&mut s, "layout.break", json!({"kind": "nextPage"}));
    assert!(s.doc.sections().len() >= 2);
    run(&mut s, "layout.applyNd30", json!({}));
    for (_, x) in s.doc.sections() {
        assert!(close(x.margin_left, mm(30.0)) && close(x.margin_top, mm(20.0)), "{x:?}");
    }
}
