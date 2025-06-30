import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
  setupAnime4K,
  type Anime4KConfig,
  type Anime4KController,
} from "../anime4k/player";
import { VideoControls, type Anime4KState } from "./VideoControls";
import type { CompareConfig } from "./constants";

export function VideoPlayer({
  src,
  config,
  compare,
  onUpdateConfig,
  onUpdateCompare,
  onSelectFile,
  onLoadedMetadata,
}: {
  readonly src: string;
  readonly config: Anime4KConfig | null;
  readonly compare: CompareConfig;
  readonly onUpdateConfig: (config: Anime4KConfig | null) => void;
  readonly onUpdateCompare: (compare: CompareConfig) => void;
  readonly onSelectFile: () => void;
  readonly onLoadedMetadata?: (event: Event) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);

  const [canvas, setCanvas] = useState<HTMLCanvasElement | null>(null);
  const [video, setVideo] = useState<HTMLVideoElement | null>(null);

  const canvasRefCallback = useCallback(
    (element: HTMLCanvasElement | null): void => {
      setCanvas(element);
    },
    []
  );
  const videoRefCallback = useCallback(
    (element: HTMLVideoElement | null): void => {
      setVideo(element);
    },
    []
  );

  const [controllerState, setControllerState] = useState<{
    controller: Anime4KController;
    video: HTMLVideoElement | null;
    canvas: HTMLCanvasElement | null;
  } | null>(null);
  const [anime4KState, setAnime4KState] = useState<Anime4KState>({
    type: "pending",
  });

  useEffect(() => {
    if (!canvas || !video) {
      return;
    }

    let cleanuped = false;

    const controller = setupAnime4K(canvas, video);
    setControllerState({
      controller,
      video,
      canvas,
    });

    controller.ready.then(
      () => {
        if (cleanuped) {
          console.debug("Anime4K setup was cleaned up before completion.");
          return;
        }
        console.info("✅ Anime4K setup complete");
        setAnime4KState({ type: "ready" });
      },
      (error) => {
        if (cleanuped) {
          console.debug("Anime4K setup was cleaned up before error handling.");
          return;
        }
        console.error("❌ Anime4K setup failed:", error);
        setAnime4KState({ type: "error", error });
      }
    );

    return (): void => {
      controller.cleanup();
      setControllerState(null);
      setAnime4KState({ type: "pending" });
    };
  }, [canvas, video]);

  useEffect(() => {
    if (!controllerState) {
      return;
    }

    controllerState.controller.updateConfig(config);
  }, [controllerState, config]);

  const handleFullscreen = useCallback((): void => {
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
    <div
      ref={containerRef}
      class="relative w-full h-full overflow-clip"
      data-theme="sunset"
    >
      <video
        ref={videoRefCallback}
        class="w-full h-full object-contain"
        src={src}
        onError={(e) => console.error("❌ Video error:", e)}
        onLoadedMetadata={onLoadedMetadata}
      >
        <track kind="captions" />
      </video>
      <canvas
        ref={canvasRefCallback}
        class="absolute w-full h-full inset-0 object-contain pointer-events-none"
        style={compareStyle}
      />
      {video && (
        <VideoControls
          video={video}
          config={config}
          compare={compare}
          anime4KState={anime4KState}
          onUpdateConfig={onUpdateConfig}
          onUpdateCompare={onUpdateCompare}
          onFullscreen={handleFullscreen}
          onSelectFile={onSelectFile}
        />
      )}
    </div>
  );
}
