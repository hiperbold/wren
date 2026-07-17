import { AbsoluteFill, Audio, interpolate, Sequence, staticFile } from "remotion";
import { Background } from "./components/Background";
import { ColdOpen, MultiAppScene, AnywhereBanner, ValuePropSlide, CTA } from "./scenes";

// Timeline (30fps, total 900 = 30s).
// Structure: hook → 3 real demos (cursor clicks → shortcut → bubble rises recording →
// voice dictates → transcribes → done) → 3 benefits → CTA.
const HOOK = [0, 72] as const;
const APP_DUR = 158;
const APP_VO_START = 30; // frame, within the app, where voice starts
const APP_CLICK = 20; // frame of mouse click
const APPS = [
  { target: "editor" as const, vo: "vo/dictation-editor.mp3", voLen: 68, from: 72 },
  { target: "browser" as const, vo: "vo/dictation-browser.mp3", voLen: 63, from: 230 },
  // Sarah in this window (compare timbre with Jessica in the other two).
  { target: "chat" as const, vo: "vo/dictation-chat-sarah.mp3", voLen: 97, from: 388 },
];
const DEMO_START = 72;
const DEMO_END = 546; // 72 + 3*158
const PROP_START = 546;
const PROP_DUR = 62;
const CTA_FROM = 732;

const sfx = (name: string) => staticFile(`sfx/${name}`);

// Window spans (absolute) where voice plays — to duck the music
const VOICE_SPANS = APPS.map((a) => [a.from + APP_VO_START, a.from + APP_VO_START + a.voLen]);

export const PromoVideo: React.FC = () => {
  return (
    <AbsoluteFill>
      <Background />

      {/* ── hook ── */}
      <Sequence from={HOOK[0]} durationInFrames={HOOK[1]}>
        <ColdOpen />
      </Sequence>

      {/* ── multi-app demos ── */}
      <Sequence from={DEMO_START} durationInFrames={DEMO_END - DEMO_START}>
        <AnywhereBanner />
      </Sequence>
      {APPS.map((a) => (
        <Sequence key={a.target} from={a.from} durationInFrames={APP_DUR}>
          <MultiAppScene target={a.target} voLen={a.voLen} />
        </Sequence>
      ))}

      {/* ── benefits (one at a time) ── */}
      {[0, 1, 2].map((i) => (
        <Sequence key={i} from={PROP_START + i * PROP_DUR} durationInFrames={PROP_DUR}>
          <ValuePropSlide index={i} />
        </Sequence>
      ))}

      {/* ── CTA ── */}
      <Sequence from={CTA_FROM} durationInFrames={900 - CTA_FROM}>
        <CTA />
      </Sequence>

      {/* ── audio ── */}
      {/* Custom ambient bed (synthesized, no license), with ducking under each voice. */}
      <Audio
        src={staticFile("music/ambient.wav")}
        volume={(f) => {
          const bed = interpolate(f, [0, 20, 880, 900], [0, 0.18, 0.18, 0], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          });
          let duck = 1;
          for (const [s, e] of VOICE_SPANS) {
            const d = interpolate(f, [s - 12, s, e, e + 12], [1, 0.4, 0.4, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            });
            duck = Math.min(duck, d);
          }
          return bed * duck;
        }}
      />

      {/* Voiceover per app (ElevenLabs, "Jessica" voice): the person dictating the text. */}
      {APPS.map((a) => (
        <Sequence key={`vo-${a.target}`} from={a.from + APP_VO_START}>
          <Audio src={staticFile(a.vo)} volume={1} />
        </Sequence>
      ))}

      {/* SFX: mouse click + "done" ding per app. */}
      {APPS.map((a) => (
        <Sequence key={`click-${a.target}`} from={a.from + APP_CLICK}>
          <Audio src={sfx("click.wav")} volume={0.4} />
        </Sequence>
      ))}
      {APPS.map((a) => (
        <Sequence key={`ding-${a.target}`} from={a.from + APP_VO_START + a.voLen + 8}>
          <Audio src={sfx("ding.wav")} volume={0.5} />
        </Sequence>
      ))}

      {/* Whooshes on transitions + ticks on benefit slides. */}
      {[34, DEMO_START, 230, 388, PROP_START, CTA_FROM].map((f) => (
        <Sequence key={`w${f}`} from={f}>
          <Audio src={sfx("whoosh.wav")} volume={0.3} />
        </Sequence>
      ))}
      {[PROP_START + PROP_DUR, PROP_START + 2 * PROP_DUR].map((f) => (
        <Sequence key={`t${f}`} from={f}>
          <Audio src={sfx("click.wav")} volume={0.22} />
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};
