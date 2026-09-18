# tools/corpus — corpus coverage tools

Turns a text corpus into two measurements of how well the normalizer covers
real input:

- `verbalize survey <path>` — every digit/symbol **shape** seen, with its
  spoken form and whether it fell back to a last-resort reading.
- `verbalize audit --lang <lang> <path>` — every unit the resolver **guessed**
  as an SI prefix on a shorter symbol (`Grad` = `G` + `rad`) and that the
  corpus *also* uses as an ordinary word. Each row is a candidate misreading;
  an empty report over a large corpus is the evidence that the guess surface
  does not spell words.

Corpora are never vendored. Fetch or download into a gitignored directory and
point the tools at it.

## Quick start

```sh
python3 tools/corpus/fetch.py --lang de --articles 2000      # sample
cargo run -p verbalize-cli -- survey tools/corpus/.cache/de
cargo run -p verbalize-cli -- audit --lang de tools/corpus/.cache/de
```

`fetch.py` samples Wikipedia article intros (MediaWiki API, CC BY-SA 4.0) or
the Tatoeba sentence export (CC BY 2.0 FR); it does not mirror. For a large
run, download a full dump from the table below and point the tools at the
extracted directory.

## Sources

All per-language and free to use; prefer ones dense in numerals, units,
dates and currency — that is where the classes live.

| Source | What | License | Scale | Where |
|---|---|---|---|---|
| Wikipedia dumps | full article text | CC BY-SA 4.0 | tens of GB compressed per language | `https://dumps.wikimedia.org/{de,en,ro}wiki/latest/` |
| Wikipedia API | article intros | CC BY-SA 4.0 | sampled, no dump needed | `fetch.py --source wiki` |
| Tatoeba | sentences | CC BY 2.0 FR | `deu` 12 MB, `ron` 0.6 MB | `fetch.py --source tatoeba` |
| OPUS | parallel corpora, incl. OpenSubtitles (spoken style) | per-corpus, mostly permissive | GBs | `https://opus.nlpl.eu` |
| Leipzig Corpora (Wortschatz) | news/web/wiki sentences | CC BY | 1M–10M sentences per language | `https://wortschatz.uni-leipzig.de` |
| OSCAR / FineWeb-2 / CC-100 / mC4 | web crawl | CC BY / ODC-By | TBs | HuggingFace `datasets` |
| EU DGT TM, JRC-Acquis, ECDC | legal, medical, all three languages incl. Romanian | EUPL / CC BY | MBs–GBs | `https://opus.nlpl.eu`, JRC |
| PubMed Central OA | biomedical (lab values, doses) | per-article, mostly CC BY | GBs | `https://ftp.ncbi.nlm.nih.gov/pub/pmc/` |
| Project Gutenberg | books | public domain | MBs–GBs | `https://www.gutenberg.org` |

For the `audit` report specifically, Wikipedia, news, legal and medical text
are the useful ones: they put `29 Grad`, `5 mg`, `100 Mbit/s` in running
prose next to the same words used normally.

## Reading a dump

A `pages-articles.xml.bz2` dump is wikitext, not plain text. Extract it
(e.g. `wikiextractor`, `mwxml`) to one text file per article in a directory
and pass that directory; `survey`/`audit` walk a directory tree and skip
non-UTF-8 files. `sqlite:<path>` is also accepted — every `TEXT` column of
every table is scanned.
