import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
  setupAnime4K,
  type Anime4KConfig,
  type Anime4KController,
} from "../anime4k/player";
import { VideoControls, type CompareConfig } from "./VideoControls";

function createCompareStyle(compare: CompareConfig) {
  switch (compare.mode) {
    case "onyx":
      return {
        opacity: compare.ratio,
      };

    case "left":
      return {
        clipPath: `inset(0 ${100 - compare.ratio * 100}% 0 0)`,
      };

    case "right":
      return {
        clipPath: `inset(0 0 0 ${100 - compare.ratio * 100}%)`,
      };

    case "top":
      return {
        clipPath: `inset(0 0 ${100 - compare.ratio * 100}% 0)`,
      };

    case "bottom":
      return {
        clipPath: `inset(${100 - compare.ratio * 100}% 0 0 0)`,
      };
  }

  return {};
}

export function VideoPlayer({ src }: { readonly src: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  const [video, setVideo] = useState<HTMLVideoElement | null>(null);
  const controllerRef = useRef<Anime4KController | null>(null);

  const [config, setConfig] = useState<Anime4KConfig | null>({
    preset: "a",
    performance: "light",
    scale: 2.0,
  });

  const [compare, setCompare] = useState<CompareConfig>({
    mode: "left",
    ratio: 0.5,
  });

  useEffect(() => {
    if (!canvasRef.current || !videoRef.current) {
      setVideo(null);
      return;
    }

    setVideo(videoRef.current);

    const controller = setupAnime4K(
      canvasRef.current,
      videoRef.current,
      config
    );
    controllerRef.current = controller;

    return (): void => {
      setVideo(null);
      controller.cleanup();
    };
  }, []);

  const fullscreen = useCallback((): void => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    if (container.requestFullscreen) {
      container.requestFullscreen();
    }
  }, []);

  return (
    <div ref={containerRef} class="relative w-screen h-screen bg-blue-100">
      <video
        ref={videoRef}
        class="w-full h-full object-contain"
        src={src}
        onError={(e) => console.error("❌ Video error:", e)}
      />
      <canvas
        ref={canvasRef}
        class="absolute w-full h-full inset-0 object-contain pointer-events-none"
        style={createCompareStyle(compare)}
      />
      {video && (
        <VideoControls
          video={video}
          config={config}
          compare={compare}
          onUpdateConfig={(newConfig) => {
            controllerRef.current?.updateConfig(newConfig);
            setConfig(newConfig);
          }}
          onUpdateCompare={setCompare}
          onFullscreen={fullscreen}
        />
      )}
    </div>
  );
}
