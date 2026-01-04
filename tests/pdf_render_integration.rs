use assert_cmd::Command;
use lopdf::content::Content;
use lopdf::{Document, Object};
use markdown2pdf::markdown::{Lexer, Token};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;

fn pdf_unescape_paren_string(s: &str) -> Vec<u8> {
    // Very small PDF string unescape: handles \\ \( \) and octal \ooo
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next.is_digit(8) {
                    // octal sequence up to 3 digits
                    let mut oct = String::new();
                    while oct.len() < 3 {
                        if let Some(&d) = chars.peek() {
                            if d.is_digit(8) {
                                oct.push(d);
                                chars.next();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if let Ok(v) = u8::from_str_radix(&oct, 8) {
                        out.push(v);
                    }
                } else {
                    // simple escapes
                    match chars.next().unwrap() {
                        'n' => out.push(b'\n'),
                        'r' => out.push(b'\r'),
                        't' => out.push(b'\t'),
                        '\\' => out.push(b'\\'),
                        '(' => out.push(b'('),
                        ')' => out.push(b')'),
                        other => out.push(other as u8),
                    }
                }
            }
        } else {
            out.push(c as u8);
        }
    }
    out
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        return pdf_unescape_paren_string(inner);
    }
    let mut s = s.trim_start_matches('<').trim_end_matches('>').to_string();
    // Remove any whitespace inside hex (e.g., '<00 41>')
    s.retain(|c| !c.is_whitespace());
    // If odd length, pad with a leading zero to avoid panics and interpret correctly.
    if s.len() % 2 == 1 {
        s = format!("0{}", s);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0))
        .collect()
}

