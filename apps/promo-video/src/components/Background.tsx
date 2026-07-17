import { AbsoluteFill, useCurrentFrame } from "remotion";
import { theme } from "../theme";

// Dark amber background with subtle breathing glow. Consistent across all scenes.
export const Background: React.FC<{ glow?: string }> = ({ glow = theme.amberDeep }) => {
  const frame = useCurrentFrame();
  const breathe = 0.5 + 0.5 * Math.sin(frame / 40);
  return (
    <AbsoluteFill style={{ background: theme.bg }}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(120% 70% at 50% 18%, ${glow}22 0%, transparent 55%)`,
          opacity: 0.6 + 0.4 * breathe,
        }}
      />
      <AbsoluteFill
        style={{
          background: `radial-gradient(90% 60% at 50% 108%, ${theme.bgDeep} 0%, transparent 60%)`,
        }}
      />
    </AbsoluteFill>
  );
};
