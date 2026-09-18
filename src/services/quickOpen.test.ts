import { describe, expect, it } from "vitest";

import { fuzzyMatch, rankNotes } from "./quickOpen";
import type { NoteMeta } from "../types/notes";

function note(id: string, title: string, folder = ""): NoteMeta {
  return {
    id,
    title,
    folder,
    created: null,
    modified: 0,
    sizeBytes: 0,
    tags: [],
    aiExcluded: false,
  };
}

const CORPUS: NoteMeta[] = [
  note("work/gpu-infrastructure.md", "GPU Infrastructure", "work"),
  note("work/quarterly-capacity-review.md", "Quarterly Capacity Review", "work"),
  note("work/realtime-guidance.md", "Real-Time Guidance", "work"),
  note("ideas/rag-ideas.md", "RAG ideas", "ideas"),
  note("gpu-issue.md", "GPU issue", ""),
  note("personal/grouping-public-updates.md", "Grouping Public Updates", "personal"),
];

const titles = (query: string, recent: string[] = []) =>
  rankNotes(CORPUS, query, recent).map((hit) => hit.note.title);

describe("fuzzyMatch", () => {
  it("matches a subsequence and reports where", () => {
    const hit = fuzzyMatch("GPU Infrastructure", "gpu");
    expect(hit).not.toBeNull();
    expect(hit!.indices).toEqual([0, 1, 2]);
  });

  it("returns null when a character is missing", () => {
    expect(fuzzyMatch("GPU Infrastructure", "gpuz")).toBeNull();
  });

  it("is case-insensitive in both directions", () => {
    expect(fuzzyMatch("GPU Infrastructure", "GPU")).not.toBeNull();
    expect(fuzzyMatch("gpu infrastructure", "GPU")).not.toBeNull();
  });

  it("scores a contiguous run above scattered letters", () => {
    // Both contain g, p, u in order; only one contains "gpu".
    const contiguous = fuzzyMatch("GPU Infrastructure", "gpu")!;
    const scattered = fuzzyMatch("Grouping Public Updates", "gpu")!;
    expect(contiguous.score).toBeGreaterThan(scattered.score);
  });

  it("scores a word start above the middle of a word", () => {
    const atStart = fuzzyMatch("Capacity Review", "rev")!;
    const inMiddle = fuzzyMatch("Unreviewed Draft", "rev")!;
    expect(atStart.score).toBeGreaterThan(inMiddle.score);
  });

  it("an empty query matches everything with no score", () => {
    const hit = fuzzyMatch("anything", "");
    expect(hit).toEqual({ score: 0, indices: [] });
  });
});

describe("rankNotes", () => {
  it("puts the obvious match first", () => {
    expect(titles("gpu")[0]).toBe("GPU issue");
    expect(titles("gpu")).toContain("GPU Infrastructure");
  });

  it("ranks a real word match above an accidental subsequence", () => {
    const ranked = titles("gpu");
    expect(ranked.indexOf("GPU Infrastructure")).toBeLessThan(
      ranked.indexOf("Grouping Public Updates"),
    );
  });

  it("finds a note by its path when the title does not match", () => {
    // "realtime" is in the filename; the title is "Real-Time Guidance".
    expect(titles("realtime")).toContain("Real-Time Guidance");
  });

  it("prefers a title match over a path match", () => {
    const ranked = rankNotes(CORPUS, "guidance", []);
    expect(ranked[0]!.note.title).toBe("Real-Time Guidance");
    expect(ranked[0]!.titleMatches.length).toBeGreaterThan(0);
  });

  it("excludes notes that do not match at all", () => {
    expect(titles("zzzz")).toEqual([]);
  });

  it("boosts a recently opened note", () => {
    const without = titles("gpu");
    const withRecent = titles("gpu", ["work/gpu-infrastructure.md"]);
    expect(withRecent[0]).toBe("GPU Infrastructure");
    expect(without[0]).not.toBe("GPU Infrastructure");
  });

  it("decays the recency boost down the list", () => {
    // First in the MRU beats second, all else equal.
    const recent = ["work/gpu-infrastructure.md", "gpu-issue.md"];
    const ranked = rankNotes(CORPUS, "gpu", recent);
    expect(ranked[0]!.note.id).toBe("work/gpu-infrastructure.md");
  });

  it("an empty query lists recently opened notes, newest first", () => {
    const recent = ["ideas/rag-ideas.md", "gpu-issue.md"];
    expect(titles("", recent)).toEqual(["RAG ideas", "GPU issue"]);
  });

  it("an empty query with no history shows nothing rather than everything", () => {
    // 5,000 notes in arbitrary order is not a useful first screen.
    expect(titles("")).toEqual([]);
  });

  it("ignores surrounding whitespace", () => {
    expect(titles("  gpu  ")).toEqual(titles("gpu"));
  });

  it("respects the limit", () => {
    const many = Array.from({ length: 200 }, (_, i) =>
      note(`n${i}.md`, `Note ${i}`),
    );
    expect(rankNotes(many, "note", [], 10)).toHaveLength(10);
  });

  it("is fast enough on a large corpus", () => {
    // The 50 ms budget is the whole reason matching is not an IPC call.
    const many = Array.from({ length: 5000 }, (_, i) =>
      note(`folder${i % 50}/note-${i}.md`, `Note ${i} about capacity`, `folder${i % 50}`),
    );
    const started = performance.now();
    rankNotes(many, "capacity", []);
    expect(performance.now() - started).toBeLessThan(50);
  });
});

describe("ranking 5,000 notes", () => {
  // R5: Quick Open filters in the window, on every keystroke, over the whole
  // note list. The threshold is loose on purpose — this is here to catch a
  // change that makes ranking quadratic, not to fail on a busy CI box.
  const many: NoteMeta[] = Array.from({ length: 5000 }, (_, i) =>
    note(
      `folder-${String(i % 50).padStart(2, "0")}/note-${i}.md`,
      `${["capacity", "latency", "dialler", "transcript", "queue"][i % 5]} note ${i}`,
      `folder-${String(i % 50).padStart(2, "0")}`,
    ),
  );

  it("filters well inside a keystroke", () => {
    // The interesting queries are the ones that match a lot: a single letter
    // matches nearly everything and still has to score and sort it.
    const queries = ["c", "ca", "cap", "capa", "note 49", "qln", "zzzz"];
    const timings: number[] = [];

    for (const query of queries) {
      const started = performance.now();
      rankNotes(many, query, []);
      timings.push(performance.now() - started);
    }

    const worst = Math.max(...timings);
    const report = queries
      .map((q, i) => `${q}=${timings[i]!.toFixed(1)}ms`)
      .join(" ");
    // eslint-disable-next-line no-console
    console.log(`quick open over ${many.length} notes: ${report}`);

    expect(worst).toBeLessThan(200);
  });
});
