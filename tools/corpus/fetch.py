#!/usr/bin/env python3
"""Fetch bounded text samples for the corpus coverage tools.

Sources (all free, no account):

  wiki     Wikipedia article intros via the MediaWiki API — CC BY-SA 4.0.
  tatoeba  Tatoeba sentence export (per-language dump) — CC BY 2.0 FR.
  opus     An OPUS corpus, source-language side of a moses pair (medical,
           legal, subtitles) — per-corpus licence, extracted next to the text.
  pubmed   PubMed abstracts via NCBI E-utilities (English biomedical:
           doses, units, lab values) — NLM terms; abstracts © their authors.

One UTF-8 file per source is written into `--out` (default
`tools/corpus/.cache/<lang>/`, gitignored), each with a `<file>.source`
sidecar recording the URL, licence and attribution. See ATTRIBUTION.md.

  python3 tools/corpus/fetch.py --lang de --articles 2000
  python3 tools/corpus/fetch.py --lang de --source opus --opus-corpus ECDC
  python3 tools/corpus/fetch.py --lang en --source pubmed --articles 5000

Point the coverage tools at the directory:

  cargo run -p verbalize-cli -- survey --lang de tools/corpus/.cache/de
  cargo run -p verbalize-cli -- audit --lang de tools/corpus/.cache/de

This samples; it does not mirror. For a huge corpus, download one of the
full dumps listed in README.md and point the tools at the extracted tree.
"""
from __future__ import annotations

import argparse
import bz2
import datetime
import io
import json
import pathlib
import sys
import time
import urllib.parse
import urllib.request
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
API = "https://{lang}.wikipedia.org/w/api.php"
TATOEBA = "https://downloads.tatoeba.org/exports/per_language/{code}/{code}_sentences.tsv.bz2"
OPUS_API = "https://opus.nlpl.eu/opusapi"
EUTILS = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/"
# Tatoeba uses ISO 639-3 codes; the library uses ISO 639-1.
CODE = {"de": "deu", "en": "eng", "ro": "ron"}
UA = "verbalize-corpus/0.1 (+https://github.com/relu/verbalize)"


