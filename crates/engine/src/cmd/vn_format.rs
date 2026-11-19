//! The Vietnamese administrative document format (thể thức văn bản hành chính), Nghị định
//! 30/2020/NĐ-CP, Phụ lục I: A4 paper, margins top 20 mm, bottom 20 mm, left 30 mm, right
//! 15 mm, Times New Roman 13–14 pt, body paragraphs justified with a first-line indent and at
//! least 6 pt between paragraphs.
//!
//! `layout.applyNd30` gives an open document that page setup and Normal style;
//! [`nd30_document`] is the File › New template with the header block (cơ quan ban hành, quốc
//! hiệu và tiêu ngữ), the document type, the subject line and the signature block. The template's
//! text is placeholders laid out as the decree prescribes, not anyone's document.

use std::sync::Arc;

use serde_json::{Value, json};
use wordcraft_doc::para::InlineObject;
use wordcraft_doc::props::{Align, CharProps, LineSpacing, Underline};
use wordcraft_doc::{Block, Document, Paragraph, PartKind, Table, para_block};

use crate::{CmdError, CmdResult, CommandSpec, Session, p};

/// Points per millimetre.
const PT_PER_MM: f32 = 72.0 / 25.4;
/// Margins, millimetres: top, bottom, left, right.
pub const MARGINS_MM: (f32, f32, f32, f32) = (20.0, 20.0, 30.0, 15.0);
/// A4, points.
pub const A4: (f32, f32) = (595.28, 841.89);
/// The body font.
pub const FONT: &str = "Times New Roman";
/// First-line indent of body paragraphs, points (1.27 cm).
pub const FIRST_LINE: f32 = 36.0;
/// Space after body paragraphs, points.
pub const SPACE_AFTER: f32 = 6.0;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("layout.applyNd30", "Thể thức NĐ 30/2020", "Layout › Page Setup", apply).params(
            r#"{"size"?: 13|14 (body font size, default 14)} — A4, margins 20/20/30/15 mm on every section, Normal style Times New Roman, justified, first line 1.27 cm, 6 pt after"#,
        ),
    ]
}

fn mm(v: f32) -> f32 {
    v * PT_PER_MM
}

/// The body font size from params: 13 or 14 (the decree's range), 14 when not given.
fn size_param(v: &Value) -> Result<f32, CmdError> {
    match p::f32(v, "size") {
        None => Ok(14.0),
        Some(x) if (13.0..=14.0).contains(&x) => Ok(x),
        Some(x) => Err(CmdError::Params(format!("`size` must be 13 or 14 (got {x})"))),
    }
}

fn apply(s: &mut Session, v: &Value) -> CmdResult {
    let size = size_param(v)?;
    apply_to(&mut s.doc, size);
    super::para::touch_all(s);
    let x = super::page::sect(s);
    Ok(json!({
        "pageWidth": x.page_w, "pageHeight": x.page_h,
        "margins": {"top": x.margin_top, "bottom": x.margin_bottom, "left": x.margin_left, "right": x.margin_right},
        "font": FONT, "size": size,
    }))
}

/// Page setup on every section and the Normal style, as `layout.applyNd30` does.
pub fn apply_to(doc: &mut Document, size: f32) {
    let (t, b, l, r) = MARGINS_MM;
    let ends: Vec<usize> = doc.sections().iter().map(|(e, _)| *e).collect();
    for end in ends {
        let x = doc.section_mut(end);
        (x.page_w, x.page_h) = if x.landscape { (A4.1, A4.0) } else { A4 };
        x.margin_top = mm(t);
        x.margin_bottom = mm(b);
        x.margin_left = mm(l);
        x.margin_right = mm(r);
    }
    let styles = Arc::make_mut(&mut doc.styles);
    styles.default_chr.font = Some(FONT.into());
    styles.default_chr.size = Some(size);
    if let Some(normal) = styles.get_mut("Normal") {
        normal.chr.font = Some(FONT.into());
        normal.chr.size = Some(size);
        normal.para.align = Some(Align::Justify);
        normal.para.indent_first = Some(FIRST_LINE);
        normal.para.space_before = Some(0.0);
        normal.para.space_after = Some(SPACE_AFTER);
        normal.para.line_spacing = Some(LineSpacing::Multiple(1.0));
    }
}

fn run(size: f32) -> CharProps {
    CharProps { size: Some(size), ..Default::default() }
}

fn bold(size: f32) -> CharProps {
    CharProps { bold: Some(true), ..run(size) }
}

/// A paragraph of one run, centred, with no first-line indent or space after (the header and
/// signature blocks sit in table cells and keep their lines together).
fn line(text: &str, props: CharProps, align: Align) -> Paragraph {
    let mut p = Paragraph::with_text(text, props);
    p.props.align = Some(align);
    p.props.indent_first = Some(0.0);
    p.props.space_after = Some(0.0);
    p
}

