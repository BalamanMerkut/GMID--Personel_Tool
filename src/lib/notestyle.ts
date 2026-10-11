// Not renkleri ve yazı tipleri (not başına seçilir).

export const NOTE_COLORS: { id: string; label: string; swatch: string; paper: string }[] = [
  { id: "default", label: "Kömür", swatch: "#2f3235", paper: "linear-gradient(160deg, rgba(47,50,53,.9), rgba(34,37,42,.92))" },
  { id: "gold", label: "Altın", swatch: "#b8963f", paper: "linear-gradient(160deg, rgba(110,90,38,.55), rgba(34,37,42,.92))" },
  { id: "teal", label: "Turkuaz", swatch: "#35a0a2", paper: "linear-gradient(160deg, rgba(26,92,96,.6), rgba(30,36,40,.92))" },
  { id: "navy", label: "Lacivert", swatch: "#1f4a86", paper: "linear-gradient(160deg, rgba(11,32,62,.85), rgba(30,34,42,.92))" },
  { id: "plum", label: "Mor", swatch: "#7848a8", paper: "linear-gradient(160deg, rgba(72,40,110,.6), rgba(34,32,42,.92))" },
  { id: "green", label: "Yeşil", swatch: "#4a8a4f", paper: "linear-gradient(160deg, rgba(36,80,44,.6), rgba(30,36,34,.92))" },
  { id: "rose", label: "Gül", swatch: "#b04a5f", paper: "linear-gradient(160deg, rgba(110,40,56,.55), rgba(36,32,36,.92))" },
];

export const NOTE_FONTS: { id: string; label: string; family: string }[] = [
  { id: "inter", label: "Sade", family: "var(--f-body)" },
  { id: "mono", label: "Daktilo", family: '"JetBrains Mono", ui-monospace, Consolas, monospace' },
  { id: "hand", label: "El yazısı", family: '"Caveat", "Segoe Print", cursive' },
];

export const colorOf = (id: string) => NOTE_COLORS.find((c) => c.id === id) ?? NOTE_COLORS[0];
export const fontOf = (id: string) => NOTE_FONTS.find((f) => f.id === id) ?? NOTE_FONTS[0];
