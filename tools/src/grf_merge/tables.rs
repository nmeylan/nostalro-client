//! Folds the key-addressed `data/*.txt` tables of a layer stack into one file.
//!
//! Records are spliced as raw bytes. These tables are EUC-KR, where a trail
//! byte is `0x41-0x5A`, `0x61-0x7A` or `0x81-0xFE`, so `#`, `/`, the digits and
//! the newlines can never be the second half of a character and splitting on
//! them needs no decoding.

use ragnarok_resources::table;
use std::collections::HashSet;
use std::ops::Range;

/// How a table delimits its records and where the key sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// One record per line, key in the nth `#`-separated field.
    LineKeyed { key_field: usize },
    /// A `#`-delimited stream of fixed-size records, key in the first field.
    TokenChunk { arity: usize },
    /// A `#`-delimited stream where a token matching `key` opens a record and
    /// the tokens after it are its body, up to the next key.
    KeyedRun { key: KeyKind },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// What `parse_item_description_table` treats as an id.
    Numeric,
    /// What `parse_skill_description_table` treats as a skill name.
    Identifier,
}

/// The tables we know how to fold. Everything absent from this list — most
/// notably `msgstringtable.txt`, whose id is a record's position rather than a
/// field of it — keeps the first archive's copy whole.
pub const MERGED: &[(&str, Shape)] = &[
    (table::RES_NAME, Shape::LineKeyed { key_field: 0 }),
    (table::MAP_NAME, Shape::LineKeyed { key_field: 0 }),
    (table::MP3_NAME, Shape::LineKeyed { key_field: 0 }),
    (table::INDOOR_RSW, Shape::LineKeyed { key_field: 0 }),
    (
        table::IDENTIFIED_ITEM_NAME,
        Shape::LineKeyed { key_field: 0 },
    ),
    (
        table::UNIDENTIFIED_ITEM_NAME,
        Shape::LineKeyed { key_field: 0 },
    ),
    (
        table::IDENTIFIED_ITEM_RESOURCE,
        Shape::LineKeyed { key_field: 0 },
    ),
    (
        table::UNIDENTIFIED_ITEM_RESOURCE,
        Shape::LineKeyed { key_field: 0 },
    ),
    (table::ITEM_SLOT_COUNT, Shape::LineKeyed { key_field: 0 }),
    (
        table::CARD_ILLUSTRATION_NAME,
        Shape::LineKeyed { key_field: 0 },
    ),
    (table::CARD_PREFIX_NAME, Shape::LineKeyed { key_field: 0 }),
    (table::CARD_POSTFIX_NAME, Shape::LineKeyed { key_field: 0 }),
    (table::SKILL_NAME, Shape::LineKeyed { key_field: 0 }),
    // `region#name#left#top#right#bottom#` — the map name is the key.
    (table::MAP_POSITION, Shape::LineKeyed { key_field: 1 }),
    (table::QUEST_DISPLAY, Shape::TokenChunk { arity: 6 }),
    (table::FOG_PARAMETER, Shape::TokenChunk { arity: 5 }),
    (
        table::IDENTIFIED_ITEM_DESC,
        Shape::KeyedRun {
            key: KeyKind::Numeric,
        },
    ),
    (
        table::UNIDENTIFIED_ITEM_DESC,
        Shape::KeyedRun {
            key: KeyKind::Numeric,
        },
    ),
    (
        table::SKILL_DESC,
        Shape::KeyedRun {
            key: KeyKind::Identifier,
        },
    ),
];

