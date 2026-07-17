import { interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { fontStack, theme } from "../theme";

// Large social caption, with optional amber highlight.
export const Caption: React.FC<{
  text: string;
  accent?: string;
  size?: number;
  y?: number;
  delay?: number;
}> = ({ text, accent, size = 64, y = 0, delay = 0 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame: frame - delay, fps, config: { damping: 200 } });
  const opacity = interpolate(s, [0, 1], [0, 1]);
  const ty = interpolate(s, [0, 1], [26, 0]);

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top: `calc(50% + ${y}px)`,
        transform: `translateY(${ty}px)`,
        opacity,
        textAlign: "center",
        padding: "0 90px",
        fontFamily: fontStack,
        fontWeight: 800,
        fontSize: size,
        lineHeight: 1.08,
        letterSpacing: "-0.02em",
        color: accent ?? theme.ink,
        textShadow: "0 4px 30px rgba(0,0,0,0.5)",
      }}
    >
      {text}
    </div>
  );
};
