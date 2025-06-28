import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
  setupAnime4K,
  type Anime4KConfig,
  type Anime4KController,
} from "../anime4k/player";
import { VideoControls } from "./VideoControls";
import type { CompareConfig } from "./constants";

export function VideoPlayer({
  src,
  config,
  compare,
  onUpdateConfig,
  onUpdateCompare,
}: {
  readonly src: string;
  readonly config: Anime4KConfig | null;
  readonly compare: CompareConfig;
  readonly onUpdateConfig: (config: Anime4KConfig | null) => void;
  readonly onUpdateCompare: (compare: CompareConfig) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  const [video, setVideo] = useState<HTMLVideoElement | null>(null);
  const controllerRef = useRef<Anime4KController | null>(null);

  useEffect(() => {
    if (!canvasRef.current || !videoRef.current || !config) {
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
  }, [config]);

  const fullscreen = useCallback((): void => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    if (container.requestFullscreen) {
      container.requestFullscreen();
    }
  }, []);

  const compareStyle = {
    none: {},
    onyx: {
      opacity: compare.ratio,
    },
    left: {
      clipPath: `inset(0 ${100 - compare.ratio * 100}% 0 0)`,
    },
    right: {
      clipPath: `inset(0 0 0 ${100 - compare.ratio * 100}%)`,
    },
    top: {
      clipPath: `inset(0 0 ${100 - compare.ratio * 100}% 0)`,
    },
    bottom: {
      clipPath: `inset(${100 - compare.ratio * 100}% 0 0 0)`,
    },
  }[compare.mode];

  return (
    <div ref={containerRef} class="relative w-full h-full">
      <video
        ref={videoRef}
        class="w-full h-full object-contain"
        src={src}
        onError={(e) => console.error("❌ Video error:", e)}
      >
        <track kind="captions" />
      </video>
      <canvas
        ref={canvasRef}
        class="absolute w-full h-full inset-0 object-contain pointer-events-none"
        style={compareStyle}
      />
      {video && (
        <VideoControls
          video={video}
          config={config}
          compare={compare}
          onUpdateConfig={(newConfig) => {
            if (newConfig) {
              controllerRef.current?.updateConfig(newConfig);
            }
            onUpdateConfig(newConfig);
          }}
          onUpdateCompare={onUpdateCompare}
          onFullscreen={fullscreen}
        />
      )}
    </div>
  );
}
