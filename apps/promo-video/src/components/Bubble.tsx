import { interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { theme, shader } from "../theme";
import { barLevels, BubbleState } from "../waveform";

// Web recreation of the native overlay (pill 320x200 logical from WGSL shader).
// Here we render only the "pill" with bars + state dot + amber border.
// Faithful to apps/desktop/src-tauri/src/overlay_native.rs — never screen-captured.

type Props = {
  state: BubbleState;
  scale?: number;
  // appearance (0..1) to animate entry/exit
  appear?: number;
};

const LOGICAL_W = 320;
const LOGICAL_H = 96; // we use only the pill strip (not the full 200 of app canvas)

export const Bubble: React.FC<Props> = ({ state, scale = 2.6, appear = 1 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = frame / fps;

  const levels = barLevels(state, t);
  const barCount = shader.BAR_COUNT;

  const accent =
    state === "done" ? theme.done : state === "processing" ? theme.amber : theme.amber;
  const dot =
    state === "done" ? theme.done : state === "processing" ? theme.amber : theme.recDot;

  const padX = 26;
  const usableW = LOGICAL_W - padX * 2;
  const gap = usableW / barCount;
  const barW = gap * 0.55;
  const midY = LOGICAL_H / 2;
  const maxBar = LOGICAL_H * 0.62;

  const w = LOGICAL_W * scale;
  const h = LOGICAL_H * scale;

  const enter = interpolate(appear, [0, 1], [0.82, 1]);
  const dotPulse =
    state === "recording" ? 0.6 + 0.4 * Math.abs(Math.sin(t * 4.2)) : 1;

  return (
    <div
      style={{
        transform: `scale(${enter})`,
        opacity: appear,
        filter: "drop-shadow(0 24px 60px rgba(0,0,0,0.55))",
      }}
    >
      <svg
        width={w}
        height={h}
        viewBox={`0 0 ${LOGICAL_W} ${LOGICAL_H}`}
        style={{ display: "block", overflow: "visible" }}
      >
        {/* pill */}
        <rect
          x={1.5}
          y={1.5}
          rx={LOGICAL_H / 2 - 1.5}
          ry={LOGICAL_H / 2 - 1.5}
          width={LOGICAL_W - 3}
          height={LOGICAL_H - 3}
          fill={theme.bubbleBg}
          stroke={accent}
          strokeWidth={1.5}
          strokeOpacity={0.35}
        />

        {/* state dot */}
        <circle
          cx={padX - 6}
          cy={midY}
          r={5}
          fill={dot}
          opacity={dotPulse}
        />

        {/* bars */}
        <g>
          {levels.map((lv, i) => {
            const bh = Math.max(2, lv * maxBar);
            const x = padX + 6 + i * gap;
            return (
              <rect
                key={i}
                x={x}
                y={midY - bh / 2}
                width={barW}
                height={bh}
                rx={barW / 2}
                fill={accent}
                opacity={0.92}
              />
            );
          })}
        </g>
      </svg>
    </div>
  );
};
