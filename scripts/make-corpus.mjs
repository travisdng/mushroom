// Generate a reproducible benchmark corpus: 5,000 notes across 50 folders.
//
// Reproducible on purpose — a measurement taken against a different corpus is
// not comparable with the last one, and "it got faster" would be unfalsifiable.
// Seeded PRNG, fixed vocabulary, fixed size distribution.
//
//   node make-corpus.mjs <target-folder>

import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";

const target = process.argv[2];
if (!target) {
  console.error("usage: node make-corpus.mjs <target-folder>");
  process.exit(1);
}

const NOTES = 5000;
const FOLDERS = 50;

/** Deterministic PRNG (mulberry32), so every run produces the same corpus. */
function rng(seed) {
  return function () {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const random = rng(20260917);

// Terms the benchmark queries search for. Rare on purpose: in a real notebook
// a meaningful search term appears in a small fraction of notes, and how many
// rows match is the single thing search latency depends on.
const TOPICS = [
  "capacity", "latency", "dialler", "transcript", "queue", "agent",
  "retention", "campaign", "routing", "escalation", "handover", "wrap-up",
  "GPU", "node pool", "drain window", "orchestrator", "batch", "index",
  "compliance", "recording", "scorecard", "calibration", "outbound",
];

/**
 * The rest of the vocabulary.
 *
 * The first version of this generator had 33 words total, so every note
 * contained every term and every search matched all 5,000 notes — which
 * measured the worst case imaginable and called it typical. Real prose is
 * Zipfian: a handful of words everywhere, a long tail almost nowhere. That
 * distribution is what decides how many rows FTS5 has to rank, so it is the
 * part of the corpus that actually has to be right.
 */
function buildVocabulary(size) {
  const SYLLABLES = [
    "ka", "re", "to", "mi", "sa", "ne", "lo", "ti", "vu", "da", "shi", "pen",
    "gar", "mel", "sor", "tan", "bri", "cul", "dep", "fen", "hol", "jun",
  ];
  const words = [];
  for (let i = 0; i < size; i += 1) {
    const parts = 2 + Math.floor(random() * 2);
    let word = "";
    for (let p = 0; p < parts; p += 1) {
      word += SYLLABLES[Math.floor(random() * SYLLABLES.length)];
    }
    // Collisions would quietly make the tail less rare than intended.
    words.push(words.includes(word) ? `${word}${i}` : word);
  }
  return words;
}

const VOCABULARY = buildVocabulary(4000);

/** Zipf: index 0 is everywhere, the tail is almost nowhere. */
function zipfIndex(size) {
  // 1/x over [1, size] — cheap to sample by inverse transform.
  return Math.min(size - 1, Math.floor(Math.exp(random() * Math.log(size))) - 1);
}

/** How many notes each word landed in, so the benchmark can name a worst case. */
const notesContaining = new Map();

function words(count, seenInThisNote) {
  const out = [];
  for (let i = 0; i < count; i += 1) {
    const word = VOCABULARY[Math.max(0, zipfIndex(VOCABULARY.length))];
    if (!seenInThisNote.has(word)) {
      seenInThisNote.add(word);
      notesContaining.set(word, (notesContaining.get(word) ?? 0) + 1);
    }
    out.push(word);
  }
  return out.join(" ");
}

/**
 * Realistic size spread: most notes are short, a few are long.
 * A corpus of 5,000 identical notes would measure the wrong thing.
 */
function bodyLength() {
  const roll = random();
  if (roll < 0.7) return 40 + Math.floor(random() * 120);   // a paragraph
  if (roll < 0.95) return 200 + Math.floor(random() * 400); // a page
  return 800 + Math.floor(random() * 1500);                 // a long one
}

rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });

const started = Date.now();
let bytes = 0;
/** How many notes each topic term ended up in, printed so it can be checked. */
const topicCounts = new Map(TOPICS.map((t) => [t, 0]));

for (let i = 0; i < NOTES; i += 1) {
  const folder = `folder-${String(i % FOLDERS).padStart(2, "0")}`;
  const dir = join(target, folder);
  if (i < FOLDERS) mkdirSync(dir, { recursive: true });

  const title = `${TOPICS[i % TOPICS.length]} note ${i}`;
  const sections = 1 + Math.floor(random() * 4);

  let body = `---\ntitle: ${title}\n---\n\n# ${title}\n\n`;
  topicCounts.set(TOPICS[i % TOPICS.length], topicCounts.get(TOPICS[i % TOPICS.length]) + 1);

  const seenInThisNote = new Set();
  for (let s = 0; s < sections; s += 1) {
    let text = words(bodyLength() / sections, seenInThisNote);

    // Sprinkle topic terms into a minority of notes, so a search for one
    // matches roughly 2-4% of the corpus rather than all of it or none.
    if (random() < 0.25) {
      const topic = TOPICS[Math.floor(random() * TOPICS.length)];
      const at = Math.floor(random() * text.length);
      text = `${text.slice(0, at)} ${topic} ${text.slice(at)}`;
      topicCounts.set(topic, topicCounts.get(topic) + 1);
    }

    body += `## Section ${s + 1}\n\n${text}\n\n`;
  }

  const path = join(dir, `note-${String(i).padStart(4, "0")}.md`);
  writeFileSync(path, body);
  bytes += body.length;
}

console.log(
  `${NOTES} notes across ${FOLDERS} folders, ${(bytes / 1024 / 1024).toFixed(1)} MB, ` +
    `in ${((Date.now() - started) / 1000).toFixed(1)}s`,
);

const counts = [...topicCounts.entries()].sort((a, b) => b[1] - a[1]);
console.log(
  `topic term frequency: ${counts[0][0]} in ${counts[0][1]} notes ` +
    `(${((counts[0][1] / NOTES) * 100).toFixed(1)}%), ` +
    `${counts[counts.length - 1][0]} in ${counts[counts.length - 1][1]} ` +
    `(${((counts[counts.length - 1][1] / NOTES) * 100).toFixed(1)}%)`,
);
console.log(`vocabulary: ${VOCABULARY.length} words, Zipf-distributed`);

// The benchmark needs a word that is in almost every note — the worst case
// FTS5 can be handed, where ranking has to score the entire corpus.
const commonest = [...notesContaining.entries()]
  .sort((a, b) => b[1] - a[1])
  .slice(0, 5);
console.log(
  "commonest words: " +
    commonest
      .map(([w, n]) => `${w} (${((n / NOTES) * 100).toFixed(0)}% of notes)`)
      .join(", "),
);
console.log(target);
