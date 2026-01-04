use lopdf::{Document, Object};
use std::env;

fn dump_stream(obj: &Object) {
    match obj {
        Object::Stream(s) => match s.decompressed_content() {
            Ok(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(s) => println!("--- stream text start ---\n{}\n--- stream text end ---", s),
                Err(_) => println!(
                    "--- stream hex start ---\n{:02X?}\n--- stream hex end ---",
                    bytes
                ),
            },
            Err(e) => {
                println!("failed to decompress stream: {:?}", e);
                println!("raw stream dict: {:?}", s.dict);
                // print raw content length and a short hex preview (without attempting to decompress)
                println!("raw content len: {}", s.content.len());
                let preview_len = std::cmp::min(s.content.len(), 256);
                println!(
                    "raw content preview (hex): {:02X?}",
                    &s.content[..preview_len]
                ); // Print raw content lossily as text (first N chars) to see textual CMap content
                let text_preview = String::from_utf8_lossy(&s.content[..preview_len]);
                println!("raw content preview (text lossily): {}", text_preview); // print the Filter entry if present
                match s.dict.get(b"Filter") {
                    Ok(f) => println!("Filter entry: {:?}", f),
                    Err(_) => println!("Filter entry: <not present>"),
                }
            }
        },
        _ => println!("not a stream: {:?}", obj),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: inspect_pdf <file.pdf>");
        std::process::exit(2);
    }
    let doc = match Document::load(&args[1]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("failed to load: {:?}", e);
            std::process::exit(1)
        }
    };

    println!("PDF version: {:?}", doc.version);
    println!("Trailer: {:?}", doc.trailer);
    // Try to read Info dictionary Title/Author/Subject
    if let Ok(info_obj) = doc.trailer.get(b"Info") {
        match info_obj {
            Object::Reference(info_ref) => {
                if let Ok(obj) = doc.get_object(*info_ref) {
                    if let Object::Dictionary(d) = obj {
                        if let Ok(title) = d.get(b"Title") {
                            println!("Info.Title: {:?}", title);
                        }
                        if let Ok(author) = d.get(b"Author") {
                            println!("Info.Author: {:?}", author);
                        }
                    }
                }
            }
            other => println!("Info key present but not a reference: {:?}", other),
        }
    } else {
        println!("No Info dictionary present in trailer");
    }

    // List objects that look like ToUnicode or Font dicts
    for (id, object) in doc.objects.iter() {
        match object {
            Object::Dictionary(dict) => {
                // ToUnicode key
                if dict.get(b"ToUnicode").is_ok() {
                    println!("Object {:?} has ToUnicode (raw)", id);
                    if let Ok(o) = dict.get(b"ToUnicode") {
                        match o {
                            Object::Reference(oid) => {
                                if let Ok(tu_obj) = doc.get_object(*oid) {
                                    dump_stream(&tu_obj);
                                } else {
                                    println!(
                                        "  failed to get referenced ToUnicode object: {:?}",
                                        oid
                                    );
                                }
                            }
                            Object::Stream(s) => {
                                dump_stream(&Object::Stream(s.clone()));
                            }
                            other => println!("  ToUnicode is {:?}", other),
                        }
                    }
                }
                // Font-like dictionaries
                if dict.get(b"BaseFont").is_ok() || dict.get(b"FontDescriptor").is_ok() {
                    println!("Font dict object {:?}:", id);
                    for (k, v) in dict.iter() {
                        println!("  {:?} => {:?}", k, v);
                    }
                }
            }
            Object::Stream(stream) => {
                if let Ok(bytes) = stream.decompressed_content() {
                    let s = String::from_utf8_lossy(&bytes);
                    if s.contains("beginbfchar")
                        || s.contains("beginbfrange")
                        || s.contains("CIDInit")
                    {
                        println!("Found possible CMap stream at object {:?}", id);
                        println!(
                            "--- stream content start ---\n{}\n--- stream content end ---",
                            s
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Print page font resource info
    for (pnum, &page_id) in doc.get_pages().iter() {
        println!("Page {} => {:?}", pnum, page_id);
        if let Ok(page) = doc.get_object(page_id) {
            if let Object::Dictionary(page_dict) = page {
                // Print page content stream preview
                if let Ok(contents) = page_dict.get(b"Contents") {
                    match contents {
                        Object::Reference(c_oid) => {
                            if let Ok(cobj) = doc.get_object(*c_oid) {
                                println!("Page {} content (ref {:?}):", pnum, c_oid);
                                dump_stream(&cobj);
                            }
                        }
                        Object::Stream(s) => {
                            println!("Page {} content (stream):", pnum);
                            dump_stream(&Object::Stream(s.clone()));
                        }
                        Object::Array(arr) => {
                            println!("Page {} content (array):", pnum);
                            for item in arr.iter() {
                                if let Object::Reference(c_oid) = item {
                                    if let Ok(cobj) = doc.get_object(*c_oid) {
                                        println!("  content ref {:?}:", c_oid);
                                        dump_stream(&cobj);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if let Ok(resources) = page_dict.get(b"Resources") {
                    match resources {
                        Object::Reference(oid) => {
                            if let Ok(resolved) = doc.get_object(*oid) {
                                if let Object::Dictionary(res_dict) = resolved {
                                    if let Ok(fonts) = res_dict.get(b"Font") {
                                        match fonts {
                                            Object::Reference(f_oid) => {
                                                if let Ok(fonts_obj) = doc.get_object(*f_oid) {
                                                    if let Object::Dictionary(fonts_dict) =
                                                        fonts_obj
                                                    {
                                                        for (name, fref) in fonts_dict.iter() {
                                                            println!(
                                                                "  Font {:?} => {:?}",
                                                                name, fref
                                                            );
                                                            match fref {
                                                                Object::Reference(fr_oid) => {
                                                                    if let Ok(fobj) =
                                                                        doc.get_object(*fr_oid)
                                                                    {
                                                                        println!(
                                                                            "    font dict: {:?}",
                                                                            fobj
                                                                        );
                                                                        if let Object::Dictionary(
                                                                            fd,
                                                                        ) = fobj
                                                                        {
                                                                            if let Ok(tu) =
                                                                                fd.get(b"ToUnicode")
                                                                            {
                                                                                println!("    Has ToUnicode reference: {:?}", tu);
                                                                                match tu {
                                                                                    Object::Reference(tu_oid) => {
                                                                                        if let Ok(tu_obj) = doc.get_object(*tu_oid) {
                                                                                            dump_stream(&tu_obj);
                                                                                        }
                                                                                    }
                                                                                    Object::Stream(s) => dump_stream(&Object::Stream(s.clone())),
                                                                                    _ => {}
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Object::Stream(s) => {
                                                                    println!("    font dict stream: {:?}", s);
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Object::Dictionary(fonts_dict) => {
                                                for (name, fref) in fonts_dict.iter() {
                                                    println!("  Font {:?} => {:?}", name, fref);
                                                    match fref {
                                                        Object::Reference(fr_oid) => {
                                                            if let Ok(fobj) =
                                                                doc.get_object(*fr_oid)
                                                            {
                                                                println!(
                                                                    "    font dict: {:?}",
                                                                    fobj
                                                                );
                                                                if let Object::Dictionary(fd) = fobj
                                                                {
                                                                    if let Ok(tu) =
                                                                        fd.get(b"ToUnicode")
                                                                    {
                                                                        println!("    Has ToUnicode reference: {:?}", tu);
                                                                        match tu {
                                                                            Object::Reference(
                                                                                tu_oid,
                                                                            ) => {
                                                                                if let Ok(tu_obj) =
                                                                                    doc.get_object(
                                                                                        *tu_oid,
                                                                                    )
                                                                                {
                                                                                    dump_stream(
                                                                                        &tu_obj,
                                                                                    );
                                                                                }
                                                                            }
                                                                            Object::Stream(s) => {
                                                                                dump_stream(
                                                                                    &Object::Stream(
                                                                                        s.clone(),
                                                                                    ),
                                                                                )
                                                                            }
                                                                            _ => {}
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Object::Stream(s) => {
                                                            println!(
                                                                "    font dict stream: {:?}",
                                                                s
                                                            );
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Object::Dictionary(res_dict) => {
                            if let Ok(fonts) = res_dict.get(b"Font") {
                                match fonts {
                                    Object::Reference(f_oid) => {
                                        if let Ok(fonts_obj) = doc.get_object(*f_oid) {
                                            if let Object::Dictionary(fonts_dict) = fonts_obj {
                                                for (name, fref) in fonts_dict.iter() {
                                                    println!("  Font {:?} => {:?}", name, fref);
                                                    match fref {
                                                        Object::Reference(fr_oid) => {
                                                            if let Ok(fobj) =
                                                                doc.get_object(*fr_oid)
                                                            {
                                                                println!(
                                                                    "    font dict: {:?}",
                                                                    fobj
                                                                );
                                                                if let Object::Dictionary(fd) = fobj
                                                                {
                                                                    if let Ok(tu) =
                                                                        fd.get(b"ToUnicode")
                                                                    {
                                                                        println!("    Has ToUnicode reference: {:?}", tu);
                                                                        match tu {
                                                                            Object::Reference(
                                                                                tu_oid,
                                                                            ) => {
                                                                                if let Ok(tu_obj) =
                                                                                    doc.get_object(
                                                                                        *tu_oid,
                                                                                    )
                                                                                {
                                                                                    dump_stream(
                                                                                        &tu_obj,
                                                                                    );
                                                                                }
                                                                            }
                                                                            Object::Stream(s) => {
                                                                                dump_stream(
                                                                                    &Object::Stream(
                                                                                        s.clone(),
                                                                                    ),
                                                                                )
                                                                            }
                                                                            _ => {}
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Object::Stream(s) => {
                                                            println!(
                                                                "    font dict stream: {:?}",
                                                                s
                                                            );
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Object::Dictionary(fonts_dict) => {
                                        for (name, fref) in fonts_dict.iter() {
                                            println!("  Font {:?} => {:?}", name, fref);
                                            match fref {
                                                Object::Reference(fr_oid) => {
                                                    if let Ok(fobj) = doc.get_object(*fr_oid) {
                                                        println!("    font dict: {:?}", fobj);
                                                        if let Object::Dictionary(fd) = fobj {
                                                            if let Ok(tu) = fd.get(b"ToUnicode") {
                                                                println!("    Has ToUnicode reference: {:?}", tu);
                                                                match tu {
                                                                    Object::Reference(tu_oid) => {
                                                                        if let Ok(tu_obj) =
                                                                            doc.get_object(*tu_oid)
                                                                        {
                                                                            dump_stream(&tu_obj);
                                                                        }
                                                                    }
                                                                    Object::Stream(s) => {
                                                                        dump_stream(
                                                                            &Object::Stream(
                                                                                s.clone(),
                                                                            ),
                                                                        )
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Object::Stream(s) => {
                                                    println!("    font dict stream: {:?}", s);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
