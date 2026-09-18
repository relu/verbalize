# tools/corpus — corpus coverage tools

Turns a text corpus into two measurements of how well the normalizer covers
real input:

- `verbalize survey <path>` — every digit/symbol **shape** seen, with its
  spoken form and how many occurrences fell back to a last-resort reading or
  were left verbatim.
- `verbalize audit --lang <lang> <path>` — every unit the resolver **guessed**
  as an SI prefix on a shorter symbol (`Grad` = `G` + `rad`) and that the
  corpus *also* uses as an ordinary word. Each row is a candidate misreading;
  an empty report over a large corpus is the evidence that the guess surface
  does not spell words.

Corpora are never vendored. Fetch or download into a gitignored directory and
point the tools at it. Attribution for every source is in
[ATTRIBUTION.md](ATTRIBUTION.md) and in the `<file>.source` sidecar written
next to each fetched file.

## Quick start

```sh
python3 tools/corpus/fetch.py --lang de --articles 2000                     # general
python3 tools/corpus/fetch.py --lang de --source opus --opus-corpus ECDC    # medical
python3 tools/corpus/fetch.py --lang en --source pubmed --articles 5000     # biomedical
tools/corpus/check.sh                                                      # both measurements
```

`fetch.py` samples — it does not mirror. For a large run, download a full
dump from the table below and point the tools at the extracted directory.

## Sources

| Source | What | Licence | Scale | Where |
|---|---|---|---|---|
| Wikipedia dumps | full article text | CC BY-SA 4.0 | tens of GB compressed per language | `https://dumps.wikimedia.org/{de,en,ro}wiki/latest/` |
| Wikipedia API | article intros | CC BY-SA 4.0 | sampled, no dump needed | `fetch.py --source wiki` |
| Tatoeba | sentences | CC BY 2.0 FR | `deu` 12 MB, `ron` 0.6 MB | `fetch.py --source tatoeba` |
| OPUS | medical/legal/subtitles, per corpus | per corpus (archive `LICENSE`) | KBs–GBs | `fetch.py --source opus --opus-corpus ECDC` |
| PubMed | biomedical abstracts | NLM terms; © authors | sampled | `fetch.py --source pubmed --lang en` |
| OSCAR / FineWeb-2 / CC-100 / mC4 | web crawl | CC BY / ODC-By | TBs | HuggingFace `datasets` |
| Leipzig Corpora (Wortschatz) | news/web/wiki sentences | CC BY | 1M–10M sentences per language | `https://wortschatz.uni-leipzig.de` |
| Project Gutenberg | books | public domain | MBs–GBs | `https://www.gutenberg.org` |

Useful OPUS corpora: **ECDC** (medical, all three languages, small),
**DGT** (EU legal, all three, ~200 MB), **JRC-Acquis** (EU law),
**OpenSubtitles** (spoken style, GBs). For the `audit` report specifically,
the domain text is the point: it puts `29 Grad`, `5 mg`, `100 Mbit/s` in
running prose next to the same words used normally.

## Release report

`check.sh` runs `audit` and prints each language's unhandled survey shapes.
The `audit` list is a **triage**, not a verdict: a genuine unit appears in
prose too (`kV`, `mGy`, `mmol/L`), so a hit needs a human eye — `Palacios MA`
is an author initial, not megaampere. `check.sh --strict` turns any hit into
a failure, for a curated corpus. Corpora are not vendored and CI has no
network, so this is a release-checklist step, not a CI job; see the
"Releasing" section of the top-level README.

## Reading a dump

A `pages-articles.xml.bz2` dump is wikitext, not plain text. Extract it
(e.g. `wikiextractor`, `mwxml`) to one text file per article in a directory
and pass that directory; `survey`/`audit` walk a directory tree and skip
non-UTF-8 files. `sqlite:<path>` is also accepted — every `TEXT` column of
every table is scanned.
