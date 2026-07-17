import {
  AbsoluteFill,
  Img,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { fontStack, monoStack, theme } from "./theme";
import { Bubble } from "./components/Bubble";
import { Window, Caret, revealText } from "./components/MockApp";
import { Keyboard, Mic, Zap, ShieldCheck, Feather } from "./components/Icon";
import { Cursor } from "./components/Cursor";
import { BubbleState } from "./waveform";
import { t } from "./strings";

const clamp = (p: number) => Math.max(0, Math.min(1, p));
const blink = (frame: number, fps: number) => Math.floor((frame / fps) * 2) % 2 === 0;

// Small state label below the bubble
const StateLabel: React.FC<{ text: string; color: string }> = ({ text, color }) => (
  <div
    style={{
      marginTop: 30,
      fontFamily: fontStack,
      fontSize: 30,
      fontWeight: 600,
      letterSpacing: "0.14em",
      textTransform: "uppercase",
      color,
      display: "flex",
      alignItems: "center",
      gap: 14,
    }}
  >
    <span style={{ width: 12, height: 12, borderRadius: "50%", background: color }} />
    {text}
  </div>
);

// ── 1. Hook ───────────────────────────────────────────────────────────────────
// Beat 1 (0–CUT): pattern-interrupt — "Still typing everything?" + keyboard fades out.
// Beat 2 (CUT–end): turn — amber mic pulses + "Just talk." + category.
export const ColdOpen: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const CUT = 34;

  // beat 1
  const in1 = spring({ frame, fps, config: { damping: 200 } });
  const out1 = interpolate(frame, [CUT - 7, CUT], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const settle = interpolate(frame, [8, CUT], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const shake = Math.sin(frame * 1.4) * 4 * settle;

  // beat 2
  const in2 = spring({ frame: frame - CUT, fps, config: { damping: 13, mass: 0.6 } });
  const line2 = spring({ frame: frame - CUT - 4, fps, config: { damping: 200 } });
  const sub2 = spring({ frame: frame - CUT - 14, fps, config: { damping: 200 } });
  const micPulse = 1 + 0.08 * Math.sin((frame - CUT) * 0.5);

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      {/* Beat 1 */}
      {frame < CUT && (
        <div
          style={{
            position: "absolute",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 44,
            opacity: in1 * out1,
            transform: `translateX(${shake}px)`,
            padding: "0 90px",
            textAlign: "center",
          }}
        >
          <Keyboard size={150} color={theme.inkFaint} strokeWidth={1.8} />
          <div
            style={{
              fontFamily: fontStack,
              fontWeight: 800,
              fontSize: 92,
              letterSpacing: "-0.03em",
              color: theme.inkMuted,
              lineHeight: 1.05,
            }}
          >
            {t.hookProblem}
          </div>
        </div>
      )}

      {/* Beat 2 */}
      {frame >= CUT && (
        <div
          style={{
            position: "absolute",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 44,
            padding: "0 90px",
            textAlign: "center",
          }}
        >
          <div
            style={{
              transform: `scale(${interpolate(in2, [0, 1], [0.5, 1]) * micPulse})`,
              width: 180,
              height: 180,
              borderRadius: "50%",
              background: `${theme.amber}22`,
              border: `2px solid ${theme.amber}`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: `0 0 60px ${theme.amber}55`,
            }}
          >
            <Mic size={96} color={theme.amber} strokeWidth={2} />
          </div>
          <div
            style={{
              fontFamily: fontStack,
              fontWeight: 800,
              fontSize: 110,
              letterSpacing: "-0.03em",
              color: theme.ink,
              opacity: line2,
              transform: `translateY(${interpolate(line2, [0, 1], [24, 0])}px)`,
              textShadow: "0 4px 30px rgba(0,0,0,0.5)",
            }}
          >
            {t.tagline}
          </div>
          <div
            style={{
              fontFamily: fontStack,
              fontWeight: 500,
              fontSize: 42,
              lineHeight: 1.25,
              color: theme.inkMuted,
              opacity: sub2,
              transform: `translateY(${interpolate(sub2, [0, 1], [16, 0])}px)`,
            }}
          >
            {t.taglineSub}
          </div>
        </div>
      )}
    </AbsoluteFill>
  );
};

// ── 2. Recording ──────────────────────────────────────────────────────────────
export const RecordScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const appear = spring({ frame, fps, config: { damping: 200 } });

  // hint fades when voice starts
  const hintOut = interpolate(frame, [30, 45], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // words emerge one by one — synchronized with voiceover (starts ~frame 92
  // absolute = ~20 in this scene; voice lasts ~2.3s ≈ 68 frames).
  const words = t.spokenWords;
  const startAt = 18;
  const perWord = 10;
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      {/* shortcut hint */}
      <div
        style={{
          position: "absolute",
          top: "24%",
          opacity: hintOut,
          fontFamily: fontStack,
          fontSize: 34,
          fontWeight: 600,
          color: theme.inkFaint,
          display: "flex",
          gap: 16,
          alignItems: "center",
        }}
      >
        <Kbd>Fn</Kbd>
        <span>+</span>
        <Kbd>Space</Kbd>
        <span style={{ marginLeft: 12 }}>{t.pressHint}</span>
      </div>

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center" }}>
        <Bubble state="recording" scale={3.2} appear={appear} />
        <StateLabel text={t.stateListening} color={theme.recDot} />
      </div>

      {/* speech emerging */}
      <div
        style={{
          position: "absolute",
          bottom: "20%",
          left: 0,
          right: 0,
          padding: "0 100px",
          textAlign: "center",
          fontFamily: fontStack,
          fontWeight: 800,
          fontSize: 60,
          lineHeight: 1.15,
          color: theme.ink,
          letterSpacing: "-0.02em",
        }}
      >
        {words.map((w, i) => {
          const ws = spring({
            frame: frame - (startAt + i * perWord),
            fps,
            config: { damping: 200 },
          });
          return (
            <span
              key={i}
              style={{
                opacity: ws,
                display: "inline-block",
                marginRight: 16,
                transform: `translateY(${interpolate(ws, [0, 1], [14, 0])}px)`,
              }}
            >
              {w}
            </span>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const Kbd: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <span
    style={{
      fontFamily: monoStack,
      fontSize: 30,
      padding: "8px 16px",
      borderRadius: 10,
      background: theme.panelHi,
      border: `1px solid ${theme.border}`,
      color: theme.inkMuted,
    }}
  >
    {children}
  </span>
);

// ── 3. Processing + Done ──────────────────────────────────────────────────────
export const ProcessScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  // turns "done" at the midpoint of the scene
  const half = Math.round(durationInFrames * 0.45);
  const isDone = frame >= half;
  const pop = spring({ frame: frame - half, fps, config: { damping: 12, mass: 0.6 } });
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          transform: isDone ? `scale(${interpolate(pop, [0, 1], [0.9, 1])})` : "none",
        }}
      >
        <Bubble state={isDone ? "done" : "processing"} scale={3.2} />
        <StateLabel
          text={isDone ? t.stateDone : t.stateThinking}
          color={isDone ? theme.done : theme.amber}
        />
      </div>
    </AbsoluteFill>
  );
};

// ── 4. Multi-app: text lands wherever the cursor is ────────────────────────────
type Target = "editor" | "browser" | "chat";


const EditorMock: React.FC<{ text: string; caretOn: boolean }> = ({ text, caretOn }) => (
  <Window title={t.appEditor} accent width={860}>
    <div style={{ fontFamily: monoStack, fontSize: 30, lineHeight: 1.7 }}>
      <div style={{ color: theme.inkFaint }}>
        <span style={{ color: "#c586c0" }}>fn</span>{" "}
        <span style={{ color: "#dcdcaa" }}>ship_release</span>() {"{"}
      </div>
      <div style={{ paddingLeft: 40, color: theme.inkFaint }}>
        <span style={{ color: "#6a9955" }}>// {text}</span>
        <Caret on={caretOn} />
      </div>
      <div style={{ color: theme.inkFaint }}>{"}"}</div>
    </div>
  </Window>
);

const BrowserMock: React.FC<{ text: string; caretOn: boolean }> = ({ text, caretOn }) => (
  <Window title={t.appBrowser} accent width={860}>
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 18,
        padding: "20px 26px",
        borderRadius: 999,
        background: theme.bg,
        border: `1px solid ${theme.border}`,
      }}
    >
      <svg width="30" height="30" viewBox="0 0 24 24" fill="none">
        <circle cx="11" cy="11" r="7" stroke={theme.inkFaint} strokeWidth="2" />
        <path d="M16 16l5 5" stroke={theme.inkFaint} strokeWidth="2" strokeLinecap="round" />
      </svg>
      <span style={{ fontFamily: fontStack, fontSize: 32, color: theme.ink }}>
        {text}
        <Caret on={caretOn} h={30} />
      </span>
    </div>
  </Window>
);

