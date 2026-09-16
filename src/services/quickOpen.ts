import type { NoteMeta } from "../types/notes";

/**
 * Scoring for Quick Open.
 *
 * A pure function over the already-cached note list. No IPC per keystroke —
 * that is the whole reason it can stay under the 50 ms budget on 5,000 notes.
 */

export type QuickOpenHit = {
  note: NoteMeta;
  score: number;
  /** Indices into the title that matched, for highlighting. */
  titleMatches: number[];
};

/** Recency beats a slightly better spelling match: you reopen what you just had. */
const MRU_BONUS = 400;
const MRU_DECAY = 20;

const TITLE_BASE = 1000;
const PATH_BASE = 300;
/** Each additional character in a contiguous run is worth more than the last. */
const CONTIGUITY_BONUS = 12;
const START_OF_STRING_BONUS = 40;
const WORD_START_BONUS = 25;

type Match = { score: number; indices: number[] } | null;

/**
 * Subsequence match, preferring contiguous runs and word starts.
 *
 * `gpu` matches "GPU Infrastructure" better than "Grouping Public Updates",
 * even though both contain g, p and u in order.
 */
export function fuzzyMatch(haystack: string, needle: string): Match {
  if (!needle) return { score: 0, indices: [] };

  const hay = haystack.toLowerCase();
  const pin = needle.toLowerCase();

  const indices: number[] = [];
  let score = 0;
  let run = 0;
  let from = 0;

  for (const char of pin) {
    const at = hay.indexOf(char, from);
    if (at < 0) return null;

    if (at === from && indices.length > 0) {
      // Contiguous with the previous character: each extra character in the
      // run is worth more, so a whole-word match beats scattered letters.
      run += 1;
      score += CONTIGUITY_BONUS * run;
    } else {
      run = 0;
    }

    if (at === 0) score += START_OF_STRING_BONUS;
    else if (isWordBoundary(hay, at)) score += WORD_START_BONUS;

    indices.push(at);
    from = at + 1;
  }

  // A short haystack that matched is a better hit than a long one: "GPU" is a
  // better match for `gpu` than "GPU Infrastructure Capacity Notes".
  score += Math.max(0, 60 - haystack.length);
  return { score, indices };
}

function isWordBoundary(text: string, at: number): boolean {
  if (at === 0) return true;
  const previous = text[at - 1];
  return previous === " " || previous === "-" || previous === "_" || previous === "/";
}

/**
 * Rank notes for a query.
 *
 * Title matches beat path matches: you think of a note by its name. An empty
 * query lists the most recently opened first, which makes `Ctrl+P` `Enter` a
 * quick way back to what you just had.
 */
export function rankNotes(
  notes: NoteMeta[],
  query: string,
  recent: string[],
  limit = 50,
): QuickOpenHit[] {
  const trimmed = query.trim();
  const recencyOf = (id: string) => {
    const at = recent.indexOf(id);
    return at < 0 ? 0 : Math.max(0, MRU_BONUS - at * MRU_DECAY);
  };

  if (!trimmed) {
    return notes
      .filter((note) => recent.includes(note.id))
      .sort((a, b) => recent.indexOf(a.id) - recent.indexOf(b.id))
      .slice(0, limit)
      .map((note) => ({ note, score: recencyOf(note.id), titleMatches: [] }));
  }

  const hits: QuickOpenHit[] = [];

  for (const note of notes) {
    const byTitle = fuzzyMatch(note.title, trimmed);
    if (byTitle) {
      hits.push({
        note,
        score: TITLE_BASE + byTitle.score + recencyOf(note.id),
        titleMatches: byTitle.indices,
      });
      continue;
    }

    // Fall back to the path, so `work/gpu` finds it too.
    const byPath = fuzzyMatch(note.id, trimmed);
    if (byPath) {
      hits.push({
        note,
        score: PATH_BASE + byPath.score + recencyOf(note.id),
        titleMatches: [],
      });
    }
  }

  return hits
    .sort((a, b) => b.score - a.score || a.note.title.localeCompare(b.note.title))
    .slice(0, limit);
}
