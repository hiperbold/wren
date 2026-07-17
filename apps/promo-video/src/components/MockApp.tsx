import { fontStack, monoStack, theme } from "../theme";

// Generic window chrome (mock) — never a real app screenshot.
const TrafficLights: React.FC = () => (
  <div style={{ display: "flex", gap: 9 }}>
    {["#ff5f57", "#febc2e", "#28c840"].map((c) => (
      <div key={c} style={{ width: 15, height: 15, borderRadius: "50%", background: c }} />
    ))}
  </div>
);

export const Window: React.FC<{
  title: string;
  children: React.ReactNode;
  width?: number;
  accent?: boolean;
}> = ({ title, children, width = 820, accent }) => (
  <div
    style={{
      width,
      borderRadius: 20,
      overflow: "hidden",
      background: theme.panel,
      border: `1px solid ${accent ? theme.amber : theme.border}`,
      boxShadow: "0 40px 90px rgba(0,0,0,0.55)",
      fontFamily: fontStack,
    }}
  >
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 18,
        padding: "18px 22px",
        background: theme.panelHi,
        borderBottom: `1px solid ${theme.border}`,
      }}
    >
      <TrafficLights />
      <div style={{ color: theme.inkFaint, fontSize: 22, fontWeight: 500 }}>{title}</div>
    </div>
    <div style={{ padding: 34 }}>{children}</div>
  </div>
);

// Blinking cursor (deterministic by frame)
export const Caret: React.FC<{ on: boolean; color?: string; h?: number }> = ({
  on,
  color = theme.amber,
  h = 34,
}) => (
  <span
    style={{
      display: "inline-block",
      width: 3,
      height: h,
      marginLeft: 2,
      marginBottom: -4,
      background: color,
      opacity: on ? 1 : 0,
      borderRadius: 2,
    }}
  />
);

// Text that "lands" dictated: reveals `text` as progress goes 0..1.
export const revealText = (text: string, progress: number): string => {
  const n = Math.round(interpClamp(progress) * text.length);
  return text.slice(0, n);
};
const interpClamp = (p: number) => Math.max(0, Math.min(1, p));

export { fontStack, monoStack };
