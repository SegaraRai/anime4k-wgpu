import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { Anime4KConfig } from "../anime4k/player";
import type {
  Anime4KPerformancePreset,
  Anime4KPreset,
} from "../anime4k/presets";

export interface CompareConfig {
  readonly mode: "none" | "onyx" | "left" | "right" | "top" | "bottom";
  readonly ratio: number;
}

function formatTime(
  current: number | null,
  duration: number | null
): [string, string] {
  if (current === null || duration === null) {
    return ["0:00", "0:00"];
  }

  const format = (time: number): string => {
    const hours = Math.floor(time / 3600);
    const minutes = Math.floor((time % 3600) / 60);
    const seconds = Math.floor(time % 60);
    return duration >= 3600
      ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
      : duration >= 600
        ? `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
        : `${minutes}:${String(seconds).padStart(2, "0")}`;
  };

  return [format(current), format(duration)];
}

function Slider({
  current,
  buffered,
  duration,
  onChange,
}: {
  readonly current: number | null;
  readonly buffered: number | null;
  readonly duration: number | null;
  readonly onChange: (value: number) => void;
}) {
  const [draggingPosition, setDraggingPosition] = useState<number | null>(null);
  const [sliderRect, setSliderRect] = useState<DOMRect | null>(null);

  useEffect(() => {
    if (draggingPosition == null || duration == null || sliderRect == null) {
      return;
    }

    const controller = new AbortController();
    const { signal } = controller;

    document.addEventListener(
      "mousemove",
      (event: MouseEvent): void => {
        const offsetX = event.clientX - sliderRect.left;
        const clampedOffsetX = Math.max(0, Math.min(offsetX, sliderRect.width));
        const newPosition = (clampedOffsetX / sliderRect.width) * duration;
        setDraggingPosition(newPosition);
      },
      { signal }
    );

    document.addEventListener(
      "mouseup",
      (): void => {
        if (draggingPosition == null) {
          return;
        }

        onChange(draggingPosition);
        setDraggingPosition(null);
        setSliderRect(null);
      },
      { signal }
    );

    return () => {
      controller.abort();
    };
  }, [draggingPosition, duration, sliderRect, onChange]);

  const handleMouseDown = useCallback(
    (event: MouseEvent): void => {
      if (duration == null) {
        return;
      }

      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      const offsetX = event.clientX - rect.left;
      const newPosition = (offsetX / rect.width) * duration;
      setDraggingPosition(newPosition);
      setSliderRect(rect);
    },
    [duration]
  );

  return (
    <div
      class="relative w-full h-4 cursor-pointer group/slider"
      data-dragging={draggingPosition != null ? 1 : undefined}
      onMouseDown={handleMouseDown}
    >
      {duration != null && (
        <>
          <div class="absolute w-full h-1 inset-0 my-auto bg-gray-700 rounded-full"></div>
          <div
            class="absolute inset-[0_auto_0_0] my-auto h-1 bg-gray-600 rounded-full"
            style={{
              width: `${((buffered ?? 0) / duration) * 100}%`,
            }}
          ></div>
          <div
            class="absolute inset-[0_auto_0_0] my-auto h-1 bg-gray-300 rounded-full"
            style={{
              width: `${((draggingPosition ?? current ?? 0) / duration) * 100}%`,
            }}
          ></div>
          <button
            class="absolute top-0 bottom-0 left-0 size-4 bg-white rounded-full -translate-x-2 opacity-0 group-hover/slider:opacity-100 group-focus-within/slider:opacity-100 group-[[data-dragging]]/slider:opacity-100 transition-opacity"
            style={{
              left: `${((draggingPosition ?? current ?? 0) / duration) * 100}%`,
            }}
          ></button>
        </>
      )}
    </div>
  );
}

export function CompareController({
  value: { mode, ratio },
  onChange,
}: {
  readonly value: CompareConfig;
  readonly onChange: (compare: CompareConfig) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [draggingRatio, setDraggingRatio] = useState<number | null>(null);
  const [containerRect, setSliderRect] = useState<DOMRect | null>(null);

  const calcRatio = (
    mode: "left" | "right" | "top" | "bottom",
    rect: DOMRect,
    event: MouseEvent
  ): number => {
    const position =
      mode === "left" || mode === "right"
        ? (event.clientX - rect.left) / rect.width
        : (event.clientY - rect.top) / rect.height;
    const ratio = mode === "left" || mode === "top" ? position : 1 - position;
    return Math.max(0, Math.min(ratio, 1));
  };

  useEffect(() => {
    if (draggingRatio == null || containerRect == null) {
      return;
    }

    const controller = new AbortController();
    const { signal } = controller;

    document.addEventListener(
      "mousemove",
      (event: MouseEvent): void => {
        if (mode === "none" || mode === "onyx") {
          return;
        }

        const newDraggingRatio = calcRatio(mode, containerRect, event);
        setDraggingRatio(newDraggingRatio);
        onChange({
          mode,
          ratio: newDraggingRatio,
        });
      },
      { signal }
    );

    document.addEventListener(
      "mouseup",
      (): void => {
        if (draggingRatio == null) {
          return;
        }

        onChange({
          mode,
          ratio: draggingRatio,
        });
        setDraggingRatio(null);
        setSliderRect(null);
      },
      { signal }
    );

    return () => {
      controller.abort();
    };
  }, [mode, draggingRatio, containerRect, onChange]);

  const handleMouseDown = useCallback(
    (event: MouseEvent): void => {
      if (mode === "none" || mode === "onyx" || !containerRef.current) {
        return;
      }

      const rect = containerRef.current.getBoundingClientRect();
      setDraggingRatio(calcRatio(mode, rect, event));
      setSliderRect(rect);
    },
    [mode]
  );

  if (mode === "none") {
    return null;
  }

  if (mode === "onyx") {
    return (
      <div class="absolute inset-[0_0_auto_0] w-60 mx-auto my-4 px-4 py-2 bg-gray-400/40 rounded-lg">
        <input
          type="range"
          class="range range-xs text-white [--range-bg:#555] [--range-thumb:white] [--range-fill:0]"
          value={ratio * 100}
          min={0}
          max={100}
          step={1}
          onInput={(event) => {
            const target = event.target as HTMLInputElement;
            onChange({
              mode: "onyx",
              ratio: parseFloat(target.value) / 100,
            });
          }}
        />
      </div>
    );
  }

  const [varClass, iconClass] = {
    left: [
      "[--inset:0_0_0_var(--ratio)] [--w:var(--spacing)] [--transform:translateX(-50%)] [--margin:auto_0]",
      "icon-[akar-icons--chevron-horizontal]",
    ],
    right: [
      "[--inset:0_0_0_calc(100%-var(--ratio))] [--w:var(--spacing)] [--transform:translateX(-50%)] [--margin:auto_0]",
      "icon-[akar-icons--chevron-horizontal]",
    ],
    top: [
      "[--inset:var(--ratio)_0_0_0] [--h:var(--spacing)] [--transform:translateY(-50%)] [--margin:0_auto]",
      "icon-[akar-icons--chevron-vertical]",
    ],
    bottom: [
      "[--inset:calc(100%-var(--ratio))_0_0_0] [--h:var(--spacing)] [--transform:translateY(-50%)] [--margin:0_auto]",
      "icon-[akar-icons--chevron-vertical]",
    ],
  }[mode];

  return (
    <div
      ref={containerRef}
      class={`absolute inset-0 w-full h-full ${varClass}`}
      style={{ "--ratio": `${(draggingRatio ?? ratio) * 100}%` }}
    >
      <div class="absolute inset-[var(--inset)] transform-[var(--transform)] w-[var(--w,100%)] h-[var(--h,100%)] bg-white/80" />
      <button
        class="absolute inset-[var(--inset)] transform-[var(--transform)] m-[var(--margin)] btn btn-circle btn-md btn-soft"
        onMouseDown={handleMouseDown}
      >
        <span class={`size-5 ${iconClass}`} />
      </button>
    </div>
  );
}

const MIN_SCALE_FACTOR = 1;
const MAX_SCALE_FACTOR = 8;

const PRESETS: [Anime4KPreset, string][] = [
  ["a", "A (Restore → Upscale)"],
  ["b", "B (Restore Soft → Upscale)"],
  ["c", "C (Upscale Denoise)"],
  ["aa", "AA (Restore → Upscale → Restore)"],
  ["bb", "BB (Restore Soft → Upscale → Restore Soft)"],
  ["ca", "CA (Upscale Denoise → Restore)"],
];

const PERFORMANCE_PRESETS: [Anime4KPerformancePreset, string][] = [
  ["light", "Light (Fast, Low Quality)"],
  ["medium", "Medium (Balanced)"],
  ["high", "High (Slow, High Quality)"],
  ["ultra", "Ultra (Very Slow, Very High Quality)"],
  ["extreme", "Extreme (Insane Quality)"],
];

const FALLBACK_CONFIG: Anime4KConfig = {
  preset: "a",
  performance: "medium",
  scale: 2,
};

const COMPARE_MODES: [CompareConfig["mode"], string][] = [
  ["left", "Split Left/Right"],
  ["right", "Split Right/Left"],
  ["top", "Split Top/Bottom"],
  ["bottom", "Split Bottom/Top"],
  ["onyx", "Blend Overlay"],
  ["none", "No Comparison"],
];

export function VideoControls({
  video,
  config,
  compare,
  onUpdateConfig,
  onUpdateCompare,
  onFullscreen,
}: {
  readonly video: HTMLVideoElement;
  readonly config: Anime4KConfig | null;
  readonly compare: CompareConfig;
  readonly onUpdateConfig: (config: Anime4KConfig | null) => void;
  readonly onUpdateCompare: (compare: CompareConfig) => void;
  readonly onFullscreen: () => void;
}) {
  const [lastConfig, setLastConfig] = useState<Anime4KConfig | null>(null);
  const displayConfig = config ?? lastConfig ?? FALLBACK_CONFIG;

  const updateConfig = (newConfig: Anime4KConfig | null): void => {
    if (newConfig) {
      setLastConfig(newConfig);
    }
    onUpdateConfig(newConfig);
  };

  if (
    config &&
    (config.preset !== displayConfig.preset ||
      config.performance !== displayConfig.performance ||
      config.scale !== displayConfig.scale)
  ) {
    setLastConfig(config);
  }

  const [isLoading, setIsLoading] = useState(true);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(true);
  const [currentTime, setCurrentTime] = useState<number | null>(null);
  const [duration, setDuration] = useState<number | null>(null);
  const [volume, setVolume] = useState<number | null>(null);
  const [buffered, setBuffered] = useState<number>(0);

  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;

    const update = (): void => {
      setIsLoading(video.readyState < 2);
      setCurrentTime(video.currentTime);
      setDuration(video.duration);
      setVolume(video.volume);
      setIsMuted(video.muted);
      setIsPlaying(!video.paused);
      setBuffered(video.buffered.length > 0 ? video.buffered.end(0) : 0);
    };

    video.addEventListener("loadeddata", update, { signal });
    video.addEventListener("loadedmetadata", update, { signal });
    video.addEventListener("canplay", update, { signal });
    video.addEventListener("play", update, { signal });
    video.addEventListener("pause", update, { signal });
    video.addEventListener("timeupdate", update, { signal });
    video.addEventListener("volumechange", update, { signal });
    video.addEventListener("seeking", update, { signal });
    video.addEventListener("seeked", update, { signal });
    video.addEventListener("durationchange", update, { signal });
    video.addEventListener("ended", update, { signal });

    update();

    return () => {
      controller.abort();
    };
  }, [video]);

  const togglePlayPause = useCallback((): void => {
    if (video.readyState < 2) {
      return;
    }

    if (video.paused) {
      video.play();
    } else {
      video.pause();
    }
  }, [video]);

  const toggleFullscreen = useCallback((): void => {
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      onFullscreen();
    }
  }, [onFullscreen]);

  const toggleCompare = useCallback(
    (reverse = false): void => {
      const currentIndex = COMPARE_MODES.findIndex(
        ([mode]) => mode === compare.mode
      );
      const offset = reverse ? -1 : 1;
      const nextIndex =
        (currentIndex + offset + COMPARE_MODES.length) % COMPARE_MODES.length;
      onUpdateCompare({
        ...compare,
        mode: COMPARE_MODES[nextIndex][0],
      });
    },
    [compare, onUpdateCompare]
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent): void => {
      switch (event.key) {
        case "Enter":
        case " ":
          event.preventDefault();
          togglePlayPause();
          break;

        case "f":
          event.preventDefault();
          toggleFullscreen();
          break;

        case "c":
          event.preventDefault();
          toggleCompare(false);
          break;

        case "C":
          event.preventDefault();
          toggleCompare(true);
          break;
      }
    },
    [togglePlayPause, toggleFullscreen, toggleCompare]
  );

  const setCompareMode = useCallback(
    (mode: CompareConfig["mode"]): void => {
      if (compare.mode === mode) {
        return;
      }
      onUpdateCompare({
        ...compare,
        mode,
      });
    },
    [compare, onUpdateCompare]
  );

  const [strCurrentTime, strDuration] = formatTime(currentTime, duration);

  return (
    <div
      class="absolute inset-0 flex flex-col justify-end group select-none touch-manipulation"
      data-show-controls={!isPlaying ? 1 : undefined}
      onClick={togglePlayPause}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      {/* Compare Controller */}
      <CompareController value={compare} onChange={onUpdateCompare} />
      {/* Video Controller */}
      <div class="relative bg-gradient-to-t from-[#000000f4] from-15% via-[#000000a0] via-50% opacity-0 group-hover:opacity-100 group-[[data-show-controls]]:opacity-100 has-[.dropdown:focus-within]:opacity-100 transition-opacity duration-400 w-full h-40">
        <div
          class="absolute inset-[auto_0_0_0] flex flex-col justify-between p-4"
          onClick={(event) => {
            event.stopPropagation();
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.stopPropagation();
            }
          }}
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-4">
              {/* Play/pause button */}
              <button
                class="flex-none btn btn-circle btn-ghost btn-neutral btn-md"
                aria-label={isPlaying ? "Pause" : "Play"}
                onClick={togglePlayPause}
              >
                <span
                  class={`size-5 ${
                    isPlaying
                      ? "icon-[akar-icons--pause]"
                      : "icon-[akar-icons--play]"
                  }`}
                ></span>
              </button>
              {/* Current time display */}
              <div class="flex-none text-white [font-feature-settings:'tnum'_'lnum'_'zero'_'ss01']">
                {strCurrentTime} / {strDuration}
              </div>
            </div>
            <div class="flex items-center gap-4">
              {/* Volume control */}
              <div class="flex items-center group/volume">
                <div class="w-0 flex items-center group-hover/volume:w-30 focus-within:w-30 rounded-full transition-all">
                  <input
                    type="range"
                    aria-label="Volume"
                    class="w-full range range-xs text-white [--range-bg:#555] [--range-thumb:white] [--range-fill:0]"
                    value={isMuted ? 0 : volume !== null ? volume * 100 : 100}
                    min={0}
                    max={100}
                    step={1}
                    onInput={(event) => {
                      const target = event.target as HTMLInputElement;
                      video.volume = parseFloat(target.value) / 100;
                      setVolume(video.volume);
                      setIsMuted(video.muted);
                    }}
                  />
                  <span class="w-2"></span>
                </div>
                <button
                  class="flex-none btn btn-circle btn-ghost btn-neutral btn-md"
                  aria-label="Toggle Mute"
                  onClick={() => {
                    video.muted = !video.muted;
                    setIsMuted(video.muted);
                  }}
                >
                  <span
                    class={`size-5 ${
                      isMuted
                        ? "icon-[akar-icons--sound-off]"
                        : "icon-[akar-icons--sound-on]"
                    }`}
                  ></span>
                </button>
              </div>
              {/* Fullscreen button */}
              <button
                class="flex-none btn btn-circle btn-ghost btn-neutral btn-md"
                aria-label="Toggle Fullscreen"
                onClick={() => {
                  toggleFullscreen();
                }}
              >
                <span class="size-5 icon-[akar-icons--full-screen]"></span>
              </button>
              {/* Comparison Mode */}
              <div class="dropdown dropdown-top dropdown-end">
                <div
                  tabindex={0}
                  role="button"
                  aria-label="Comparison Mode Menu"
                  class={`flex-none btn btn-circle btn-ghost btn-neutral btn-md ${compare.mode !== "none" ? "text-accent" : ""}`}
                >
                  <span class="size-5 icon-[akar-icons--align-to-middle] rotate-90"></span>
                </div>
                <ul
                  tabindex={0}
                  class="dropdown-content menu bg-base-100 rounded-box z-1 w-52 p-2 shadow-sm"
                >
                  {COMPARE_MODES.map(([mode, label]) => (
                    <li key={mode}>
                      <button
                        class={compare.mode === mode ? "menu-active" : ""}
                        aria-pressed={compare.mode === mode}
                        onClick={() => setCompareMode(mode)}
                      >
                        {label}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
              {/* Menu */}
              <div class="dropdown dropdown-top dropdown-end">
                <div
                  tabindex={0}
                  role="button"
                  aria-label="Anime4K Settings Menu"
                  class={`flex-none btn btn-circle btn-ghost btn-neutral btn-md ${config ? "text-accent" : ""}`}
                >
                  <span class="size-5 icon-[akar-icons--sparkles]"></span>
                </div>
                <div
                  tabindex={0}
                  class="card card-sm dropdown-content bg-base-100 rounded-box z-1 w-64 shadow-sm"
                >
                  <div tabindex={0} class="card-body">
                    <label class="label text-sm">
                      <input
                        type="checkbox"
                        class="toggle"
                        checked={config !== null}
                        onChange={(event) => {
                          if (event.currentTarget.checked) {
                            updateConfig(FALLBACK_CONFIG);
                          } else {
                            updateConfig(null);
                          }
                        }}
                      />
                      Enable Anime4K
                    </label>
                    <fieldset class="fieldset" disabled={!config}>
                      <legend class="fieldset-legend">Scale Factor</legend>
                      <input
                        type="range"
                        min={MIN_SCALE_FACTOR}
                        max={MAX_SCALE_FACTOR}
                        value={displayConfig.scale}
                        class="range"
                        step="1"
                        onInput={(event) => {
                          const target = event.target as HTMLInputElement;
                          const scale = parseFloat(target.value);
                          updateConfig({
                            ...displayConfig,
                            scale,
                          });
                        }}
                      />
                      <div
                        class="flex justify-between px-2.5 mt-2 text-xs"
                        aria-hidden="true"
                      >
                        {Array.from(
                          { length: MAX_SCALE_FACTOR - MIN_SCALE_FACTOR + 1 },
                          (_, i) => (
                            <span key={i}>{i + MIN_SCALE_FACTOR}</span>
                          )
                        )}
                      </div>
                    </fieldset>
                    <fieldset class="fieldset" disabled={!config}>
                      <legend class="fieldset-legend">Preset</legend>
                      <select
                        class="select"
                        onChange={(event) => {
                          const preset = event.currentTarget
                            .value as Anime4KPreset;
                          updateConfig({
                            ...displayConfig,
                            preset,
                          });
                        }}
                      >
                        {PRESETS.map(([preset, label]) => (
                          <option
                            key={preset}
                            value={preset}
                            selected={config?.preset === preset}
                          >
                            {label}
                          </option>
                        ))}
                      </select>
                    </fieldset>
                    <fieldset class="fieldset" disabled={!config}>
                      <legend class="fieldset-legend">Performance</legend>
                      <select
                        class="select"
                        onChange={(event) => {
                          const performance = event.currentTarget
                            .value as Anime4KPerformancePreset;
                          updateConfig({
                            ...displayConfig,
                            performance,
                          });
                        }}
                      >
                        {PERFORMANCE_PRESETS.map(([preset, label]) => (
                          <option
                            key={preset}
                            value={preset}
                            selected={displayConfig.performance === preset}
                          >
                            {label}
                          </option>
                        ))}
                      </select>
                    </fieldset>
                  </div>
                </div>
              </div>
            </div>
          </div>
          {/* Playback progress */}
          <div class="px-2">
            <Slider
              current={currentTime}
              buffered={buffered}
              duration={duration}
              onChange={(value) => {
                video.currentTime = value;
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