const ChatMock: React.FC<{ text: string; caretOn: boolean }> = ({ text, caretOn }) => (
  <Window title={t.appChat} accent width={860}>
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      <div style={{ display: "flex", gap: 16, alignItems: "flex-start", opacity: 0.6 }}>
        <div style={{ width: 44, height: 44, borderRadius: 12, background: theme.amberDeep }} />
        <div style={{ fontFamily: fontStack, fontSize: 28, color: theme.inkMuted }}>
          <b style={{ color: theme.ink }}>ana</b> &nbsp; can you take the hotfix?
        </div>
      </div>
      <div
        style={{
          alignSelf: "flex-end",
          maxWidth: "80%",
          padding: "20px 26px",
          borderRadius: "20px 20px 6px 20px",
          background: theme.amberDeep,
          color: "#1a120b",
          fontFamily: fontStack,
          fontSize: 30,
          fontWeight: 600,
        }}
      >
        {text}
        <Caret on={caretOn} color="#1a120b" h={28} />
      </div>
    </div>
  </Window>
);

// Demo per app: fake cursor moves to window and clicks → shortcut → bubble RISES
// already recording (waveform responds to voice) → voice dictates text → transcribes → done.
// `voLen` = voice duration in frames (syncs the typing).
export const MultiAppScene: React.FC<{ target: Target; voLen: number }> = ({
  target,
  voLen,
}) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames, width, height } = useVideoConfig();
  const wide = width > height;

  const enter = spring({ frame, fps, config: { damping: 200 } });
  const out = interpolate(frame, [durationInFrames - 12, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Layout by orientation. Portrait: window on top, bubble rises below (approved).
  // Landscape: window on left, bubble rises on right (leverages width).
  const clampBoth = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
  const winX = wide ? Math.round(width * 0.3) : 560;
  const winY = wide ? Math.round(height * 0.5) : 820;
  const bubbleX = wide ? Math.round(width * 0.75) : Math.round(width * 0.5);
  const bubbleY = wide ? Math.round(height * 0.52) : Math.round(height * 0.66);
  const bubbleScale = wide ? 2.6 : 2.4;
  const startX = wide ? Math.round(width * 0.92) : 920;
  const startY = wide ? Math.round(height * 0.9) : 1460;

  // cursor: from corner to target (0→CLICK), click at CLICK
  const CLICK = 20;
  const targetX = winX;
  const targetY = winY;
  const cx = interpolate(frame, [0, CLICK], [startX, targetX], clampBoth);
  const cy = interpolate(frame, [0, CLICK], [startY, targetY], clampBoth);
  const pressed = frame >= CLICK && frame < CLICK + 6;
  const ripple = interpolate(frame, [CLICK, CLICK + 18], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const rippleShow = frame >= CLICK && frame < CLICK + 18;

  // bubble rises after click; state: recording → processing → done
  const RISE = 26;
  const rise = spring({ frame: frame - RISE, fps, config: { damping: 16 } });
  const VO_START = 30;
  const voiceEnd = VO_START + voLen;
  const bubbleState: BubbleState =
    frame < voiceEnd ? "recording" : frame < voiceEnd + 8 ? "processing" : "done";
  const labelColor =
    bubbleState === "recording"
      ? theme.recDot
      : bubbleState === "processing"
        ? theme.amber
        : theme.done;
  const labelText =
    bubbleState === "recording"
      ? t.stateListening
      : bubbleState === "processing"
        ? t.stateThinking
        : t.stateDone;

  // texto sincronizado com a voz
  const typeP = clamp((frame - VO_START) / voLen);
  const full =
    target === "editor" ? t.dictatedEditor : target === "browser" ? t.dictatedBrowser : t.dictatedChat;
  const shown = revealText(full, typeP);
  const caretOn = typeP < 1 ? true : blink(frame, fps);

  const windowMock = (
    <>
      {target === "editor" && <EditorMock text={shown} caretOn={caretOn} />}
      {target === "browser" && <BrowserMock text={shown} caretOn={caretOn} />}
      {target === "chat" && <ChatMock text={shown} caretOn={caretOn} />}
    </>
  );

  return (
    <AbsoluteFill style={{ opacity: out }}>
      {wide ? (
        <>
          {/* window on the left */}
          <div
            style={{
              position: "absolute",
              left: winX,
              top: winY,
              transform: `translate(-50%, -50%) translateY(${interpolate(enter, [0, 1], [40, 0])}px)`,
              opacity: enter,
            }}
          >
            {windowMock}
          </div>
          {/* bubble that RISES on the right */}
          <div
            style={{
              position: "absolute",
              left: bubbleX,
              top: bubbleY,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              opacity: rise,
              transform: `translate(-50%, -50%) translateY(${interpolate(rise, [0, 1], [120, 0])}px)`,
            }}
          >
            <Bubble state={bubbleState} scale={bubbleScale} />
            <StateLabel text={labelText} color={labelColor} />
          </div>
        </>
      ) : (
        <>
          {/* window (shifted up to make room for bubble below) */}
          <AbsoluteFill
            style={{ alignItems: "center", justifyContent: "center", paddingBottom: 420 }}
          >
            <div
              style={{
                transform: `translateY(${interpolate(enter, [0, 1], [40, 0])}px)`,
                opacity: enter,
              }}
            >
              {windowMock}
            </div>
          </AbsoluteFill>

          {/* bubble that RISES, recording with waveform responding to voice */}
          <div
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              top: "60%",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              opacity: rise,
              transform: `translateY(${interpolate(rise, [0, 1], [120, 0])}px)`,
            }}
          >
            <Bubble state={bubbleState} scale={bubbleScale} />
            <StateLabel text={labelText} color={labelColor} />
          </div>
        </>
      )}

      {/* click ripple */}
      {rippleShow && (
        <div
          style={{
            position: "absolute",
            left: targetX - ripple * 45,
            top: targetY - ripple * 45,
            width: 12 + ripple * 90,
            height: 12 + ripple * 90,
            borderRadius: "50%",
            border: `3px solid ${theme.amber}`,
            opacity: (1 - ripple) * 0.8,
          }}
        />
      )}

      {/* fake cursor */}
      <div style={{ position: "absolute", left: cx - 8, top: cy - 6 }}>
        <Cursor pressed={pressed} />
      </div>
    </AbsoluteFill>
  );
};

