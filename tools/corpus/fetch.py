#!/usr/bin/env python3
"""Fetch a bounded text sample for the corpus coverage tools.

Sources (per-language, free, no account):

  wiki     Wikipedia article intros via the MediaWiki API — CC BY-SA 4.0.
  tatoeba  Tatoeba sentence export (per-language dump) — CC BY 2.0 FR.

One UTF-8 file per source is written into `--out` (default
`tools/corpus/.cache/<lang>/`, gitignored). Point the coverage tools at that
directory:

  python3 tools/corpus/fetch.py --lang de --articles 2000
  cargo run -p verbalize-cli -- survey de tools/corpus/.cache/de
  cargo run -p verbalize-cli -- audit --lang de tools/corpus/.cache/de

This samples; it does not mirror. For a huge corpus, download one of the
full dumps listed in README.md and point the tools at the extracted tree.
"""
from __future__ import annotations

import argparse
import bz2
import json
import pathlib
import sys
import time
import urllib.parse
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
API = "https://{lang}.wikipedia.org/w/api.php"
TATOEBA = "https://downloads.tatoeba.org/exports/per_language/{code}/{code}_sentences.tsv.bz2"
# Tatoeba uses ISO 639-3 codes; the library uses ISO 639-1.
CODE = {"de": "deu", "en": "eng", "ro": "ron"}
UA = "verbalize-corpus/0.1 (+https://github.com/relu/verbalize)"


def get(url: str):
    request = urllib.request.Request(url, headers={"User-Agent": UA})
    return urllib.request.urlopen(request, timeout=120)


def wiki(lang: str, count: int, out: pathlib.Path) -> int:
    """Article intros, in batches of 20 random pages until `count`."""
    texts: list[str] = []
    seen: set[int] = set()
    attempts = 0
    while len(texts) < count and attempts < count // 20 + 50:
        attempts += 1
        params = {
            "action": "query",
            "generator": "random",
            "grnnamespace": "0",
            "grnlimit": "20",
            "prop": "extracts",
            "explaintext": "1",
            "exintro": "1",
            "exlimit": "20",
            "format": "json",
            "formatversion": "2",
        }
        url = API.format(lang=lang) + "?" + urllib.parse.urlencode(params)
        with get(url) as response:
            data = json.load(response)
        for page in data.get("query", {}).get("pages", []):
            extract = page.get("extract")
            if extract and page["pageid"] not in seen:
                seen.add(page["pageid"])
                texts.append(extract.strip())
        time.sleep(0.2)  # be polite to the API
    (out / "wiki.txt").write_text("\n\n".join(texts) + "\n", encoding="utf-8")
    return len(texts)


def tatoeba(lang: str, count: int, out: pathlib.Path) -> int:
    """The first `count` rows of the per-language sentence export."""
    code = CODE[lang]
    with get(TATOEBA.format(code=code)) as response:
        rows = bz2.decompress(response.read()).decode("utf-8").splitlines()
    sentences = [row.split("\t", 2)[2] for row in rows[:count] if row.count("\t") >= 2]
    (out / "tatoeba.txt").write_text("\n".join(sentences) + "\n", encoding="utf-8")
    return len(sentences)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lang", default="de", choices=sorted(CODE))
    parser.add_argument("--source", default="wiki", choices=["wiki", "tatoeba"])
    parser.add_argument("--articles", type=int, default=500, help="pages (wiki) or sentences (tatoeba)")
    parser.add_argument("--out", type=pathlib.Path, default=None)
    args = parser.parse_args()

    out = args.out or HERE / ".cache" / args.lang
    out.mkdir(parents=True, exist_ok=True)
    fetch = wiki if args.source == "wiki" else tatoeba
    written = fetch(args.lang, args.articles, out)
    print(f"{args.source}: {written} texts -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
