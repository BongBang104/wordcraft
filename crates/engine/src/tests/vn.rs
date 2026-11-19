//! Personal fork: spreadsheet cells pasted as plain text become a table.

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
