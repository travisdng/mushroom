# Search syntax

`Ctrl+F` searches the full text of every note. Typing plain words is the
normal case and needs nothing from this page; the rest is for when plain words
find too much.

## Plain words

```
drain window
```

Finds notes containing **both** words, anywhere, in any order. Adding a word
narrows the result rather than widening it.

Case does not matter. Neither does punctuation around a word: `latency,` and
`latency` are the same search.

## Quoted phrases

```
"drain window"
```

Finds the two words next to each other, in that order. Use this when the words
are common on their own — `drain window` finds every note mentioning drains
*and* windows separately, which on a large notebook is most of them.

A quote you forget to close runs to the end of what you typed rather than
failing.

## Excluding a word

```
capacity -gpu
```

Finds notes about capacity that do **not** mention GPUs. The `-` must touch
the word.

Exclusions only narrow an existing search. A query of nothing but exclusions
(`-gpu`) would mean "every note you have except these", which is not a search,
so it returns nothing.

## Prefix matching

```
escalat*
```

Matches `escalate`, `escalated`, `escalation`, `escalating`. The `*` goes at
the end only — Mushroom cannot search for words by their ending.

Note that a plain search does **not** do this automatically: `escalat` on its
own matches nothing, because it is not a word in any note.

## Combining them

These all work together:

```
"node pool" drain* -scheduled
```

— notes containing the phrase *node pool*, and some word starting with
*drain*, and not the word *scheduled*.

## What is not supported

- **`OR`** — every search requires all its terms. Run the two searches.
- **Brackets** — no grouping. `(` and `)` are treated as ordinary characters.
- **Field search** — no `title:foo`. Search titles with `Ctrl+P` instead,
  which matches titles and folders only.
- **Regular expressions** and wildcards inside a word (`dr*n`).

Anything Mushroom does not understand is searched for literally rather than
rejected, so a stray bracket or colon gives you results, not an error.

## How results are ranked

Notes are ordered by [BM25](https://en.wikipedia.org/wiki/Okapi_BM25): a note
where your words are rare, frequent, and in a short note ranks above one where
they are common and buried in a long one. Titles are weighted above body text.

Each result shows one passage — the best-matching part of that note, not the
first. One line per note, so the list is notes you could open, not fragments.

## Filters beside the box

The **In:** dropdown limits a search to one folder. The date filter limits it
by when the note was last modified. Both are applied by the index rather than
after the fact, so narrowing makes a search faster, not slower.

## Asking a question instead

The **AI** button is a different thing to the search box. It takes a question
in ordinary English, finds the relevant notes itself, and writes an answer
citing them. It is looser about wording on purpose: a question is expanded and
matched on *any* of its words, because requiring all of them would find
nothing. Search when you know roughly what you wrote; ask when you do not.

## If search finds nothing you expected

The index is built from your files and can lag if something changed outside
Mushroom while it was closed. `F5` rescans. **Tools → Rebuild Index** rebuilds
from scratch — it reads your notes and never writes to them, so it is always
safe. See [troubleshooting.md](troubleshooting.md).
