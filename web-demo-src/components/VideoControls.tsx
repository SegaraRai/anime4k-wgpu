import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { Anime4KConfig } from "../anime4k/player";
import type {
  Anime4KPerformancePreset,
  Anime4KPreset,
} from "../anime4k/presets";
import {
  COMPARE_MODES,
  DEFAULT_CONFIG,
  MAX_SCALE_FACTOR,
  MIN_SCALE_FACTOR,
  PERFORMANCE_PRESETS,
  PRESETS,
  type CompareConfig,
} from "./constants";
import { Toast } from "./Toast";
import { useDrag } from "./useDrag";
import { useToast } from "./useToast";

const SEEK_OFFSET_DIRECT = 10; // seconds
const SEEK_OFFSET_SLIDER = 10; // seconds
const VOLUME_OFFSET = 10;

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

function nanToNull(value: number | null): number | null {
  return value == null || isNaN(value) ? null : value;
}

function Slider({
  current,
  seeking,
  buffered,
  duration,
  onChange,
}: {
  readonly current: number | null;
  readonly seeking: number | null;
  readonly buffered: number | null;
  readonly duration: number | null;
  readonly onChange: (value: number) => void;
}) {
  const [draggingPosition, setDraggingPosition] = useState<number | null>(null);

  const onUpdateDrag = useCallback(
    (value: number): void => {
      setDraggingPosition(value * (duration ?? 0));
    },
    [duration]
  );

  const onEndDrag = useCallback(
    (value: number): void => {
      onChange(value * (duration ?? 0));
      setDraggingPosition(null);
    },
    [duration, onChange]
  );

  const { handleMouseDown, handleTouchStart } = useDrag(
    duration != null
      ? {
          onUpdate: onUpdateDrag,
          onEnd: onEndDrag,
        }
      : {}
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent): void => {
      if (duration == null || current == null) {
        return;
      }

      const key = event.key;
      if (draggingPosition != null && (key === "Enter" || key === " ")) {
        event.preventDefault();
        setDraggingPosition(null);
        onChange(draggingPosition);
      } else if (
        key === "ArrowLeft" ||
        key === "ArrowRight" ||
        key === "Enter" ||
        key === " "
      ) {
        const offset =
          (
            {
              ArrowLeft: -SEEK_OFFSET_SLIDER,
              ArrowRight: SEEK_OFFSET_SLIDER,
            } as Record<string, number | undefined>
          )[key] ?? 0;
        event.preventDefault();
        setDraggingPosition(
          (draggingPosition) => (draggingPosition ?? current ?? 0) + offset
        );
      }
    },
    [duration, current, draggingPosition]
  );

  return (
    <div
      class="relative w-full h-4 cursor-pointer group/slider"
      data-dragging={draggingPosition != null ? 1 : undefined}
      onKeyDown={handleKeyDown}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
        if (duration != null) {
          handleMouseDown(event.currentTarget, "x", event);
        }
      }}
      onTouchStart={(event) => {
        event.preventDefault();
        event.stopPropagation();
        if (duration != null) {
          handleTouchStart(event.currentTarget, "x", event);
        }
      }}
    >
      {duration != null && (
        <>
          <div class="absolute w-full h-1 inset-0 my-auto bg-gray-700 rounded-full" />
          <div
            class="absolute inset-[0_auto_0_0] my-auto h-1 bg-gray-600 rounded-full"
            style={{
              width: `${((buffered ?? 0) / duration) * 100}%`,
            }}
          />
          <div
            class="absolute inset-[0_auto_0_0] my-auto h-1 bg-gray-300 rounded-full"
            style={{
              width: `${((draggingPosition ?? seeking ?? current ?? 0) / duration) * 100}%`,
            }}
          />
          <button
            type="button"
            class="absolute top-0 bottom-0 left-0 size-4 bg-white rounded-full -translate-x-2 opacity-0 group-hover/slider:opacity-100 group-focus-within/slider:opacity-100 group-[[data-dragging]]/slider:opacity-100 transition-opacity cursor-pointer"
            style={{
              left: `${((draggingPosition ?? seeking ?? current ?? 0) / duration) * 100}%`,
            }}
          />
        </>
      )}
    </div>
  );
}