/// Folds `layers` in declaration order: a key several layers define keeps the
/// earliest layer's record, and keys only later layers define are appended in
/// their own order. Content carrying no key — comments, blank lines — is taken
/// from the first layer alone.
pub fn merge(shape: Shape, layers: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    let line_delimited = matches!(shape, Shape::LineKeyed { .. });

    for (index, data) in layers.iter().enumerate() {
        for record in records(shape, data) {
            match record.key {
                None if index > 0 => continue,
                Some(ref key) if !seen.insert(key.clone()) => continue,
                _ => {}
            }
            // A `#` separates token-delimited records but not line-keyed ones,
            // and an archive's last line need not end in a newline.
            match out.last() {
                None | Some(b'\n') => {}
                Some(b'#') if !line_delimited => {}
                Some(_) => out.push(b'\n'),
            }
            let mut span = record.span;
            if out.last() == Some(&b'\n') {
                // A token-delimited record starts after the `#` that closed the
                // previous one, so it carries that line's break into its span.
                while span.start < span.end && matches!(data[span.start], b'\n' | b'\r') {
                    span.start += 1;
                }
            }
            out.extend_from_slice(&data[span]);
        }
    }
    if out.last().is_some_and(|b| *b != b'\n') {
        out.push(b'\n');
    }
    out
}

struct Record {
    /// `None` for a span that holds no record: a comment, a blank line, or a
    /// trailing fragment too short to make one.
    key: Option<Vec<u8>>,
    span: Range<usize>,
}

fn records(shape: Shape, data: &[u8]) -> Vec<Record> {
    match shape {
        Shape::LineKeyed { key_field } => line_records(data, key_field),
        Shape::TokenChunk { arity } => chunk_records(data, arity),
        Shape::KeyedRun { key } => run_records(data, key),
    }
}

fn line_records(data: &[u8], key_field: usize) -> Vec<Record> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < data.len() {
        let end = match data[start..].iter().position(|b| *b == b'\n') {
            Some(offset) => start + offset + 1,
            None => data.len(),
        };
        let line = &data[start..end];
        let key = (!is_blank_or_comment(line))
            .then(|| {
                line.split(|b| *b == b'#')
                    .nth(key_field)
                    .map(normalize_key)
                    .filter(|k| !k.is_empty())
            })
            .flatten();
        out.push(Record {
            key,
            span: start..end,
        });
        start = end;
    }
    out
}

fn chunk_records(data: &[u8], arity: usize) -> Vec<Record> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut key: Option<Vec<u8>> = None;
    let mut fields = 0;

    for token in tokens(data) {
        // Every token is a field, empty ones included: a quest with no
        // description still ends in `#`, and skipping it would shift the rest
        // of the file by one field.
        if fields == 0 {
            key = Some(normalize_key(&significant_text(
                &data[content(&token, data)],
            )))
            .filter(|k| !k.is_empty());
        }
        fields += 1;
        if fields == arity {
            out.push(Record {
                key: key.take(),
                span: start..token.end,
            });
            start = token.end;
            fields = 0;
        }
    }

    if start < data.len() {
        out.push(Record {
            key: None,
            span: start..data.len(),
        });
    }
    out
}

fn run_records(data: &[u8], kind: KeyKind) -> Vec<Record> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut key: Option<Vec<u8>> = None;

    for token in tokens(data) {
        let text = &data[content(&token, data)];
        if !is_key_token(text, kind) {
            continue;
        }
        if token.start > start {
            out.push(Record {
                key: key.take(),
                span: start..token.start,
            });
        }
        key = Some(normalize_key(&significant_text(text)));
        start = token.start;
    }

    if start < data.len() {
        out.push(Record {
            key,
            span: start..data.len(),
        });
    }
    out
}

/// The `#`-separated spans of `data`, each including its own trailing `#`.
fn tokens(data: &[u8]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, byte) in data.iter().enumerate() {
        if *byte == b'#' {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    if start < data.len() {
        out.push(start..data.len());
    }
    out
}

/// A token's span without the `#` that closes it.
fn content(token: &Range<usize>, data: &[u8]) -> Range<usize> {
    if data[token.end - 1] == b'#' {
        token.start..token.end - 1
    } else {
        token.clone()
    }
}

/// Mirrors what the runtime parsers accept as the token that opens a record:
/// one significant line, holding an id or a skill name.
fn is_key_token(token: &[u8], kind: KeyKind) -> bool {
    let lines: Vec<&[u8]> = token
        .split(|b| *b == b'\n')
        .map(trim)
        .filter(|line| !line.is_empty() && !line.starts_with(b"//"))
        .collect();
    let [text] = lines[..] else {
        return false;
    };
    match kind {
        KeyKind::Numeric => std::str::from_utf8(text).is_ok_and(|s| s.parse::<u16>().is_ok()),
        KeyKind::Identifier => text.contains(&b'_') && !text.iter().any(u8::is_ascii_whitespace),
    }
}

/// A token's content with comment lines and surrounding whitespace removed,
/// used to decide what the token is. Never written to the output.
fn significant_text(token: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in token.split(|b| *b == b'\n') {
        if is_blank_or_comment(line) {
            continue;
        }
        out.extend_from_slice(trim(line));
    }
    trim(&out).to_vec()
}

fn is_blank_or_comment(line: &[u8]) -> bool {
    let trimmed = trim(line);
    trimmed.is_empty() || trimmed.starts_with(b"//")
}

fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &bytes[start..end]
}

