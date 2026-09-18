# Corpus attribution

The corpora fetched by `fetch.py` are **not** part of this repository: they
are written to the gitignored `.cache/` and are never redistributed. Each
fetched file carries a `<file>.source` sidecar recording its URL, licence and
attribution, and an OPUS archive's own `LICENSE`/`README` are extracted next
to the text. Any report published from a corpus must carry that attribution.

| Source | Licence | Attribution | Where |
|---|---|---|---|
| Wikipedia (API or dumps) | CC BY-SA 4.0 | Wikipedia contributors | <https://dumps.wikimedia.org> |
| Tatoeba | CC BY 2.0 FR | Tatoeba contributors | <https://tatoeba.org> |
| OPUS corpora | per corpus — see the extracted `LICENSE` | OPUS and the corpus authors | <https://opus.nlpl.eu> |
| PubMed (E-utilities) | NLM Terms and Conditions; abstracts © their authors/publishers | U.S. National Library of Medicine / NCBI | <https://www.ncbi.nlm.nih.gov> |

## OPUS

OPUS aggregates many corpora, each under its own terms; `fetch.py` extracts
the archive's `LICENSE` next to the text so the terms travel with the data.
Cite:

> Jörg Tiedemann. 2012. *Parallel Data, Tools and Interfaces in OPUS.* LREC 2012.

Corpora used by the coverage runs, with their terms (check the extracted
`LICENSE` for the authoritative text):

- **ECDC** — medical, from the European Centre for Disease Prevention and
  Control; CC BY 4.0.
- **DGT** — EU Directorate-General for Translation translation memory;
  EUPL / European Commission reuse terms.
- **JRC-Acquis** — EU law; European Commission reuse terms.
- **OpenSubtitles** — subtitles; OpenSubtitles terms.

## PubMed

Abstracts come from NCBI E-utilities under the NLM Terms and Conditions;
copyright remains with the authors or publishers. Do not redistribute the
fetched text.