def get(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": UA})
    return urllib.request.urlopen(request, timeout=120).read()


def record(out: pathlib.Path, name: str, url: str, licence: str, attribution: str) -> None:
    """A `<name>.source` sidecar: where the text came from and its terms."""
    (out / f"{name}.source").write_text(
        f"source: {name}\n"
        f"url: {url}\n"
        f"licence: {licence}\n"
        f"attribution: {attribution}\n"
        f"fetched: {datetime.date.today().isoformat()}\n",
        encoding="utf-8",
    )


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
        data = json.loads(get(url))
        for page in data.get("query", {}).get("pages", []):
            extract = page.get("extract")
            if extract and page["pageid"] not in seen:
                seen.add(page["pageid"])
                texts.append(extract.strip())
        time.sleep(0.2)  # be polite to the API
    (out / "wiki.txt").write_text("\n\n".join(texts) + "\n", encoding="utf-8")
    record(
        out,
        "wiki",
        API.format(lang=lang),
        "CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/)",
        f"Wikipedia contributors ({lang}.wikipedia.org)",
    )
    return len(texts)


def tatoeba(lang: str, count: int, out: pathlib.Path) -> int:
    """The first `count` rows of the per-language sentence export."""
    code = CODE[lang]
    url = TATOEBA.format(code=code)
    rows = bz2.decompress(get(url)).decode("utf-8").splitlines()
    sentences = [row.split("\t", 2)[2] for row in rows[:count] if row.count("\t") >= 2]
    (out / "tatoeba.txt").write_text("\n".join(sentences) + "\n", encoding="utf-8")
    record(
        out,
        "tatoeba",
        url,
        "CC BY 2.0 FR (https://creativecommons.org/licenses/by/2.0/fr/)",
        "Tatoeba contributors (https://tatoeba.org)",
    )
    return len(sentences)


def opus(lang: str, count: int, out: pathlib.Path, corpus: str, target: str | None) -> int:
    """The source-language side of one OPUS corpus, moses preprocessing."""
    target = target or ("de" if lang == "en" else "en")
    query = urllib.parse.urlencode(
        {"corpus": corpus, "source": lang, "target": target, "preprocessing": "moses"}
    )
    data = json.loads(get(f"{OPUS_API}?{query}"))
    entries = data.get("corpora") or []
    if not entries:
        raise SystemExit(f"OPUS has no {corpus} {lang}-{target} pair")
    entry = max(entries, key=lambda e: e.get("source_tokens") or 0)
    archive = zipfile.ZipFile(io.BytesIO(get(entry["url"])))
    # The archive may be named `src-tgt` or `tgt-src`; take the side that ends
    # in the requested language.
    members = [n for n in archive.namelist() if n.endswith(f".{lang}")]
    if not members:
        raise SystemExit(f"{corpus}: no .{lang} side in {entry['url']}")
    lines = archive.read(members[0]).decode("utf-8").splitlines()[:count]
    name = f"opus-{corpus.lower()}"
    (out / f"{name}.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    # OPUS archives carry the corpus's own licence and readme; keep them.
    for member_name in ("LICENSE", "README"):
        if member_name in archive.namelist():
            (out / f"{name}.{member_name}").write_bytes(archive.read(member_name))
    record(
        out,
        name,
        entry["url"],
        f"see {name}.LICENSE (extracted from the archive)",
        f"OPUS (https://opus.nlpl.eu), corpus {corpus} {entry.get('version', '')}".strip(),
    )
    return len(lines)


def pubmed(lang: str, count: int, out: pathlib.Path, query: str) -> int:
    """PubMed abstracts via E-utilities (English biomedical text)."""
    if lang != "en":
        raise SystemExit("pubmed is English-only; use --lang en")
    search = urllib.parse.urlencode(
        {"db": "pubmed", "retmax": count, "retmode": "json", "sort": "date", "term": query}
    )
    ids = json.loads(get(f"{EUTILS}esearch.fcgi?{search}"))["esearchresult"]["idlist"]
    blocks: list[str] = []
    for at in range(0, len(ids), 200):
        fetch = urllib.parse.urlencode(
            {"db": "pubmed", "id": ",".join(ids[at : at + 200]), "rettype": "abstract", "retmode": "text"}
        )
        blocks.append(get(f"{EUTILS}efetch.fcgi?{fetch}").decode("utf-8", "replace"))
        time.sleep(0.4)  # E-utilities asks for <= 3 requests/second
    (out / "pubmed.txt").write_text("\n\n".join(blocks), encoding="utf-8")
    record(
        out,
        "pubmed",
        f"{EUTILS}efetch.fcgi?db=pubmed",
        "NLM terms of use (https://www.nlm.nih.gov/databases/download/terms_and_conditions.html); "
        "abstracts remain © their authors/publishers",
        "U.S. National Library of Medicine / NCBI PubMed",
    )
    return len(ids)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lang", default="de", choices=sorted(CODE))
    parser.add_argument("--source", default="wiki", choices=["wiki", "tatoeba", "opus", "pubmed"])
    parser.add_argument("--articles", type=int, default=500, help="pages, sentences or abstracts")
    parser.add_argument("--opus-corpus", default="ECDC", help="ECDC, DGT, JRC-Acquis, OpenSubtitles, …")
    parser.add_argument("--opus-target", default=None, help="pair language; default: en (de when --lang en)")
    parser.add_argument("--pubmed-query", default="mg[Title/Abstract] OR dose[Title/Abstract]")
    parser.add_argument("--out", type=pathlib.Path, default=None)
    args = parser.parse_args()

    out = args.out or HERE / ".cache" / args.lang
    out.mkdir(parents=True, exist_ok=True)
    if args.source == "wiki":
        written = wiki(args.lang, args.articles, out)
    elif args.source == "tatoeba":
        written = tatoeba(args.lang, args.articles, out)
    elif args.source == "opus":
        written = opus(args.lang, args.articles, out, args.opus_corpus, args.opus_target)
    else:
        written = pubmed(args.lang, args.articles, out, args.pubmed_query)
    print(f"{args.source}: {written} texts -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