function calcRatio(
  mode: "left" | "right" | "top" | "bottom",
  rect: DOMRect,
  event: MouseEvent
): number {
  const position =
    mode === "left" || mode === "right"
      ? (event.clientX - rect.left) / rect.width
      : (event.clientY - rect.top) / rect.height;
  const ratio = mode === "left" || mode === "top" ? position : 1 - position;
  return Math.max(0, Math.min(ratio, 1));
}

export function CompareController({
  value: { mode, ratio },
  onChange,
}: {
  readonly value: CompareConfig;
  readonly onChange: (compare: CompareConfig) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);

  const onUpdateDrag = useCallback(
    (value: number): void => {
      onChange({
        mode,
        ratio: value,
      });
    },
    [mode, onChange]
  );

  const { handleMouseDown, handleTouchStart } = useDrag(
    mode !== "none" && mode !== "onyx"
      ? {
          onUpdate: onUpdateDrag,
          onEnd: onUpdateDrag,
        }
      : {}
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

  const axis = mode === "left" || mode === "right" ? "x" : "y";

  return (
    <div
      ref={containerRef}
      class={`absolute inset-0 pointer-events-none ${varClass}`}
      style={{ "--ratio": `${ratio * 100}%` }}
    >
      <div class="contents pointer-events-auto">
        <div class="absolute inset-[var(--inset)] transform-[var(--transform)] w-[var(--w,100%)] h-[var(--h,100%)] bg-white/80" />
        <button
          type="button"
          class="absolute inset-[var(--inset)] transform-[var(--transform)] m-[var(--margin)] btn btn-circle btn-md btn-soft"
          onMouseDown={(event) => {
            event.preventDefault();
            event.stopPropagation();
            if (containerRef.current) {
              handleMouseDown(containerRef.current, axis, event);
            }
          }}
          onTouchStart={(event) => {
            event.preventDefault();
            event.stopPropagation();
            if (containerRef.current) {
              handleTouchStart(containerRef.current, axis, event);
            }
          }}
        >
          <span class={`size-5 ${iconClass}`} />
        </button>
      </div>
    </div>
  );
}

export function VideoControls({
  video,
  config,
  compare,
  onUpdateConfig,
  onUpdateCompare,
  onFullscreen,
  onSelectFile,
}: {
  readonly video: HTMLVideoElement;
  readonly config: Anime4KConfig | null;
  readonly compare: CompareConfig;
  readonly onUpdateConfig: (config: Anime4KConfig | null) => void;
  readonly onUpdateCompare: (compare: CompareConfig) => void;
  readonly onFullscreen: () => void;
  readonly onSelectFile: () => void;
}) {
  const toast = useToast();

  const [lastConfig, setLastConfig] = useState<Anime4KConfig | null>(null);
  const displayConfig = config ?? lastConfig ?? DEFAULT_CONFIG;

  if (
    config &&
    (config.preset !== displayConfig.preset ||
      config.performance !== displayConfig.performance ||
      config.scale !== displayConfig.scale)
  ) {
    setLastConfig(config);
  }

  const updateConfig = useCallback(
    (newConfig: Anime4KConfig | null): void => {
      if (newConfig) {
        setLastConfig(newConfig);
      }
      onUpdateConfig(newConfig);

      // Show toast notification
      let message: string;
      if (newConfig === null) {
        message = "Anime4K disabled";
      } else {
        const presetLabel =
          PRESETS.find((p) => p.value === newConfig.preset)?.label ??
          newConfig.preset;
        const performanceLabel =
          PERFORMANCE_PRESETS.find((p) => p.value === newConfig.performance)
            ?.label ?? newConfig.performance;
        message = `Anime4K enabled (${newConfig.scale}x) · ${presetLabel} · ${performanceLabel}`;
      }

      toast.showToast(message);
    },
    [onUpdateConfig, toast]
  );

  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(true);
  const [currentTime, setCurrentTime] = useState<number | null>(null);
  const [seekingTime, setSeekingTime] = useState<number | null>(null);
  const [duration, setDuration] = useState<number | null>(null);
  const [volume, setVolume] = useState<number | null>(null);
  const [buffered, setBuffered] = useState<number>(0);

  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;

    const update = (): void => {
      setCurrentTime(nanToNull(video.currentTime));
      setDuration(nanToNull(video.duration));
      setVolume(nanToNull(video.volume));
      setIsMuted(video.muted);
      setIsPlaying(!video.paused);
      setBuffered(
        Array.from({ length: video.buffered.length }, (_, i) => [
          video.buffered.start(i),
          video.buffered.end(i),
        ]).find(([start, end]) => {
          return start <= video.currentTime && video.currentTime <= end;
        })?.[1] ?? 0
      );
      if (!video.seeking) {
        setSeekingTime(null);
      }
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

  const updateVolumeTo = useCallback(
    (volume: number): void => {
      const newVolume = Math.min(Math.max(Math.round(volume) / 100, 0), 1);
      video.volume = newVolume;
      video.muted = newVolume === 0;
      setVolume(newVolume);
      setIsMuted(video.muted);
    },
    [video]
  );

  const updateVolumeByOffset = useCallback(
    (offset: number): void => {
      if (!offset) {
        return;
      }

      updateVolumeTo(Math.round((video.volume ?? 0) * 100 + offset));
    },
    [video, updateVolumeTo]
  );

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
        ({ value }) => value === compare.mode
      );
      const offset = reverse ? -1 : 1;
      const nextIndex =
        (currentIndex + offset + COMPARE_MODES.length) % COMPARE_MODES.length;
      onUpdateCompare({
        ...compare,
        mode: COMPARE_MODES[nextIndex].value,
      });
    },
    [compare, onUpdateCompare]
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent): void => {
      const key =
        /^digit(\d)$/i.exec(event.code)?.[1] ?? event.key.toLowerCase();
      switch (key) {
        case "enter":
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
          toggleCompare(event.shiftKey);
          break;

        case "m":
          event.preventDefault();
          video.muted = !video.muted;
          setIsMuted(video.muted);
          break;

        case "o":
          event.preventDefault();
          onSelectFile();
          break;

        case "arrowleft":
        case "arrowright":
          event.preventDefault();
          if (video.readyState >= 2) {
            const offset =
              key === "arrowleft" ? -SEEK_OFFSET_DIRECT : SEEK_OFFSET_DIRECT;
            const newTime = (seekingTime ?? currentTime ?? 0) + offset;
            video.currentTime = newTime;
            setSeekingTime(newTime);
          }
          break;

        case "arrowup":
        case "arrowdown":
          event.preventDefault();
          updateVolumeByOffset(
            key === "arrowup" ? VOLUME_OFFSET : -VOLUME_OFFSET
          );
          break;

        case "0":
          if (
            event.ctrlKey &&
            !event.shiftKey &&
            !event.altKey &&
            !event.metaKey
          ) {
            event.preventDefault();
            updateConfig(null); // Disable Anime4K
          }
          break;

        case "1":
        case "2":
        case "3":
        case "4":
        case "5":
        case "6":
          if (
            event.ctrlKey &&
            !event.shiftKey &&
            !event.altKey &&
            !event.metaKey
          ) {
            event.preventDefault();
            const presetIndex = parseInt(key) - 1;
            if (presetIndex < PRESETS.length) {
              const preset = PRESETS[presetIndex].value;
              updateConfig({
                ...displayConfig,
                preset,
              });
            }
          } else if (
            event.shiftKey &&
            !event.ctrlKey &&
            !event.altKey &&
            !event.metaKey
          ) {
            event.preventDefault();
            const perfIndex = parseInt(key) - 1;
            if (perfIndex < PERFORMANCE_PRESETS.length) {
              const performance = PERFORMANCE_PRESETS[perfIndex].value;
              updateConfig({
                ...displayConfig,
                performance,
              });
            }
          }
          break;
      }
    },
    [
      togglePlayPause,
      toggleFullscreen,
      toggleCompare,
      updateConfig,
      displayConfig,
    ]
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
      onKeyDown={handleKeyDown}
    >
      <button
        class="absolute inset-0 focus:!outline-none opacity-0"
        aria-label="Play/Pause"
        onClick={togglePlayPause}
      />
      <div
        class="contents"
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.stopPropagation();
          }
        }}
      >
        {/* Compare Controller */}
        <CompareController value={compare} onChange={onUpdateCompare} />

        {/* Toast Notification */}
        <Toast
          class="alert-info alert-soft [--color-base-100:var(--color-base-200)]/80"
          message={toast.message}
          isVisible={toast.isVisible}
        />

        {/* Video Controller */}
        <div class="pointer-events-none relative bg-gradient-to-t from-[#000000f4] from-10% via-[#000000a0] via-50% opacity-0 group-hover:opacity-100 group-[[data-show-controls]]:opacity-100 has-[.dropdown:focus-within]:opacity-100 has-[:focus-visible]:opacity-100 transition-opacity duration-400 w-full h-40">
          <div class="pointer-events-auto absolute inset-[auto_0_0_0] flex flex-col justify-between p-4">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-4">
                {/* Play/pause button */}
                <button
                  type="button"
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
                <div
                  class="flex items-center group/volume not-sm:hidden"
                  onWheel={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    updateVolumeByOffset(
                      Math.sign(event.deltaY) * -VOLUME_OFFSET
                    );
                  }}
                >
                  <div class="w-0 flex items-center group-hover/volume:w-30 focus-within:w-30 rounded-full transition-all">
                    <input
                      type="range"
                      aria-label="Volume"
                      class="w-full range range-xs text-white [--range-bg:#555] [--range-thumb:white] [--range-fill:0]"
                      value={isMuted ? 0 : volume != null ? volume * 100 : 100}
                      min={0}
                      max={100}
                      step={1}
                      onKeyDown={(event) => {
                        const offset = {
                          ArrowUp: VOLUME_OFFSET,
                          ArrowDown: -VOLUME_OFFSET,
                          ArrowLeft: -VOLUME_OFFSET,
                          ArrowRight: VOLUME_OFFSET,
                        }[event.key];
                        if (offset == null) {
                          return;
                        }
                        event.preventDefault();
                        event.stopPropagation();
                        updateVolumeByOffset(offset);
                      }}
                      onInput={(event) => {
                        updateVolumeTo(
                          parseFloat((event.target as HTMLInputElement).value)
                        );
                      }}
                    />
                    <span class="w-2"></span>
                  </div>
                  <button
                    type="button"
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
                  type="button"
                  class="flex-none btn btn-circle btn-ghost btn-neutral btn-md"
                  aria-label="Toggle Fullscreen"
                  onClick={() => {
                    toggleFullscreen();
                  }}
                >
                  <span class="size-5 icon-[akar-icons--full-screen]"></span>
                </button>
                {/* Comparison Mode */}
                <div
                  class="dropdown dropdown-top dropdown-end"
                  onKeyDown={(event) => {
                    if (
                      event.key === "ArrowLeft" ||
                      event.key === "ArrowRight" ||
                      event.key === "ArrowUp" ||
                      event.key === "ArrowDown"
                    ) {
                      event.stopPropagation();
                    }
                  }}
                >
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
                    {COMPARE_MODES.map(({ value, label }) => (
                      <li key={value}>
                        <button
                          type="button"
                          class={compare.mode === value ? "menu-active" : ""}
                          aria-pressed={compare.mode === value}
                          onClick={() => setCompareMode(value)}
                        >
                          {label}
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
                {/* Menu */}
                <div
                  class="dropdown dropdown-top dropdown-end"
                  onKeyDown={(event) => {
                    if (
                      event.key === "ArrowLeft" ||
                      event.key === "ArrowRight" ||
                      event.key === "ArrowUp" ||
                      event.key === "ArrowDown"
                    ) {
                      event.stopPropagation();
                    }
                  }}
                >
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
                      <label class="label text-sm text-base-content">
                        <input
                          type="checkbox"
                          class="toggle"
                          checked={config !== null}
                          onChange={(event) => {
                            if (event.currentTarget.checked) {
                              updateConfig(DEFAULT_CONFIG);
                            } else {
                              updateConfig(null);
                            }
                          }}
                        />
                        <span>Enable Anime4K</span>
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
                          {PRESETS.map(({ value, label }) => (
                            <option
                              key={value}
                              value={value}
                              selected={config?.preset === value}
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
                          {PERFORMANCE_PRESETS.map(({ value, label }) => (
                            <option
                              key={value}
                              value={value}
                              selected={displayConfig.performance === value}
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
            <div class="px-2 pb-2">
              <Slider
                current={currentTime}
                seeking={seekingTime}
                buffered={buffered}
                duration={duration}
                onChange={(value) => {
                  video.currentTime = value;
                  setSeekingTime(value);
                }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
