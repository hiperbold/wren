import { Composition } from "remotion";
import { PromoVideo } from "./PromoVideo";

// 30s @ 30fps. Vertical 1080x1920 (social feed: X, LinkedIn, Product Hunt, Reddit)
// + horizontal 1920x1080 (YouTube, landing, X post com preview wide).
export const FPS = 30;
export const DURATION_IN_FRAMES = 30 * FPS; // 900
export const WIDTH = 1080;
export const HEIGHT = 1920;

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="WrenPromo"
        component={PromoVideo}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={WIDTH}
        height={HEIGHT}
      />
      <Composition
        id="WrenPromoWide"
        component={PromoVideo}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={1920}
        height={1080}
      />
    </>
  );
};
