#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_TABLES: usize = 4;
const MAX_COLUMNS: usize = 8;
const MAX_ROWS: usize = 256;
const MAX_NAME_LEN: usize = 32;
const MAX_TEXT_LEN: usize = 64;

const PORT: u16 = 7881;

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

unsafe fn write_all(fd: i32, buf: &[u8]) {
    let mut written = 0;
    while written < buf.len() {
        let ret = libc::write(
            fd,
            buf.as_ptr().add(written) as *const libc::c_void,
            buf.len() - written,
        );
        if ret <= 0 {
            return;
        }
        written += ret as usize;
    }
}

fn format_u32(n: u32, buf: &mut [u8; 10]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut val = n;
    let mut pos = 10;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    let len = 10 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

fn format_i32(n: i32, buf: &mut [u8; 11]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let negative = n < 0;
    let mut val = if negative { (-(n as i64)) as u32 } else { n as u32 };
    let mut pos = 11;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    if negative {
        pos -= 1;
        buf[pos] = b'-';
    }
    let len = 11 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

fn copy_to(dest: &mut [u8], pos: &mut usize, src: &[u8]) {
    let mut i = 0;
    while i < src.len() && *pos < dest.len() {
        dest[*pos] = src[i];
        *pos += 1;
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// HTTP parsing helpers
// ---------------------------------------------------------------------------

fn find_header_end(buf: &[u8], len: usize) -> Option<usize> {
    if len < 4 {
        return None;
    }
    let mut i = 0;
    while i + 3 < len {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
            return Some(i + 4);
        }
        i += 1;
    }
    None
}

fn parse_content_length(buf: &[u8], header_end: usize) -> usize {
    let needle = b"Content-Length: ";
    let mut i = 0;
    while i + needle.len() < header_end {
        let mut matched = true;
        let mut j = 0;
        while j < needle.len() {
            let a = if buf[i + j] >= b'A' && buf[i + j] <= b'Z' {
                buf[i + j] + 32
            } else {
                buf[i + j]
            };
            let b = if needle[j] >= b'A' && needle[j] <= b'Z' {
                needle[j] + 32
            } else {
                needle[j]
            };
            if a != b {
                matched = false;
                break;
            }
            j += 1;
        }
        if matched {
            let start = i + needle.len();
            let mut val: usize = 0;
            let mut k = start;
            while k < header_end && buf[k] >= b'0' && buf[k] <= b'9' {
                val = val * 10 + (buf[k] - b'0') as usize;
                k += 1;
            }
            return val;
        }
        i += 1;
    }
    0
}

fn parse_request_line(buf: &[u8], len: usize) -> (usize, usize, usize) {
    let mut method_end = 0;
    while method_end < len && buf[method_end] != b' ' {
        method_end += 1;
    }
    let path_start = method_end + 1;
    let mut path_end = path_start;
    while path_end < len && buf[path_end] != b' ' {
        path_end += 1;
    }
    (method_end, path_start, path_end)
}

fn method_is(buf: &[u8], method_end: usize, expected: &[u8]) -> bool {
    if method_end != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < expected.len() {
        if buf[i] != expected[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn path_eq(buf: &[u8], start: usize, end: usize, expected: &[u8]) -> bool {
    let len = end - start;
    if len != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < len {
        if buf[start + i] != expected[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// HTTP response helper
// ---------------------------------------------------------------------------

unsafe fn send_response(fd: i32, status: &[u8], content_type: &[u8], body: &[u8]) {
    let mut num_buf = [0u8; 10];
    write_all(fd, b"HTTP/1.1 ");
    write_all(fd, status);
    write_all(fd, b"\r\nContent-Type: ");
    write_all(fd, content_type);
    write_all(fd, b"\r\nConnection: close\r\nContent-Length: ");
    let cl_len = format_u32(body.len() as u32, &mut num_buf);
    write_all(fd, &num_buf[..cl_len]);
    write_all(fd, b"\r\n\r\n");
    write_all(fd, body);
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum ValType {
    Null = 0,
    Int = 1,
    Text = 2,
}

#[derive(Clone, Copy)]
struct Value {
    vtype: ValType,
    int_val: i32,
    text_val: [u8; MAX_TEXT_LEN],
    text_len: usize,
}

struct Table {
    occupied: bool,
    name: [u8; MAX_NAME_LEN],
    name_len: usize,
    col_names: [[u8; MAX_NAME_LEN]; MAX_COLUMNS],
    col_name_lens: [usize; MAX_COLUMNS],
    col_types: [ValType; MAX_COLUMNS],
    col_count: usize,
    rows: [[Value; MAX_COLUMNS]; MAX_ROWS],
    row_occupied: [bool; MAX_ROWS],
    row_count: usize,
}

struct Database {
    tables: [Table; MAX_TABLES],
    table_count: usize,
}

// ---------------------------------------------------------------------------
// Table/column lookup helpers
// ---------------------------------------------------------------------------

fn name_eq_ci(a: &[u8], a_len: usize, b: &[u8], b_len: usize) -> bool {
    if a_len != b_len {
        return false;
    }
    let mut i = 0;
    while i < a_len {
        let ca = if a[i] >= b'A' && a[i] <= b'Z' { a[i] + 32 } else { a[i] };
        let cb = if b[i] >= b'A' && b[i] <= b'Z' { b[i] + 32 } else { b[i] };
        if ca != cb {
            return false;
        }
        i += 1;
    }
    true
}

fn find_table<'a>(db: &'a mut Database, name: &[u8], name_len: usize) -> Option<usize> {
    let mut i = 0;
    while i < MAX_TABLES {
        if db.tables[i].occupied
            && name_eq_ci(&db.tables[i].name, db.tables[i].name_len, name, name_len)
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_column(table: &Table, name: &[u8], name_len: usize) -> Option<usize> {
    let mut i = 0;
    while i < table.col_count {
        if name_eq_ci(&table.col_names[i], table.col_name_lens[i], name, name_len) {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// SQL tokenizer
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Keyword {
    Create,
    Table,
    Insert,
    Into,
    Values,
    Select,
    From,
    Where,
    Update,
    Set,
    Delete,
    Drop,
    Int,
    Text,
    And,
}

#[derive(Clone, Copy, PartialEq)]
enum Token {
    Kw(Keyword),
    Ident,
    IntLit,
    StrLit,
    LParen,
    RParen,
    Comma,
    Star,
    Eq,
    Eof,
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    len: usize,
    // Token data
    tok: Token,
    tok_start: usize,
    tok_len: usize,
    int_val: i32,
    str_buf: [u8; MAX_TEXT_LEN],
    str_len: usize,
}

fn keyword_match(s: &[u8], len: usize) -> Option<Keyword> {
    // Case-insensitive keyword matching
    let mut lower = [0u8; 16];
    if len > 16 {
        return None;
    }
    let mut i = 0;
    while i < len {
        lower[i] = if s[i] >= b'A' && s[i] <= b'Z' { s[i] + 32 } else { s[i] };
        i += 1;
    }
    let w = &lower[..len];
    if len == 3 && w[0] == b'i' && w[1] == b'n' && w[2] == b't' {
        return Some(Keyword::Int);
    }
    if len == 3 && w[0] == b's' && w[1] == b'e' && w[2] == b't' {
        return Some(Keyword::Set);
    }
    if len == 3 && w[0] == b'a' && w[1] == b'n' && w[2] == b'd' {
        return Some(Keyword::And);
    }
    if len == 4 {
        if w[0] == b't' && w[1] == b'e' && w[2] == b'x' && w[3] == b't' {
            return Some(Keyword::Text);
        }
        if w[0] == b'i' && w[1] == b'n' && w[2] == b't' && w[3] == b'o' {
            return Some(Keyword::Into);
        }
        if w[0] == b'f' && w[1] == b'r' && w[2] == b'o' && w[3] == b'm' {
            return Some(Keyword::From);
        }
        if w[0] == b'd' && w[1] == b'r' && w[2] == b'o' && w[3] == b'p' {
            return Some(Keyword::Drop);
        }
    }
    if len == 5 {
        if w[0] == b't' && w[1] == b'a' && w[2] == b'b' && w[3] == b'l' && w[4] == b'e' {
            return Some(Keyword::Table);
        }
        if w[0] == b'w' && w[1] == b'h' && w[2] == b'e' && w[3] == b'r' && w[4] == b'e' {
            return Some(Keyword::Where);
        }
    }
    if len == 6 {
        if w[0] == b'c' && w[1] == b'r' && w[2] == b'e' && w[3] == b'a' && w[4] == b't' && w[5] == b'e' {
            return Some(Keyword::Create);
        }
        if w[0] == b'i' && w[1] == b'n' && w[2] == b's' && w[3] == b'e' && w[4] == b'r' && w[5] == b't' {
            return Some(Keyword::Insert);
        }
        if w[0] == b's' && w[1] == b'e' && w[2] == b'l' && w[3] == b'e' && w[4] == b'c' && w[5] == b't' {
            return Some(Keyword::Select);
        }
        if w[0] == b'u' && w[1] == b'p' && w[2] == b'd' && w[3] == b'a' && w[4] == b't' && w[5] == b'e' {
            return Some(Keyword::Update);
        }
        if w[0] == b'd' && w[1] == b'e' && w[2] == b'l' && w[3] == b'e' && w[4] == b't' && w[5] == b'e' {
            return Some(Keyword::Delete);
        }
        if w[0] == b'v' && w[1] == b'a' && w[2] == b'l' && w[3] == b'u' && w[4] == b'e' && w[5] == b's' {
            return Some(Keyword::Values);
        }
    }
    None
}

fn next_token(p: &mut Parser) {
    // Skip whitespace
    while p.pos < p.len && (p.input[p.pos] == b' ' || p.input[p.pos] == b'\t' || p.input[p.pos] == b'\r' || p.input[p.pos] == b'\n') {
        p.pos += 1;
    }
    if p.pos >= p.len {
        p.tok = Token::Eof;
        return;
    }
    let ch = p.input[p.pos];
    match ch {
        b'(' => { p.tok = Token::LParen; p.pos += 1; }
        b')' => { p.tok = Token::RParen; p.pos += 1; }
        b',' => { p.tok = Token::Comma; p.pos += 1; }
        b'*' => { p.tok = Token::Star; p.pos += 1; }
        b'=' => { p.tok = Token::Eq; p.pos += 1; }
        b'\'' => {
            // String literal
            p.pos += 1;
            p.str_len = 0;
            while p.pos < p.len {
                if p.input[p.pos] == b'\'' {
                    if p.pos + 1 < p.len && p.input[p.pos + 1] == b'\'' {
                        // Escaped quote
                        if p.str_len < MAX_TEXT_LEN {
                            p.str_buf[p.str_len] = b'\'';
                            p.str_len += 1;
                        }
                        p.pos += 2;
                    } else {
                        p.pos += 1;
                        break;
                    }
                } else {
                    if p.str_len < MAX_TEXT_LEN {
                        p.str_buf[p.str_len] = p.input[p.pos];
                        p.str_len += 1;
                    }
                    p.pos += 1;
                }
            }
            p.tok = Token::StrLit;
        }
        b'-' | b'0'..=b'9' => {
            // Integer literal
            let start = p.pos;
            let negative = ch == b'-';
            if negative {
                p.pos += 1;
            }
            let mut val: i32 = 0;
            while p.pos < p.len && p.input[p.pos] >= b'0' && p.input[p.pos] <= b'9' {
                val = val * 10 + (p.input[p.pos] - b'0') as i32;
                p.pos += 1;
            }
            p.int_val = if negative { -val } else { val };
            p.tok_start = start;
            p.tok_len = p.pos - start;
            p.tok = Token::IntLit;
        }
        _ => {
            // Identifier or keyword
            if (ch >= b'A' && ch <= b'Z') || (ch >= b'a' && ch <= b'z') || ch == b'_' {
                let start = p.pos;
                while p.pos < p.len {
                    let c = p.input[p.pos];
                    if (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || (c >= b'0' && c <= b'9') || c == b'_' {
                        p.pos += 1;
                    } else {
                        break;
                    }
                }
                p.tok_start = start;
                p.tok_len = p.pos - start;
                if let Some(kw) = keyword_match(&p.input[start..], p.tok_len) {
                    p.tok = Token::Kw(kw);
                } else {
                    p.tok = Token::Ident;
                }
            } else {
                // Skip unknown character
                p.pos += 1;
                next_token(p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SQL executor functions
// ---------------------------------------------------------------------------

const RESP_SIZE: usize = 8192;

fn exec_create_table(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // CREATE TABLE name (col1 TYPE, col2 TYPE, ...)
    next_token(p); // skip TABLE
    if p.tok != Token::Kw(Keyword::Table) {
        copy_to(resp, rlen, b"ERROR: expected TABLE\n");
        return;
    }
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;
    if tname_len > MAX_NAME_LEN {
        copy_to(resp, rlen, b"ERROR: table name too long\n");
        return;
    }

    // Check if table already exists
    if find_table(db, &p.input[tname_start..], tname_len).is_some() {
        copy_to(resp, rlen, b"ERROR: table already exists\n");
        return;
    }

    // Find free slot
    let mut slot = MAX_TABLES;
    let mut i = 0;
    while i < MAX_TABLES {
        if !db.tables[i].occupied {
            slot = i;
            break;
        }
        i += 1;
    }
    if slot == MAX_TABLES {
        copy_to(resp, rlen, b"ERROR: max tables reached\n");
        return;
    }

    next_token(p); // (
    if p.tok != Token::LParen {
        copy_to(resp, rlen, b"ERROR: expected (\n");
        return;
    }

    let table = &mut db.tables[slot];
    let mut col_count = 0;

    loop {
        next_token(p); // column name
        if p.tok == Token::RParen {
            break;
        }
        if p.tok != Token::Ident {
            copy_to(resp, rlen, b"ERROR: expected column name\n");
            return;
        }
        if col_count >= MAX_COLUMNS {
            copy_to(resp, rlen, b"ERROR: too many columns\n");
            return;
        }
        let cname_start = p.tok_start;
        let cname_len = if p.tok_len > MAX_NAME_LEN { MAX_NAME_LEN } else { p.tok_len };
        let mut j = 0;
        while j < cname_len {
            table.col_names[col_count][j] = p.input[cname_start + j];
            j += 1;
        }
        table.col_name_lens[col_count] = cname_len;

        next_token(p); // type
        match p.tok {
            Token::Kw(Keyword::Int) => table.col_types[col_count] = ValType::Int,
            Token::Kw(Keyword::Text) => table.col_types[col_count] = ValType::Text,
            _ => {
                copy_to(resp, rlen, b"ERROR: expected INT or TEXT\n");
                return;
            }
        }
        col_count += 1;

        next_token(p); // , or )
        if p.tok == Token::RParen {
            break;
        }
        if p.tok != Token::Comma {
            copy_to(resp, rlen, b"ERROR: expected , or )\n");
            return;
        }
    }

    // Commit table creation
    table.occupied = true;
    let mut j = 0;
    while j < tname_len {
        table.name[j] = p.input[tname_start + j];
        j += 1;
    }
    table.name_len = tname_len;
    table.col_count = col_count;
    table.row_count = 0;
    db.table_count += 1;

    copy_to(resp, rlen, b"OK: table created\n");
}

fn exec_insert(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // INSERT INTO table VALUES (v1, v2, ...)
    next_token(p); // INTO
    if p.tok != Token::Kw(Keyword::Into) {
        copy_to(resp, rlen, b"ERROR: expected INTO\n");
        return;
    }
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;

    let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: table not found\n");
            return;
        }
    };

    next_token(p); // VALUES
    if p.tok != Token::Kw(Keyword::Values) {
        copy_to(resp, rlen, b"ERROR: expected VALUES\n");
        return;
    }
    next_token(p); // (
    if p.tok != Token::LParen {
        copy_to(resp, rlen, b"ERROR: expected (\n");
        return;
    }

    // Find free row
    let table = &mut db.tables[tidx];
    let mut row_idx = MAX_ROWS;
    let mut i = 0;
    while i < MAX_ROWS {
        if !table.row_occupied[i] {
            row_idx = i;
            break;
        }
        i += 1;
    }
    if row_idx == MAX_ROWS {
        copy_to(resp, rlen, b"ERROR: table full\n");
        return;
    }

    let col_count = table.col_count;
    let mut col = 0;
    loop {
        next_token(p);
        if p.tok == Token::RParen {
            break;
        }
        if col >= col_count {
            copy_to(resp, rlen, b"ERROR: too many values\n");
            return;
        }

        match p.tok {
            Token::IntLit => {
                table.rows[row_idx][col].vtype = ValType::Int;
                table.rows[row_idx][col].int_val = p.int_val;
            }
            Token::StrLit => {
                table.rows[row_idx][col].vtype = ValType::Text;
                let copy_len = if p.str_len > MAX_TEXT_LEN { MAX_TEXT_LEN } else { p.str_len };
                let mut j = 0;
                while j < copy_len {
                    table.rows[row_idx][col].text_val[j] = p.str_buf[j];
                    j += 1;
                }
                table.rows[row_idx][col].text_len = copy_len;
            }
            _ => {
                copy_to(resp, rlen, b"ERROR: expected value\n");
                return;
            }
        }
        col += 1;

        next_token(p);
        if p.tok == Token::RParen {
            break;
        }
        if p.tok != Token::Comma {
            copy_to(resp, rlen, b"ERROR: expected , or )\n");
            return;
        }
    }

    if col != col_count {
        copy_to(resp, rlen, b"ERROR: column count mismatch\n");
        return;
    }

    table.row_occupied[row_idx] = true;
    table.row_count += 1;

    copy_to(resp, rlen, b"OK: 1 row inserted\n");
}

fn parse_where(p: &mut Parser, table: &Table) -> Option<(usize, Value)> {
    // WHERE col = val
    next_token(p); // column name
    if p.tok != Token::Ident {
        return None;
    }
    let col_idx = match find_column(table, &p.input[p.tok_start..], p.tok_len) {
        Some(i) => i,
        None => return None,
    };
    next_token(p); // =
    if p.tok != Token::Eq {
        return None;
    }
    next_token(p); // value
    let mut val: Value = Value {
        vtype: ValType::Null,
        int_val: 0,
        text_val: [0; MAX_TEXT_LEN],
        text_len: 0,
    };
    match p.tok {
        Token::IntLit => {
            val.vtype = ValType::Int;
            val.int_val = p.int_val;
        }
        Token::StrLit => {
            val.vtype = ValType::Text;
            let copy_len = if p.str_len > MAX_TEXT_LEN { MAX_TEXT_LEN } else { p.str_len };
            let mut j = 0;
            while j < copy_len {
                val.text_val[j] = p.str_buf[j];
                j += 1;
            }
            val.text_len = copy_len;
        }
        _ => return None,
    }
    Some((col_idx, val))
}

fn row_matches(row: &[Value; MAX_COLUMNS], col_idx: usize, val: &Value) -> bool {
    let cell = &row[col_idx];
    if cell.vtype != val.vtype {
        return false;
    }
    match cell.vtype {
        ValType::Int => cell.int_val == val.int_val,
        ValType::Text => {
            if cell.text_len != val.text_len {
                return false;
            }
            let mut i = 0;
            while i < cell.text_len {
                if cell.text_val[i] != val.text_val[i] {
                    return false;
                }
                i += 1;
            }
            true
        }
        ValType::Null => true,
    }
}

fn write_value(val: &Value, resp: &mut [u8], rlen: &mut usize) {
    match val.vtype {
        ValType::Null => copy_to(resp, rlen, b"NULL"),
        ValType::Int => {
            let mut ibuf = [0u8; 11];
            let n = format_i32(val.int_val, &mut ibuf);
            copy_to(resp, rlen, &ibuf[..n]);
        }
        ValType::Text => {
            copy_to(resp, rlen, &val.text_val[..val.text_len]);
        }
    }
}

fn exec_select(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // SELECT */cols FROM table [WHERE col = val]
    next_token(p);

    // Parse column list
    let mut sel_cols: [usize; MAX_COLUMNS] = [0; MAX_COLUMNS];
    let mut sel_count: usize = 0;
    let select_all = p.tok == Token::Star;

    if select_all {
        next_token(p); // advance past *
    } else {
        // Column name list
        let mut col_names: [[u8; MAX_NAME_LEN]; MAX_COLUMNS] = [[0; MAX_NAME_LEN]; MAX_COLUMNS];
        let mut col_name_lens: [usize; MAX_COLUMNS] = [0; MAX_COLUMNS];
        loop {
            if p.tok != Token::Ident {
                copy_to(resp, rlen, b"ERROR: expected column name\n");
                return;
            }
            if sel_count >= MAX_COLUMNS {
                copy_to(resp, rlen, b"ERROR: too many columns\n");
                return;
            }
            let clen = if p.tok_len > MAX_NAME_LEN { MAX_NAME_LEN } else { p.tok_len };
            let mut j = 0;
            while j < clen {
                col_names[sel_count][j] = p.input[p.tok_start + j];
                j += 1;
            }
            col_name_lens[sel_count] = clen;
            sel_count += 1;
            next_token(p);
            if p.tok == Token::Comma {
                next_token(p);
            } else {
                break;
            }
        }

        // We need FROM next - but we need the table to resolve column names
        // So we store names and resolve after finding the table
        if p.tok != Token::Kw(Keyword::From) {
            copy_to(resp, rlen, b"ERROR: expected FROM\n");
            return;
        }
        next_token(p); // table name
        if p.tok != Token::Ident {
            copy_to(resp, rlen, b"ERROR: expected table name\n");
            return;
        }
        let tname_start = p.tok_start;
        let tname_len = p.tok_len;

        let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
            Some(i) => i,
            None => {
                copy_to(resp, rlen, b"ERROR: table not found\n");
                return;
            }
        };
        let table = &db.tables[tidx];

        // Resolve column names
        let mut i = 0;
        while i < sel_count {
            match find_column(table, &col_names[i], col_name_lens[i]) {
                Some(ci) => sel_cols[i] = ci,
                None => {
                    copy_to(resp, rlen, b"ERROR: column not found\n");
                    return;
                }
            }
            i += 1;
        }

        // Parse optional WHERE
        next_token(p);
        let where_clause = if p.tok == Token::Kw(Keyword::Where) {
            match parse_where(p, table) {
                Some(wc) => Some(wc),
                None => {
                    copy_to(resp, rlen, b"ERROR: invalid WHERE clause\n");
                    return;
                }
            }
        } else {
            None
        };

        // Output header
        i = 0;
        while i < sel_count {
            if i > 0 {
                copy_to(resp, rlen, b"\t");
            }
            copy_to(resp, rlen, &table.col_names[sel_cols[i]][..table.col_name_lens[sel_cols[i]]]);
            i += 1;
        }
        copy_to(resp, rlen, b"\n");

        // Output rows
        let mut row = 0;
        while row < MAX_ROWS {
            if table.row_occupied[row] {
                let matches = match &where_clause {
                    Some((col_idx, val)) => row_matches(&table.rows[row], *col_idx, val),
                    None => true,
                };
                if matches {
                    i = 0;
                    while i < sel_count {
                        if i > 0 {
                            copy_to(resp, rlen, b"\t");
                        }
                        write_value(&table.rows[row][sel_cols[i]], resp, rlen);
                        i += 1;
                    }
                    copy_to(resp, rlen, b"\n");
                }
            }
            row += 1;
        }
        return;
    }

    // SELECT * path
    if p.tok != Token::Kw(Keyword::From) {
        copy_to(resp, rlen, b"ERROR: expected FROM\n");
        return;
    }
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;

    let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: table not found\n");
            return;
        }
    };
    let table = &db.tables[tidx];

    next_token(p);
    let where_clause = if p.tok == Token::Kw(Keyword::Where) {
        match parse_where(p, table) {
            Some(wc) => Some(wc),
            None => {
                copy_to(resp, rlen, b"ERROR: invalid WHERE clause\n");
                return;
            }
        }
    } else {
        None
    };

    // Output header
    let mut i = 0;
    while i < table.col_count {
        if i > 0 {
            copy_to(resp, rlen, b"\t");
        }
        copy_to(resp, rlen, &table.col_names[i][..table.col_name_lens[i]]);
        i += 1;
    }
    copy_to(resp, rlen, b"\n");

    // Output rows
    let mut row = 0;
    while row < MAX_ROWS {
        if table.row_occupied[row] {
            let matches = match &where_clause {
                Some((col_idx, val)) => row_matches(&table.rows[row], *col_idx, val),
                None => true,
            };
            if matches {
                i = 0;
                while i < table.col_count {
                    if i > 0 {
                        copy_to(resp, rlen, b"\t");
                    }
                    write_value(&table.rows[row][i], resp, rlen);
                    i += 1;
                }
                copy_to(resp, rlen, b"\n");
            }
        }
        row += 1;
    }
}

fn exec_update(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // UPDATE table SET col = val [WHERE col = val]
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;

    let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: table not found\n");
            return;
        }
    };

    next_token(p); // SET
    if p.tok != Token::Kw(Keyword::Set) {
        copy_to(resp, rlen, b"ERROR: expected SET\n");
        return;
    }

    next_token(p); // column name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected column name\n");
        return;
    }
    let set_col = match find_column(&db.tables[tidx], &p.input[p.tok_start..], p.tok_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: column not found\n");
            return;
        }
    };

    next_token(p); // =
    if p.tok != Token::Eq {
        copy_to(resp, rlen, b"ERROR: expected =\n");
        return;
    }

    next_token(p); // value
    let mut set_val = Value {
        vtype: ValType::Null,
        int_val: 0,
        text_val: [0; MAX_TEXT_LEN],
        text_len: 0,
    };
    match p.tok {
        Token::IntLit => {
            set_val.vtype = ValType::Int;
            set_val.int_val = p.int_val;
        }
        Token::StrLit => {
            set_val.vtype = ValType::Text;
            let copy_len = if p.str_len > MAX_TEXT_LEN { MAX_TEXT_LEN } else { p.str_len };
            let mut j = 0;
            while j < copy_len {
                set_val.text_val[j] = p.str_buf[j];
                j += 1;
            }
            set_val.text_len = copy_len;
        }
        _ => {
            copy_to(resp, rlen, b"ERROR: expected value\n");
            return;
        }
    }

    next_token(p);
    let where_clause = if p.tok == Token::Kw(Keyword::Where) {
        match parse_where(p, &db.tables[tidx]) {
            Some(wc) => Some(wc),
            None => {
                copy_to(resp, rlen, b"ERROR: invalid WHERE clause\n");
                return;
            }
        }
    } else {
        None
    };

    let table = &mut db.tables[tidx];
    let mut updated = 0u32;
    let mut row = 0;
    while row < MAX_ROWS {
        if table.row_occupied[row] {
            let matches = match &where_clause {
                Some((col_idx, val)) => row_matches(&table.rows[row], *col_idx, val),
                None => true,
            };
            if matches {
                table.rows[row][set_col] = set_val;
                updated += 1;
            }
        }
        row += 1;
    }

    copy_to(resp, rlen, b"OK: ");
    let mut nbuf = [0u8; 10];
    let n = format_u32(updated, &mut nbuf);
    copy_to(resp, rlen, &nbuf[..n]);
    copy_to(resp, rlen, b" rows updated\n");
}

fn exec_delete(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // DELETE FROM table [WHERE col = val]
    next_token(p); // FROM
    if p.tok != Token::Kw(Keyword::From) {
        copy_to(resp, rlen, b"ERROR: expected FROM\n");
        return;
    }
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;

    let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: table not found\n");
            return;
        }
    };

    next_token(p);
    let where_clause = if p.tok == Token::Kw(Keyword::Where) {
        match parse_where(p, &db.tables[tidx]) {
            Some(wc) => Some(wc),
            None => {
                copy_to(resp, rlen, b"ERROR: invalid WHERE clause\n");
                return;
            }
        }
    } else {
        None
    };

    let table = &mut db.tables[tidx];
    let mut deleted = 0u32;
    let mut row = 0;
    while row < MAX_ROWS {
        if table.row_occupied[row] {
            let matches = match &where_clause {
                Some((col_idx, val)) => row_matches(&table.rows[row], *col_idx, val),
                None => true,
            };
            if matches {
                table.row_occupied[row] = false;
                table.row_count -= 1;
                deleted += 1;
            }
        }
        row += 1;
    }

    copy_to(resp, rlen, b"OK: ");
    let mut nbuf = [0u8; 10];
    let n = format_u32(deleted, &mut nbuf);
    copy_to(resp, rlen, &nbuf[..n]);
    copy_to(resp, rlen, b" rows deleted\n");
}

fn exec_drop_table(p: &mut Parser, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    // DROP TABLE name
    next_token(p); // TABLE
    if p.tok != Token::Kw(Keyword::Table) {
        copy_to(resp, rlen, b"ERROR: expected TABLE\n");
        return;
    }
    next_token(p); // table name
    if p.tok != Token::Ident {
        copy_to(resp, rlen, b"ERROR: expected table name\n");
        return;
    }
    let tname_start = p.tok_start;
    let tname_len = p.tok_len;

    let tidx = match find_table(db, &p.input[tname_start..], tname_len) {
        Some(i) => i,
        None => {
            copy_to(resp, rlen, b"ERROR: table not found\n");
            return;
        }
    };

    // Zero out the table
    let table = &mut db.tables[tidx];
    table.occupied = false;
    table.name_len = 0;
    table.col_count = 0;
    table.row_count = 0;
    let mut row = 0;
    while row < MAX_ROWS {
        table.row_occupied[row] = false;
        row += 1;
    }
    db.table_count -= 1;

    copy_to(resp, rlen, b"OK: table dropped\n");
}

// ---------------------------------------------------------------------------
// SQL dispatcher
// ---------------------------------------------------------------------------

fn exec_sql(input: &[u8], input_len: usize, db: &mut Database, resp: &mut [u8], rlen: &mut usize) {
    let mut parser = Parser {
        input,
        pos: 0,
        len: input_len,
        tok: Token::Eof,
        tok_start: 0,
        tok_len: 0,
        int_val: 0,
        str_buf: [0; MAX_TEXT_LEN],
        str_len: 0,
    };

    next_token(&mut parser);
    match parser.tok {
        Token::Kw(Keyword::Create) => exec_create_table(&mut parser, db, resp, rlen),
        Token::Kw(Keyword::Insert) => exec_insert(&mut parser, db, resp, rlen),
        Token::Kw(Keyword::Select) => exec_select(&mut parser, db, resp, rlen),
        Token::Kw(Keyword::Update) => exec_update(&mut parser, db, resp, rlen),
        Token::Kw(Keyword::Delete) => exec_delete(&mut parser, db, resp, rlen),
        Token::Kw(Keyword::Drop) => exec_drop_table(&mut parser, db, resp, rlen),
        _ => copy_to(resp, rlen, b"ERROR: unknown command\n"),
    }
}

// ---------------------------------------------------------------------------
// HTTP server + routes
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-sql-db: socket() failed\n");
            libc::exit(1);
        }

        let optval: i32 = 1;
        libc::setsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &optval as *const i32 as *const libc::c_void,
            4,
        );

        let addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: PORT.to_be(),
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };

        if libc::bind(
            sock,
            &addr as *const _ as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-sql-db: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-sql-db: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-sql-db listening on port 7881\n");

        // Allocate database via mmap
        let db_size = core::mem::size_of::<Database>();
        let db_ptr = libc::mmap(
            core::ptr::null_mut(),
            db_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if db_ptr == libc::MAP_FAILED {
            write_all(2, b"tiny-sql-db: mmap() failed\n");
            libc::exit(1);
        }
        let db = &mut *(db_ptr as *mut Database);

        loop {
            let client = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
            if client < 0 {
                continue;
            }

            // Read request
            let mut req_buf = [0u8; 8192];
            let mut req_len: usize = 0;

            loop {
                if req_len >= req_buf.len() {
                    break;
                }
                let n = libc::read(
                    client,
                    req_buf.as_mut_ptr().add(req_len) as *mut libc::c_void,
                    req_buf.len() - req_len,
                );
                if n <= 0 {
                    break;
                }
                req_len += n as usize;
                if find_header_end(&req_buf, req_len).is_some() {
                    break;
                }
            }

            let header_end = match find_header_end(&req_buf, req_len) {
                Some(end) => end,
                None => {
                    write_all(client, b"HTTP/1.1 400 Bad Request\r\n\r\n");
                    libc::close(client);
                    continue;
                }
            };

            let (method_end, path_start, path_end) = parse_request_line(&req_buf, req_len);

            if method_is(&req_buf, method_end, b"POST")
                && path_eq(&req_buf, path_start, path_end, b"/sql")
            {
                // Read body
                let content_length = parse_content_length(&req_buf, header_end);
                // Ensure we have the full body
                while req_len < header_end + content_length && req_len < req_buf.len() {
                    let n = libc::read(
                        client,
                        req_buf.as_mut_ptr().add(req_len) as *mut libc::c_void,
                        req_buf.len() - req_len,
                    );
                    if n <= 0 {
                        break;
                    }
                    req_len += n as usize;
                }

                let body_start = header_end;
                let body_len = if content_length > 0 {
                    if content_length < req_len - header_end {
                        content_length
                    } else {
                        req_len - header_end
                    }
                } else {
                    req_len - header_end
                };

                let mut resp = [0u8; RESP_SIZE];
                let mut rlen: usize = 0;

                exec_sql(&req_buf[body_start..], body_len, db, &mut resp, &mut rlen);

                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
            } else if method_is(&req_buf, method_end, b"GET")
                && path_eq(&req_buf, path_start, path_end, b"/tables")
            {
                let mut resp = [0u8; 512];
                let mut rlen: usize = 0;
                let mut i = 0;
                while i < MAX_TABLES {
                    if db.tables[i].occupied {
                        copy_to(&mut resp, &mut rlen, &db.tables[i].name[..db.tables[i].name_len]);
                        copy_to(&mut resp, &mut rlen, b"\n");
                    }
                    i += 1;
                }
                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
            } else if method_is(&req_buf, method_end, b"GET")
                && path_eq(&req_buf, path_start, path_end, b"/stats")
            {
                let mut resp = [0u8; 512];
                let mut rlen: usize = 0;
                let mut num_buf = [0u8; 10];

                copy_to(&mut resp, &mut rlen, b"tables: ");
                let n = format_u32(db.table_count as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\n");

                let mut i = 0;
                while i < MAX_TABLES {
                    if db.tables[i].occupied {
                        copy_to(&mut resp, &mut rlen, &db.tables[i].name[..db.tables[i].name_len]);
                        copy_to(&mut resp, &mut rlen, b": ");
                        let n = format_u32(db.tables[i].row_count as u32, &mut num_buf);
                        copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                        copy_to(&mut resp, &mut rlen, b" rows\n");
                    }
                    i += 1;
                }
                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
            } else {
                send_response(client, b"404 Not Found", b"text/plain", b"not found\n");
            }

            libc::close(client);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