/// Case-folds the ASCII halves of a key, stepping over EUC-KR characters whole:
/// a trail byte in `0x41-0x5A` is an ASCII capital that must not be touched.
fn normalize_key(field: &[u8]) -> Vec<u8> {
    let key = trim(field);
    let mut out = Vec::with_capacity(key.len());
    let mut i = 0;
    while i < key.len() {
        if key[i] >= 0x81 && i + 1 < key.len() && is_trail_byte(key[i + 1]) {
            out.extend_from_slice(&key[i..i + 2]);
            i += 2;
        } else {
            out.push(key[i].to_ascii_lowercase());
            i += 1;
        }
    }
    out
}

fn is_trail_byte(byte: u8) -> bool {
    matches!(byte, 0x41..=0x5A | 0x61..=0x7A | 0x81..=0xFE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `갂` is CP949 `81 41`: its trail byte is an ASCII `A`, so anything that
    /// decoded or cased these bytes would corrupt it.
    const KOREAN: &[u8] = b"\x81\x41";

    fn merged(shape: Shape, first: &[u8], second: &[u8]) -> Vec<u8> {
        merge(shape, &[first.to_vec(), second.to_vec()])
    }

    #[test]
    fn key_addressed_tables_keep_the_first_layer_and_append_the_rest() {
        // No trailing newline: the stock `indoorrswtable.txt` ends that way.
        let first = [
            b"// base\n501#Red Potion#\n1201#Knife#\n1202#".as_slice(),
            KOREAN,
            b"#",
        ]
        .concat();
        let second = b"// patch\n501#Blue Potion#\n30000#Custom Blade#\n";

        let out = merged(Shape::LineKeyed { key_field: 0 }, &first, second);

        assert_eq!(
            out,
            [
                b"// base\n501#Red Potion#\n1201#Knife#\n1202#".as_slice(),
                KOREAN,
                b"#\n30000#Custom Blade#\n",
            ]
            .concat()
        );
    }

    #[test]
    fn map_position_keys_on_the_name_not_the_region() {
        let out = merged(
            Shape::LineKeyed { key_field: 1 },
            b"1#prontera#10#10#20#20#\n",
            b"3#prontera#99#99#99#99#\n2#lutie#30#30#40#40#\n",
        );

        assert_eq!(out, b"1#prontera#10#10#20#20#\n2#lutie#30#30#40#40#\n");
    }

    #[test]
    fn fixed_arity_records_span_lines() {
        let out = merged(
            Shape::TokenChunk { arity: 5 },
            b"prontera#0.5#\n1.0#0x66ccff#0.8#\n",
            b"prontera#9.9#9.9#0x000000#9.9#\nlutie#0.2#0.9#0xffffff#0.5#\n",
        );

        assert_eq!(
            out,
            b"prontera#0.5#\n1.0#0x66ccff#0.8#\nlutie#0.2#0.9#0xffffff#0.5#\n"
        );
    }

    #[test]
    fn description_bodies_may_contain_a_separator() {
        let out = merged(
            Shape::KeyedRun {
                key: KeyKind::Numeric,
            },
            b"501#A potion.\nHeals #45 HP.#\n",
            b"501#Overridden.#\n502#A new card.#\n",
        );

        assert_eq!(out, b"501#A potion.\nHeals #45 HP.#\n502#A new card.#\n");
    }
}
