/**
 * Pixel icons, drawn as data rather than files.
 *
 * Each icon is a 16x16 grid of characters mapped to a small fixed palette and
 * rendered as 1x1 rects with crisp edges, so it stays sharp at any scale and
 * can be edited by hand right here. No icon font, no CDN, no SVG assets.
 *
 * The application icon set (.ico, PNGs) is produced separately in task 18;
 * this is what the UI draws with.
 */

const PALETTE: Record<string, string> = {
  o: "#000000", // outline
  r: "#9c1f1f", // mushroom cap
  R: "#c73b3b", // mushroom cap highlight
  c: "#f2e3c2", // mushroom stem / spots
  C: "#c9b189", // stem shadow
  w: "#ffffff", // paper / glass
  y: "#e8c060", // folder
  Y: "#c09030", // folder shadow
  b: "#4060b0", // floppy shell
};

const ICONS = {
  mushroom: [
    "................",
    ".....oooooo.....",
    "...ooRRRRRRoo...",
    "..oRRRrrccrRRo..",
    ".oRRrrrrccrrrRo.",
    ".oRrrccrrrrrrro.",
    "oRrrrccrrrccrrRo",
    "orrrrrrrrrccrrro",
    ".oooooooooooooo.",
    "....occccccco...",
    "....ocCccccCo...",
    "....ocCccccCo...",
    "....ocCcccCCo...",
    "....occccccco...",
    ".....ooooooo....",
    "................",
  ],
  new: [
    "................",
    "....oooooo......",
    "....owwwwoo.....",
    "....owwwwwwo....",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....owwwwwwwo...",
    "....ooooooooo...",
    "................",
    "................",
    "................",
  ],
  open: [
    "................",
    "................",
    "..oooo..........",
    ".oYYYYo.........",
    ".oYyyyyoooooo...",
    ".oYyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".oyyyyyyyyyyyo..",
    ".ooooooooooooo..",
    "................",
    "................",
    "................",
  ],
  save: [
    "................",
    "..oooooooooooo..",
    "..obbbbbbbbbbo..",
    "..obwwwwwwwwbo..",
    "..obwwwwwwwwbo..",
    "..obwwwwwwwwbo..",
    "..obbbbbbbbbbo..",
    "..obbbbbbbbbbo..",
    "..obwwwwwwwwbo..",
    "..obwoooooowbo..",
    "..obwoooooowbo..",
    "..obwoooooowbo..",
    "..obwwwwwwwwbo..",
    "..oooooooooooo..",
    "................",
    "................",
  ],
  search: [
    "................",
    "....oooo........",
    "...owwwwo.......",
    "..owwwwwwo......",
    "..owwwwwwo......",
    "..owwwwwwo......",
    "...owwwwo.......",
    "....oooooo......",
    "........oo......",
    ".........oo.....",
    "..........oo....",
    "...........oo...",
    "................",
    "................",
    "................",
    "................",
  ],
  ai: [
    "................",
    "................",
    "................",
    ".......o........",
    "......ooo.......",
    "......ooo.......",
    "..ooooooooooo...",
    "..ooooooooooo...",
    "......ooo.......",
    "......ooo.......",
    ".......o........",
    "................",
    "................",
    "................",
    "................",
    "................",
  ],
} as const;

export type IconName = keyof typeof ICONS;

type IconProps = {
  name: IconName;
  /** Rendered size in px. The grid is 16x16, so 16/32/48 stay pixel-exact. */
  size?: number;
  title?: string;
};

export function Icon({ name, size = 16, title }: IconProps) {
  const grid: readonly string[] = ICONS[name];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      shapeRendering="crispEdges"
      role={title ? "img" : "presentation"}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      style={{ display: "block", flex: "none" }}
    >
      {title ? <title>{title}</title> : null}
      {grid.flatMap((row, y) =>
        row.split("").map((ch, x) => {
          const fill = PALETTE[ch];
          if (!fill) return null;
          return (
            <rect key={`${x},${y}`} x={x} y={y} width={1} height={1} fill={fill} />
          );
        }),
      )}
    </svg>
  );
}