fn bytes_to_unicode_string(bytes: &[u8]) -> String {
    if bytes.len() % 2 == 0 {
        // Interpret as UTF-16BE
        let mut out = String::new();
        for chunk in bytes.chunks(2) {
            let code = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
            if let Some(ch) = std::char::from_u32(code) {
                out.push(ch);
            } else {
                out.push('\u{FFFD}');
            }
        }
        out
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

fn parse_cmap(cmap: &str) -> HashMap<Vec<u8>, String> {
    let mut map = HashMap::new();
    let mut lines = cmap.lines();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.ends_with("beginbfchar") {
            // Collect until endbfchar
            while let Some(l) = lines.next() {
                let l = l.trim();
                if l.ends_with("endbfchar") {
                    break;
                }
                // Expect lines like: <01> <0041>
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 2 {
                    let key = hex_to_bytes(parts[0]);
                    let val = hex_to_bytes(parts[1]);
                    map.insert(key, bytes_to_unicode_string(&val));
                }
            }
        } else if line.ends_with("beginbfrange") {
            // Collect until endbfrange
            while let Some(l) = lines.next() {
                let l = l.trim();
                if l.ends_with("endbfrange") {
                    break;
                }
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 3 {
                    let start = hex_to_bytes(parts[0]);
                    let end = hex_to_bytes(parts[1]);
                    let start_val = parts[2];

                    // Convert start/end to integer
                    let start_num = start.iter().fold(0u32, |acc, &b| (acc << 8) + b as u32);
                    let end_num = end.iter().fold(0u32, |acc, &b| (acc << 8) + b as u32);

                    if start_val.starts_with('<') {
                        // sequential mapping
                        let dest_start_bytes = hex_to_bytes(start_val);
                        let mut dest_start_num = dest_start_bytes
                            .iter()
                            .fold(0u32, |acc, &b| (acc << 8) + b as u32);
                        for code in start_num..=end_num {
                            // key bytes length = start.len()
                            let mut key = Vec::new();
                            let v = code;
                            for i in 0..start.len() {
                                // produce big endian
                                let shift = 8 * (start.len() - 1 - i);
                                key.push(((v >> shift) & 0xff) as u8);
                            }
                            // dest_start_num to bytes
                            let mut dest_bytes = Vec::new();
                            for i in (0..dest_start_bytes.len()).rev() {
                                // big endian
                                let shift = 8 * i;
                                dest_bytes.push(((dest_start_num >> shift) & 0xff) as u8);
                            }
                            map.insert(key, bytes_to_unicode_string(&dest_bytes));
                            dest_start_num += 1;
                        }
                    } else if start_val.starts_with('[') {
                        // explicit mapping list, but here we will parse naive split
                        let list = l[l.find('[').unwrap() + 1..l.rfind(']').unwrap()].trim();
                        let mut idx = start_num;
                        for item in list.split_whitespace() {
                            let key_num = idx;
                            let mut key = Vec::new();
                            for i in 0..start.len() {
                                let shift = 8 * (start.len() - 1 - i);
                                key.push(((key_num >> shift) & 0xff) as u8);
                            }
                            let val_bytes = hex_to_bytes(item);
                            map.insert(key, bytes_to_unicode_string(&val_bytes));
                            idx += 1;
                        }
                    }
                }
            }
        }
    }
    map
}

fn build_global_cmap_map(doc: &Document) -> HashMap<Vec<u8>, String> {
    let mut global = HashMap::new();
    for (_id, obj) in doc.objects.iter() {
        if let Object::Stream(stream) = obj {
            // Try to get decompressed content; if no Filter is present or decompression fails,
            // fall back to using the raw stream bytes (they are sometimes already unfiltered).
            let bytes = match stream.decompressed_content() {
                Ok(d) => d,
                Err(_) => stream.content.clone(),
            };
            if let Ok(cmap_str) = String::from_utf8(bytes.clone()) {
                if cmap_str.contains("beginbfchar") || cmap_str.contains("beginbfrange") {
                    let map = parse_cmap(&cmap_str);
                    for (k, v) in map {
                        global.insert(k, v);
                    }
                }
            }
            // Also check whether this stream contains the header as ASCII or UTF-16BE
            let ascii_lossy = String::from_utf8_lossy(&bytes).to_string();
            if ascii_lossy.contains("Test Snippets for Code Highlighting") {
                println!("debug: found header text in raw stream (ASCII lossily)");
            }
            // Try decoding as UTF-16BE across the stream
            if bytes.len() > 4 {
                let mut u16chars = String::new();
                for chunk in bytes.chunks(2) {
                    if chunk.len() == 2 {
                        let code = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
                        if let Some(ch) = std::char::from_u32(code) {
                            u16chars.push(ch);
                        }
                    }
                }
                if u16chars.contains("Test Snippets for Code Highlighting") {
                    println!("debug: found header in stream when interpreting as UTF-16BE");
                }
            }
        }
    }
    // Debug: print some summary info about the extracted cmap
    println!("debug: global cmap entries = {}", global.len());
    let has_t = global.values().any(|v| v.contains('T'));
    println!("debug: global cmap has 'T' in any value? {}", has_t);
    // Print some representative mappings for ASCII letters
    for (k, v) in global.iter().take(20) {
        println!("debug: cmap sample key={:02X?} val={:?}", k, v);
    }
    // Print keys that map to 'T' or 't' for inspection
    for (k, v) in global.iter() {
        if v == "T" || v == "t" || v == "\u{0054}" {
            println!("debug: cmap maps to T: key={:02X?} -> {:?}", k, v);
            break;
        }
    }
    global
}

fn collapse_single_char_sequences(s: &str) -> String {
    // Treat tokens that contain exactly one Unicode scalar (one char) as single-char tokens.
    // This avoids mis-classifying multi-byte UTF-8 characters (e.g., accented letters).
    let parts: Vec<&str> = s.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        if parts[i].chars().count() == 1 {
            // collect run of single-char tokens and join them into a single word
            let mut run = String::new();
            while i < parts.len() && parts[i].chars().count() == 1 {
                run.push_str(parts[i]);
                i += 1;
            }
            out.push(run);
        } else {
            out.push(parts[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

fn rot_n(s: &str, shift: i8) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_lowercase() {
                let a = b'a' as i8;
                let x = (c as i8 - a + shift).rem_euclid(26) + a;
                x as u8 as char
            } else if c.is_ascii_uppercase() {
                let a = b'A' as i8;
                let x = (c as i8 - a + shift).rem_euclid(26) + a;
                x as u8 as char
            } else {
                c
            }
        })
        .collect()
}

fn extract_text_with_to_unicode(
    path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let doc = Document::load(path)?;
    let mut text = String::new();

    let global_map = build_global_cmap_map(&doc);

    for (_page_num, &page_id) in doc.get_pages().iter() {
        let contents = doc.get_page_content(page_id)?;
        let content = Content::decode(&contents)?;

        for operation in content.operations {
            match operation.operator.as_ref() {
                "Tj" => {
                    if let Some(obj) = operation.operands.get(0) {
                        if let Object::String(ref bytes, _) = obj {
                            // Debug: print raw bytes hex for this Tj
                            println!(
                                "debug: Tj raw bytes hex: {:02X?}",
                                &bytes[..std::cmp::min(bytes.len(), 64)]
                            );

                            // Direct full-bytes lookup
                            let full_key = bytes.to_vec();
                            println!("debug: direct full lookup: {:?}", global_map.get(&full_key));

                            // Try simple fixed-size chunk lookups (e.g., 2-byte chunks)
                            let mut two_byte_chunks = Vec::new();
                            let mut j = 0;
                            while j + 1 < bytes.len() {
                                two_byte_chunks.push(bytes[j..j + 2].to_vec());
                                j += 2;
                            }
                            for (idx, chunk) in two_byte_chunks.iter().enumerate().take(10) {
                                println!(
                                    "debug: two-byte chunk[{}] = {:02X?} -> {:?}",
                                    idx,
                                    chunk,
                                    global_map.get(chunk)
                                );
                            }

                            // Try UTF-16BE interpretation for whole string
                            let utf16_whole = bytes_to_unicode_string(&bytes);
                            println!(
                                "debug: utf16 whole interpretation: {:?}",
                                utf16_whole.chars().take(200).collect::<String>()
                            );

                            // Try single-byte ASCII interpretation
                            let ascii_single: String = bytes.iter().map(|&b| b as char).collect();
                            println!(
                                "debug: ascii single-byte interpretation: {:?}",
                                ascii_single.chars().take(200).collect::<String>()
                            );

                            // decode using global cmap map heuristics
                            // Use a longest-match strategy based on available cmap key lengths
                            let max_key_len = global_map.keys().map(|k| k.len()).max().unwrap_or(1);
                            println!("debug: max cmap key len = {}", max_key_len);
                            let mut i = 0;
                            let mut match_count = 0;
                            while i < bytes.len() {
                                let mut matched = false;
                                let max_try = std::cmp::min(max_key_len, bytes.len() - i);
                                for len in (1..=max_try).rev() {
                                    let key = bytes[i..i + len].to_vec();
                                    if let Some(val) = global_map.get(&key) {
                                        // Debug: print matches to observe mapping
                                        if match_count < 200 {
                                            println!(
                                                "debug: matched key hex: {:02X?} -> value: {:?}",
                                                key, val
                                            );
                                        }
                                        match_count += 1;
                                        text.push_str(val);
                                        i += len;
                                        matched = true;
                                        break;
                                    }
                                }
                                if !matched {
                                    // Fallback: try interpreting as UTF-16BE when even length, else single-byte
                                    let key = bytes[i..i + 1].to_vec();
                                    // Debug: print the raw byte we'll fallback on
                                    println!("debug: fallback raw byte hex: <{:02x}>", key[0]);
                                    text.push_str(&bytes_to_unicode_string(&key));
                                    i += 1;
                                }
                            }
                            println!(
                                "debug: decoded Tj segment: {}",
                                &text
                                    .chars()
                                    .rev()
                                    .take(200)
                                    .collect::<String>()
                                    .chars()
                                    .rev()
                                    .collect::<String>()
                            );
                            text.push(' ');
                        }
                    }
                }
                "TJ" => {
                    if let Some(Object::Array(arr)) = operation.operands.get(0) {
                        for elem in arr {
                            if let Object::String(ref bytes, _) = elem {
                                // Longest-match strategy as in Tj above
                                let max_key_len =
                                    global_map.keys().map(|k| k.len()).max().unwrap_or(1);
                                let mut i = 0;
                                while i < bytes.len() {
                                    let mut matched = false;
                                    let max_try = std::cmp::min(max_key_len, bytes.len() - i);
                                    for len in (1..=max_try).rev() {
                                        let key = bytes[i..i + len].to_vec();
                                        if let Some(val) = global_map.get(&key) {
                                            text.push_str(val);
                                            i += len;
                                            matched = true;
                                            break;
                                        }
                                    }
                                    if !matched {
                                        let key = bytes[i..i + 1].to_vec();
                                        text.push_str(&bytes_to_unicode_string(&key));
                                        i += 1;
                                    }
                                }
                                text.push(' ');
                            }
                        }
                        text.push(' ');
                    }
                }
                _ => {}
            }
        }
    }

    Ok(text)
}

#[test]
fn test_markdown_tokens_contain_header() {
    // Read the test snippet file and parse tokens directly (independent of PDF encoding)
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_md = repo_root.join("tests").join("test_snippets.md");
    let md = std::fs::read_to_string(&test_md).expect("read test_snippets.md");

    // Use internal lexer to ensure the header is parsed into tokens
    let mut lexer = Lexer::new(md);
    let tokens = lexer.parse().expect("parse markdown into tokens");
    let all_text = Token::collect_all_text(&tokens);
    assert!(
        all_text.contains("Test Snippets for Code Highlighting"),
        "Tokens do not contain expected header"
    );
}
