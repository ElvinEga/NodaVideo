export const COLOR = {
  text: "#f1f3f9",
  muted: "#8a90a2",
  dim: "#626879",
  subtle: "#a3a9b9",
  soft: "#b4bac9",
  light: "#d2d6e1",
  border: "#2e313a",
  divider: "#262830",
  surface: "#1c1d22",
  panel: "#17181c",
  page: "#131417",
  hover: "#262830",
  sheet: "#f4f4f6",
  ink: "#1b1b1e",
  rule: "#d9d9de",
  well: "#131417",
  accent: "#6366f1",
  accentHover: "#4f46e5",
  accentBackground: "#252544",
  rail: "#454b5b",
  accentSoft: "#a5b4fc",
  grey: "#3a3e4b",
  error: "#ef4444",
  errorText: "#fca5a5",
  errorBackground: "#2a1a1c",
  success: "#10b981",
  warning: "#f59e0b",
  onAccent: "#ffffff",
  monitor: "#0a0b0d",
  clipVideo: "#312e81",
  clipAudio: "#064e3b",
  clipTitle: "#581c87",
  videoBadgeBorder: "#355b78",
  videoBadgeText: "#8fb9dd",
  audioBadgeBorder: "#32725a",
  audioBadgeText: "#7fd0ae",
  imageBadgeBorder: "#665184",
  imageBadgeText: "#c3a9e8",
  srtBadgeBorder: "#8c5a46",
  srtBadgeText: "#f4b99f",
  otherBadgeBorder: "#46464e",
  otherBadgeText: "#8b8b94",
  accentBorder: "#6366f1",
  activeTimeline: "#252544",
  menu: "#1c1d22",
  menuHover: "#262830",
  cursorOutline: "#080809",
  cyberCyan: "#00f0ff",
  cyberYellow: "#fcee0a",
  cyberInk: "#0a0a0b",
  cyberDisabledBorder: "#3a3926",
  cyberDisabledText: "#57563f",
} as const;

export type TrackKind = "T" | "V" | "A";

export const TRACK_KIND_COLOR: Record<
  TrackKind,
  { fill: string; text: string }
> = {
  T: { fill: "#7a5fb0", text: "#e3d4ff" },
  V: { fill: "#3c6a91", text: "#cfe6ff" },
  A: { fill: "#2f7d6d", text: "#c9f2e7" },
};

export const FONT_SANS =
  '-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",Arial,sans-serif';
export const FONT_MONO = '"SF Mono",Menlo,Monaco,Consolas,monospace';