export const AnywhereBanner: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame, fps, config: { damping: 200 } });
  return (
    <div
      style={{
        position: "absolute",
        top: "16%",
        left: 0,
        right: 0,
        textAlign: "center",
        opacity: s,
        fontFamily: fontStack,
        fontSize: 54,
        fontWeight: 800,
        letterSpacing: "-0.02em",
        color: theme.amber,
        textShadow: "0 4px 30px rgba(0,0,0,0.5)",
      }}
    >
      {t.anywhere}
    </div>
  );
};

// ── 5. Value props — one at a time, full screen, large icon ──────────────────
const PROP_ICONS = [Zap, ShieldCheck, Feather];

export const ValuePropSlide: React.FC<{ index: number }> = ({ index }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const p = t.props[index];
  const IconCmp = PROP_ICONS[index];

  const pop = spring({ frame, fps, config: { damping: 11, mass: 0.7 } });
  const title = spring({ frame: frame - 6, fps, config: { damping: 200 } });
  const sub = spring({ frame: frame - 14, fps, config: { damping: 200 } });
  const out = interpolate(frame, [durationInFrames - 8, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        flexDirection: "column",
        opacity: out,
        padding: "0 80px",
        textAlign: "center",
      }}
    >
      {/* icon badge */}
      <div
        style={{
          width: 210,
          height: 210,
          borderRadius: "50%",
          background: `${theme.amber}1c`,
          border: `2px solid ${theme.amber}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginBottom: 56,
          transform: `scale(${interpolate(pop, [0, 1], [0.4, 1])}) rotate(${interpolate(
            pop,
            [0, 1],
            [-12, 0],
          )}deg)`,
          boxShadow: `0 0 70px ${theme.amber}44`,
        }}
      >
        <IconCmp size={112} color={theme.amber} strokeWidth={2} />
      </div>

      <div
        style={{
          fontFamily: fontStack,
          fontWeight: 800,
          fontSize: 84,
          letterSpacing: "-0.03em",
          color: theme.ink,
          lineHeight: 1.05,
          opacity: title,
          transform: `translateY(${interpolate(title, [0, 1], [26, 0])}px)`,
          textShadow: "0 4px 30px rgba(0,0,0,0.5)",
        }}
      >
        {p.title}
      </div>
      <div
        style={{
          marginTop: 26,
          fontFamily: fontStack,
          fontWeight: 500,
          fontSize: 40,
          lineHeight: 1.3,
          color: theme.inkMuted,
          opacity: sub,
          transform: `translateY(${interpolate(sub, [0, 1], [18, 0])}px)`,
        }}
      >
        {p.sub}
      </div>

      {/* progress indicator (how many benefits) */}
      <div style={{ position: "absolute", bottom: "20%", display: "flex", gap: 14 }}>
        {t.props.map((_, i) => (
          <div
            key={i}
            style={{
              width: i === index ? 44 : 14,
              height: 14,
              borderRadius: 7,
              background: i === index ? theme.amber : theme.border,
              transition: "all 0.2s",
            }}
          />
        ))}
      </div>
    </AbsoluteFill>
  );
};

// ── 6. CTA ────────────────────────────────────────────────────────────────────
export const CTA: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame, fps, config: { damping: 200 } });
  const lineO = interpolate(frame, [16, 34], [0, 1], { extrapolateRight: "clamp" });
  const repo = spring({ frame: frame - 30, fps, config: { damping: 14, mass: 0.6 } });
  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        flexDirection: "column",
        padding: "0 90px",
        textAlign: "center",
      }}
    >
      <Img
        src={staticFile("wren-wordmark-dark.png")}
        style={{
          width: 440,
          opacity: s,
          transform: `translateY(${interpolate(s, [0, 1], [30, 0])}px)`,
          marginBottom: 80,
        }}
      />
      <div
        style={{
          fontFamily: fontStack,
          fontSize: 48,
          fontWeight: 600,
          color: theme.inkMuted,
          opacity: lineO,
          lineHeight: 1.25,
          marginBottom: 64,
        }}
      >
        {t.ctaLine}
      </div>
      {/* download button (conversion action) */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 16,
          fontFamily: fontStack,
          fontSize: 38,
          fontWeight: 700,
          color: "#1a120b",
          opacity: repo,
          transform: `scale(${interpolate(repo, [0, 1], [0.9, 1])})`,
          padding: "22px 40px",
          borderRadius: 16,
          background: theme.amber,
          boxShadow: `0 20px 50px ${theme.amberDeep}55`,
        }}
      >
        <span style={{ fontSize: 34 }}>↓</span>
        <span style={{ fontFamily: monoStack, fontSize: 34 }}>{t.ctaRepo}</span>
      </div>
      {/* platforms — honest: Linux only today */}
      <div
        style={{
          marginTop: 26,
          fontFamily: fontStack,
          fontSize: 28,
          color: theme.inkFaint,
          opacity: interpolate(frame, [40, 56], [0, 1], { extrapolateRight: "clamp" }),
        }}
      >
        {t.ctaPlatforms}
      </div>
    </AbsoluteFill>
  );
};
