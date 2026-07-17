import { Config } from "@remotion/cli/config";

// Vídeo social vertical: nitidez alta, cor consistente.
Config.setVideoImageFormat("jpeg");
Config.setPixelFormat("yuv420p");
Config.setCodec("h264");
Config.setCrf(18); // qualidade alta (menor = melhor)
Config.setChromiumOpenGlRenderer("angle");
Config.overrideWebpackConfig((c) => c);
