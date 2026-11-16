//! Reader for SCIP indexes (github.com/sourcegraph/scip, `scip.proto`), the format compiler-grade indexers write. Only the
//! fields Footnote uses are decoded; protobuf's wire format is short enough to read by hand, so no protobuf dependency.
use anyhow::{Result, bail};

#[derive(Debug, Default)]
pub struct Occurrence {
    /// `[start_line, start_col, end_line, end_col]`, 0-based (a 3-element SCIP range is one line).
    pub range: [i32; 4],
    pub symbol: String,
    pub roles: i32,
    /// For a definition, the whole item (a function's body); absent on some references.
    pub enclosing: Option<[i32; 4]>,
}

#[derive(Debug, Default)]
pub struct Symbol {
    pub symbol: String,
    pub display_name: String,
    pub signature: String,
}

#[derive(Debug, Default)]
pub struct Document {
    pub path: String,
    pub occurrences: Vec<Occurrence>,
    pub symbols: Vec<Symbol>,
}

/// Role bit for a definition (`SymbolRole.Definition`).
pub const DEFINITION: i32 = 1;

enum Value<'a> {
    Int(u64),
    Bytes(&'a [u8]),
}

fn varint(b: &[u8], i: &mut usize) -> Result<u64> {
    let (mut v, mut shift) = (0u64, 0);
    loop {
        let Some(&x) = b.get(*i) else { bail!("truncated SCIP varint") };
        *i += 1;
        if shift > 63 {
            bail!("SCIP varint too long");
        }
        v |= u64::from(x & 0x7f) << shift;
        if x < 0x80 {
            return Ok(v);
        }
        shift += 7;
    }
}

/// Every (field number, value) of one message, in order.
fn fields(b: &[u8]) -> Result<Vec<(u64, Value<'_>)>> {
    let (mut out, mut i) = (Vec::new(), 0);
    while i < b.len() {
        let key = varint(b, &mut i)?;
        let v = match key & 7 {
            0 => Value::Int(varint(b, &mut i)?),
            2 => {
                let n = usize::try_from(varint(b, &mut i)?)?;
                let Some(s) = b.get(i..i.saturating_add(n)) else { bail!("truncated SCIP field") };
                i += n;
                Value::Bytes(s)
            }
            1 => {
                i += 8;
                continue;
            }
            5 => {
                i += 4;
                continue;
            }
            w => bail!("unsupported protobuf wire type {w}"),
        };
        out.push((key >> 3, v));
    }
    Ok(out)
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// A repeated int32, packed (one bytes field) or not (one varint per field).
fn ints(acc: &mut Vec<i32>, v: &Value) -> Result<()> {
    match v {
        Value::Int(x) => acc.push(*x as i32),
        Value::Bytes(b) => {
            let mut i = 0;
            while i < b.len() {
                acc.push(varint(b, &mut i)? as i32);
            }
        }
    }
    Ok(())
}

fn range(r: &[i32]) -> Result<[i32; 4]> {
    match *r {
        [l, c, e] => Ok([l, c, l, e]),
        [l, c, el, ec] => Ok([l, c, el, ec]),
        _ => bail!("SCIP range with {} numbers", r.len()),
    }
}

fn occurrence(b: &[u8]) -> Result<Occurrence> {
    let (mut o, mut r, mut enc) = (Occurrence::default(), Vec::new(), Vec::new());
    for (f, v) in fields(b)? {
        match (f, &v) {
            (1, _) => ints(&mut r, &v)?,
            (2, Value::Bytes(s)) => o.symbol = text(s),
            (3, Value::Int(x)) => o.roles = *x as i32,
            (7, _) => ints(&mut enc, &v)?,
            _ => {}
        }
    }
    o.range = range(&r)?;
    o.enclosing = (!enc.is_empty()).then(|| range(&enc)).transpose()?;
    Ok(o)
}

fn symbol(b: &[u8]) -> Result<Symbol> {
    let mut s = Symbol::default();
    for (f, v) in fields(b)? {
        match (f, v) {
            (1, Value::Bytes(x)) => s.symbol = text(x),
            (6, Value::Bytes(x)) => s.display_name = text(x),
            // signature_documentation is a Document; its `text` (field 5) holds the signature.
            (7, Value::Bytes(x)) => {
                for (g, w) in fields(x)? {
                    if let (5, Value::Bytes(t)) = (g, w) {
                        s.signature = text(t);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(s)
}

/// The documents of a SCIP `Index` (field 2); metadata and external symbols are not needed.
pub fn read(index: &[u8]) -> Result<Vec<Document>> {
    let mut docs = Vec::new();
    for (f, v) in fields(index)? {
        let (2, Value::Bytes(d)) = (f, v) else { continue };
        let mut doc = Document::default();
        for (g, w) in fields(d)? {
            match (g, w) {
                (1, Value::Bytes(p)) => doc.path = text(p).replace('\\', "/"),
                (2, Value::Bytes(o)) => doc.occurrences.push(occurrence(o)?),
                (3, Value::Bytes(s)) => doc.symbols.push(symbol(s)?),
                _ => {}
            }
        }
        docs.push(doc);
    }
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(n: u8, bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![n << 3 | 2, bytes.len() as u8];
        v.extend_from_slice(bytes);
        v
    }

    #[test]
    fn reads_a_hand_built_index() {
        // Occurrence: packed range [3, 4, 9] (one line), symbol "s", roles 1, packed enclosing [3, 0, 7, 1].
        let mut occ = field(1, &[3, 4, 9]);
        occ.extend(field(2, b"s"));
        occ.extend([3 << 3, 1]);
        occ.extend(field(7, &[3, 0, 7, 1]));
        let mut sym = field(1, b"s");
        sym.extend(field(6, b"main"));
        sym.extend(field(7, &field(5, b"fn main()")));
        let mut doc = field(1, br"src\main.rs");
        doc.extend(field(2, &occ));
        doc.extend(field(3, &sym));
        let docs = read(&field(2, &doc)).unwrap();
        assert_eq!(docs[0].path, "src/main.rs");
        let o = &docs[0].occurrences[0];
        assert_eq!((o.range, o.symbol.as_str(), o.roles, o.enclosing), ([3, 4, 3, 9], "s", 1, Some([3, 0, 7, 1])));
        assert_eq!((docs[0].symbols[0].display_name.as_str(), docs[0].symbols[0].signature.as_str()), ("main", "fn main()"));
        assert!(read(&[0x12, 0x05, 1]).is_err(), "truncated input is an error, not a panic");
    }
}