fn centred(text: &str, props: CharProps) -> Paragraph {
    line(text, props, Align::Center)
}

/// A borderless table of one row: each cell's paragraphs, with the grid widths given.
fn layout_table(widths: [f32; 2], cells: [Vec<Paragraph>; 2]) -> Table {
    let mut t = Table::new(1, 2, widths[0] + widths[1]);
    t.props.style = None;
    t.props.borders = None;
    t.grid = widths.to_vec();
    if let Some(row) = t.rows.first_mut() {
        for ((cell, paras), w) in row.cells.iter_mut().zip(cells).zip(widths) {
            cell.props.width = Some(w);
            cell.props.borders = None;
            cell.blocks = paras.into_iter().map(para_block).collect();
        }
    }
    t
}

/// File › New › "Văn bản hành chính (NĐ 30/2020)".
pub fn nd30_document() -> Document {
    let mut d = Document::new();
    apply_to(&mut d, 14.0);
    let text_w = A4.0 - mm(MARGINS_MM.2) - mm(MARGINS_MM.3);
    let left_w = (text_w * 0.4).round();
    let right_w = text_w - left_w;

    let mut b: Vec<Block> = Vec::new();
    // Header block: issuing body on the left, national title and motto on the right.
    let mut agency = centred("TÊN CƠ QUAN BAN HÀNH", bold(13.0));
    agency.props.space_after = Some(6.0);
    let mut motto = centred("Độc lập - Tự do - Hạnh phúc", CharProps { underline: Some(Underline::Single), ..bold(14.0) });
    motto.props.space_after = Some(6.0);
    b.push(Block::Table(layout_table(
        [left_w, right_w],
        [
            vec![centred("TÊN CƠ QUAN CHỦ QUẢN", run(13.0)), agency, centred("Số: …/…-…", run(13.0))],
            vec![
                centred("CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM", bold(13.0)),
                motto,
                centred("Đà Nẵng, ngày … tháng … năm 20…", CharProps { italic: Some(true), ..run(14.0) }),
            ],
        ],
    )));
    // Document type and subject.
    let mut kind = centred("TÊN LOẠI VĂN BẢN", bold(14.0));
    kind.props.space_before = Some(18.0);
    b.push(Block::Para(kind));
    let mut subject = centred("Về việc …", bold(14.0));
    subject.props.space_after = Some(12.0);
    b.push(Block::Para(subject));
    // Body: Normal paragraphs (justified, first-line indent, 6 pt after).
    for t in [
        "Nội dung văn bản: căn cứ ban hành, mục đích, yêu cầu và các nội dung cụ thể.",
        "Mỗi đoạn văn lùi đầu dòng 1,27 cm, căn đều hai lề, cách đoạn sau 6 pt.",
        "Đoạn kết thúc văn bản.",
    ] {
        b.push(Block::Para(Paragraph::with_text(t, CharProps::default())));
    }
    // Recipients on the left, signature on the right.
    let mut to = line("Nơi nhận:", CharProps { italic: Some(true), ..bold(12.0) }, Align::Left);
    to.props.space_before = Some(12.0);
    let mut sign = centred("QUYỀN HẠN, CHỨC VỤ CỦA NGƯỜI KÝ", bold(14.0));
    sign.props.space_before = Some(12.0);
    let blank = || centred("", run(14.0));
    b.push(Block::Table(layout_table(
        [left_w, right_w],
        [
            vec![to, line("- Như trên;", run(11.0), Align::Left), line("- Lưu: VT, ….", run(11.0), Align::Left)],
            vec![
                sign,
                centred("(Chữ ký, dấu)", CharProps { italic: Some(true), ..run(14.0) }),
                blank(),
                blank(),
                blank(),
                centred("Họ và tên", bold(14.0)),
            ],
        ],
    )));
    // A table can't end the body: an empty paragraph after it, as Word keeps.
    b.push(Block::Para(Paragraph::new()));
    d.body = b.into_iter().map(Arc::new).collect();

    // Page numbers: top centre, 13 pt, from page 2 (the first page's header is empty).
    let mut num = Paragraph::new().styled("Header");
    num.props.align = Some(Align::Center);
    num.props.indent_first = Some(0.0);
    let _ = num.insert_object(
        0,
        InlineObject::Field { instr: "PAGE".into(), result: "2".into(), locked: false, dirty: false, code: false, form: String::new() },
        &run(13.0),
    );
    let hid = d.add_part(PartKind::Header, vec![para_block(num)]);
    let first = d.add_part(PartKind::Header, vec![para_block(Paragraph::new().styled("Header"))]);
    d.last_section.headers.default = Some(hid);
    d.last_section.headers.first = Some(first);
    d.last_section.title_page = true;
    d.core.title = "Văn bản hành chính".into();
    d
}
