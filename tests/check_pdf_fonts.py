#!/usr/bin/env python3
"""
Vérifie dans un PDF :
1) quelle police rend la chaîne "Library entry point"
2) si les caractères box-drawing (├, └, │, ─) sont présents et dans quelle police

Usage: python3 tests/check_pdf_fonts.py [path/to/pdf]
"""
import sys
from pathlib import Path

BOX_CHARS = ["├", "└", "│", "─"]
TARGET = "Library entry point"  # recherche principale (on gère les correspondances sur plusieurs spans)

try:
    import fitz  # PyMuPDF
except Exception as e:
    print("ERREUR: PyMuPDF (fitz) non installé. Installez-le avec:\n    python3 -m pip install --user PyMuPDF")
    raise


def find_pdf_path(argv):
    if len(argv) > 1:
        p = Path(argv[1])
        if p.exists():
            return p
        print(f"Fichier fourni introuvable: {p}")
        sys.exit(2)

    candidates = [Path("/tmp/dir_structure_test.pdf"), Path("dir_structure_test.pdf"), Path("/Users/rlemeill/Development/markdown2pdf/dir_structure_test.pdf")]
    for c in candidates:
        if c.exists():
            return c
    print("Aucun PDF trouvé par défaut. Passez le chemin du PDF en argument.")
    sys.exit(1)


def analyze_pdf(path: Path):
    doc = fitz.open(str(path))

    fonts_for_target = set()
    occurrences = []
    all_fonts = set()

    char_presence = {ch: {"found": False, "pages": set(), "fonts": set(), "examples": []} for ch in BOX_CHARS}

    for page_index in range(len(doc)):
        page = doc[page_index]
        # get structured text
        try:
            d = page.get_text("dict")
        except Exception:
            # fallback to plain text
            text = page.get_text()
            if TARGET in text:
                fonts_for_target.add("(plain-text-extraction)")
            for ch in BOX_CHARS:
                if ch in text:
                    char_presence[ch]["found"] = True
                    char_presence[ch]["pages"].add(page_index + 1)
            continue

        for block in d.get("blocks", []):
            if block.get("type") != 0:
                continue
            for line in block.get("lines", []):
                # Reconstitute le texte de la ligne en concaténant les spans pour détecter
                # les correspondances qui peuvent être réparties sur plusieurs spans.
                spans = line.get("spans", [])
                line_text = "".join(span.get("text", "") for span in spans)
                # Collecter les polices présentes dans la ligne
                line_fonts = [span.get("font", "") for span in spans if span.get("font")]
                for font in line_fonts:
                    all_fonts.add(font)

                # Si la cible est présente dans la ligne entière, déterminer les polices qui couvrent la zone
                if TARGET in line_text:
                    idx = line_text.find(TARGET)
                    end_idx = idx + len(TARGET)
                    fonts_used = set()
                    pos = 0
                    for span in spans:
                        s = span.get("text", "")
                        s_len = len(s)
                        span_start, span_end = pos, pos + s_len
                        # Si l'intervalle [span_start, span_end) intersecte [idx, end_idx)
                        if not (end_idx <= span_start or idx >= span_end):
                            fonts_used.add(span.get("font", "(unknown)"))
                        pos += s_len
                    fonts_for_target.update(fonts_used)
                    occurrences.append({"page": page_index + 1, "fonts": sorted(fonts_used), "text": line_text})

                # Rechercher les caractères box-drawing dans la ligne entière
                for ch in BOX_CHARS:
                    if ch in line_text:
                        char_presence[ch]["found"] = True
                        char_presence[ch]["pages"].add(page_index + 1)
                        # déterminer quelles polices couvrent ces caractères
                        pos = 0
                        for span in spans:
                            s = span.get("text", "")
                            if ch in s:
                                char_presence[ch]["fonts"].add(span.get("font", "(unknown)"))
                                if len(char_presence[ch]["examples"]) < 3:
                                    char_presence[ch]["examples"].append({"page": page_index + 1, "font": span.get("font", "(unknown)"), "context": s})
                            pos += len(s)

    return fonts_for_target, occurrences, char_presence, all_fonts


if __name__ == "__main__":
    pdf_path = find_pdf_path(sys.argv)
    print(f"Analyse du PDF : {pdf_path}\n")

    fonts_for_target, occurrences, char_presence, all_fonts = analyze_pdf(pdf_path)

    print("Polices retrouvées dans le document:")
    if all_fonts:
        for f in sorted(all_fonts):
            print(" -", f)
    else:
        print(" - Aucune police textuelle détectée via l'extraction de spans")

    if fonts_for_target:
        print(f"\nPolice(s) utilisée(s) pour '{TARGET}':")
        for f in sorted(fonts_for_target):
            print(" -", f)
        print("\nExemples d'occurrences:")
        for o in occurrences:
            fonts = ','.join(o.get('fonts', [])) if o.get('fonts') else o.get('font', '(unknown)')
            print(f"  page {o['page']}, police(s)={fonts}, texte=\"{o['text']}\"")
    else:
        print(f"\nChaîne '{TARGET}' non trouvée dans le PDF.")

    print("\nVérification des caractères box-drawing (via extraction de texte):")
    any_missing = False
    for ch in BOX_CHARS:
        info = char_presence[ch]
        if info["found"]:
            print(f" - '{ch}': TROUVÉ sur les pages {sorted(info['pages'])}, police(s): {sorted(info['fonts'])}")
            for ex in info["examples"]:
                print(f"    exemple (page {ex['page']}, police={ex['font']}): {ex['context']}")
        else:
            any_missing = True
            print(f" - '{ch}': NON TROUVÉ")

    # Recherche brute dans les octets du PDF (UTF-8)
    def search_raw_bytes_for_chars(path: Path):
        data = path.read_bytes()
        results = {}
        for ch in BOX_CHARS:
            results[ch] = data.count(ch.encode('utf-8'))
        return results

    print('\nRecherche brute (octets UTF-8) dans le PDF:')
    raw_results = search_raw_bytes_for_chars(pdf_path)
    for ch, cnt in raw_results.items():
        print(f" - '{ch}': occurrences d'octets UTF-8 dans le fichier PDF = {cnt}")

    if any_missing:
        print("\nConclusion: au moins un caractère box-drawing est absent du texte extrait du PDF.")
        print("Utilisez la recherche brute ci-dessus pour savoir si les octets UTF-8 sont présents dans le PDF.")
        sys.exit(1)
    else:
        print("\nConclusion: tous les caractères box-drawing demandés ont été trouvés dans le PDF (extraction textuelle).")
        sys.exit(0)
