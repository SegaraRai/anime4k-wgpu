import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { Anime4KConfig } from "../anime4k/player";
import {
  DEFAULT_COMPARE,
  DEFAULT_CONFIG,
  MAX_SCALE_FACTOR,
  MIN_SCALE_FACTOR,
  PERFORMANCE_PRESETS,
  PRESETS,
  type CompareConfig,
} from "./constants";
import { VideoPlayer } from "./VideoPlayer";

export function VideoPlayerPage() {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [enabled, setEnabled] = useState<boolean>(true);
  const [config, setConfig] = useState<Anime4KConfig>(DEFAULT_CONFIG);
  const [compare, setCompare] = useState<CompareConfig>(DEFAULT_COMPARE);
  const [isDragOver, setIsDragOver] = useState(false);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const currentURL = useRef<string | null>(null);
  const revokeCurrentURL = useCallback(() => {
    if (currentURL.current) {
      URL.revokeObjectURL(currentURL.current);
      currentURL.current = null;
    }
  }, []);
  const updateFile = useCallback(
    (blob: Blob) => {
      revokeCurrentURL();
      const url = URL.createObjectURL(blob);
      currentURL.current = url;
      setSelectedFile(url);
    },
    [revokeCurrentURL]
  );

  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;

    window.addEventListener("beforeunload", revokeCurrentURL, { signal });

    return () => {
      controller.abort();

      revokeCurrentURL();
    };
  }, [revokeCurrentURL]);

  const handleFileChange = useCallback((event: Event) => {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];

    if (file) {
      updateFile(file);
    }
  }, []);

  const handleFileClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((event: DragEvent) => {
    event.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((event: DragEvent) => {
    event.preventDefault();
    setIsDragOver(false);

    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      const file = files[0];
      if (file.type.startsWith("video/")) {
        updateFile(file);
      }
    }
  }, []);

  const handleUpdateConfig = useCallback((newConfig: Anime4KConfig | null) => {
    if (newConfig) {
      setEnabled(true);
      setConfig(newConfig);
    } else {
      setEnabled(false);
    }
  }, []);

  const onLoadedMetadata = useCallback((event: Event) => {
    // Update the scale factor based on the video dimensions and the current viewport size in physical pixels
    const video = event.target as HTMLVideoElement;
    const { videoWidth, videoHeight } = video;
    const viewportWidth = document.body.clientWidth * window.devicePixelRatio;
    const viewportHeight = document.body.clientHeight * window.devicePixelRatio;
    const scaleFactor = Math.max(
      Math.min(
        Math.ceil(
          Math.max(viewportWidth / videoWidth, viewportHeight / videoHeight)
        ),
        MAX_SCALE_FACTOR
      ),
      MIN_SCALE_FACTOR
    );
    setConfig((prevConfig) => ({
      ...prevConfig,
      scale: scaleFactor,
    }));
  }, []);

  return (
    <div class="bg-gradient-to-b from-base-200 to-base-400">
      {/* Header Section */}
      <div class="snap-start min-h-screen hero">
        <div class="hero-content text-center">
          <div class="max-w-4xl space-y-8">
            {/* Title and Description */}
            <div class="space-y-4">
              <h1 class="text-5xl font-bold py-2">Anime4K-wgpu Web Demo</h1>
              <p class="text-xl max-w-2xl mx-auto py-2">
                A WebGPU port of the renowned{" "}
                <a
                  href="https://github.com/bloc97/Anime4K"
                  class="link link-primary"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Anime4K
                </a>{" "}
                upscaling algorithm. Upload a video file and experience
                high-quality AI upscaling directly in your browser.
              </p>
              <p class="text-sm opacity-80">
                View the source code on{" "}
                <a
                  href="https://github.com/SegaraRai/anime4k-wgpu"
                  class="link link-primary"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  GitHub
                </a>
              </p>
            </div>

            {/* File Input */}
            <div class="space-y-6">
              <label
                class={`flex flex-col items-center space-y-4 p-8 border-2 border-dashed rounded-lg transition-all duration-200 ${
                  isDragOver
                    ? "border-primary bg-primary/10 scale-105"
                    : "border-transparent"
                }`}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
                onDrop={handleDrop}
              >
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="video/*"
                  onChange={handleFileChange}
                  hidden
                />
                <button
                  class="btn btn-primary btn-lg gap-3"
                  onClick={handleFileClick}
                >
                  <svg
                    class="w-6 h-6"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                    />
                  </svg>
                  Choose Video File
                </button>
                <p class="text-sm opacity-70 text-center">
                  {selectedFile
                    ? "Video file selected"
                    : "Drag and drop a video file here, or click to browse"}
                </p>
              </label>
            </div>

            {/* Anime4K Config Box */}
            <div class="card bg-base-100 shadow-xl max-w-4xl mx-auto">
              <div class="card-body">
                <h3 class="card-title text-lg mb-4">Configuration</h3>

                {/* Anime4K Enable/Disable */}
                <div class="form-control mb-6">
                  <label class="label cursor-pointer justify-start gap-3">
                    <input
                      type="checkbox"
                      class="toggle toggle-primary"
                      checked={enabled}
                      onChange={(event) => {
                        setEnabled(event.currentTarget.checked);
                      }}
                    />
                    <span class="label-text text-lg font-medium">
                      Enable Anime4K
                    </span>
                  </label>
                </div>

                {/* Anime4K Settings */}
                <div
                  class={`mb-6 ${!enabled ? "opacity-50 pointer-events-none" : ""}`}
                >
                  <h4 class="text-lg font-medium mb-3">Anime4K Settings</h4>
                  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    {/* Preset Selection */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">Preset</legend>
                      <select
                        class="w-full select select-bordered"
                        value={config.preset}
                        disabled={!enabled}
                        onChange={(e) => {
                          setConfig({
                            ...config,
                            preset: (e.target as HTMLSelectElement)
                              .value as any,
                          });
                        }}
                      >
                        {PRESETS.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </fieldset>

                    {/* Performance Selection */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">Performance</legend>
                      <select
                        class="w-full select select-bordered"
                        value={config.performance}
                        disabled={!enabled}
                        onInput={(e) => {
                          setConfig({
                            ...config,
                            performance: (e.target as HTMLSelectElement)
                              .value as any,
                          });
                        }}
                      >
                        {PERFORMANCE_PRESETS.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </fieldset>

                    {/* Scale Selection */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">
                        Scale Factor: {config.scale}x
                      </legend>
                      <input
                        type="range"
                        min={MIN_SCALE_FACTOR}
                        max={MAX_SCALE_FACTOR}
                        step="1"
                        value={config.scale}
                        disabled={!enabled}
                        class="w-full range range-primary"
                        onInput={(e) => {
                          setConfig({
                            ...config,
                            scale: parseFloat(
                              (e.target as HTMLInputElement).value
                            ),
                          });
                        }}
                      />
                      <div class="flex justify-between text-xs opacity-60 px-2 mt-1">
                        {Array.from(
                          { length: MAX_SCALE_FACTOR - MIN_SCALE_FACTOR + 1 },
                          (_, i) => (
                            <span key={i}>{i + MIN_SCALE_FACTOR}</span>
                          )
                        )}
                      </div>
                    </fieldset>
                  </div>
                </div>

                {/* Config Summary */}
                <div class="alert alert-info alert-soft">
                  <div class="text-sm">
                    {config ? (
                      <>
                        <strong>Current settings:</strong>{" "}
                        {PRESETS.find((p) => p.value === config.preset)?.label}{" "}
                        preset,{" "}
                        {
                          PERFORMANCE_PRESETS.find(
                            (p) => p.value === config.performance
                          )?.label
                        }{" "}
                        performance, {config.scale}x scale
                      </>
                    ) : (
                      <>
                        Anime4K is <strong>disabled.</strong>
                      </>
                    )}
                  </div>
                </div>

                {/* Keyboard Shortcuts */}
                <div class="mt-6">
                  <h4 class="text-lg font-medium mb-3">Keyboard Shortcuts</h4>
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {/* Video Controls */}
                    <div class="card bg-base-200/50 p-4">
                      <h5 class="font-medium mb-2">Video Controls</h5>
                      <div class="space-y-1 text-sm">
                        <div class="flex justify-between">
                          <kbd class="kbd kbd-xs">Space</kbd>
                          <span>Play/Pause</span>
                        </div>
                        <div class="flex justify-between">
                          <kbd class="kbd kbd-xs">Enter</kbd>
                          <span>Play/Pause</span>
                        </div>
                        <div class="flex justify-between">
                          <kbd class="kbd kbd-xs">F</kbd>
                          <span>Toggle Fullscreen</span>
                        </div>
                        <div class="flex justify-between">
                          <kbd class="kbd kbd-xs">C</kbd>
                          <span>Next Compare Mode</span>
                        </div>
                        <div class="flex justify-between">
                          <span class="space-x-1">
                            <kbd class="kbd kbd-xs">Shift</kbd>
                            <kbd class="kbd kbd-xs">C</kbd>
                          </span>
                          <span>Previous Compare Mode</span>
                        </div>
                      </div>
                    </div>

                    {/* Anime4K Controls */}
                    <div class="card bg-base-200/50 p-4">
                      <h5 class="font-medium mb-2">Anime4K Controls</h5>
                      <div class="space-y-1 text-sm">
                        <div class="flex justify-between">
                          <span class="space-x-1">
                            <kbd class="kbd kbd-xs">Ctrl</kbd>
                            <kbd class="kbd kbd-xs">0</kbd>
                          </span>
                          <span>Disable Anime4K</span>
                        </div>
                        <div class="flex justify-between">
                          <span class="space-x-1">
                            <kbd class="kbd kbd-xs">Ctrl</kbd>
                            <kbd class="kbd kbd-xs">1-6</kbd>
                          </span>
                          <span>Set Preset (A, B, C, AA, BB, CA)</span>
                        </div>
                        <div class="flex justify-between">
                          <span class="space-x-1">
                            <kbd class="kbd kbd-xs">Shift</kbd>
                            <kbd class="kbd kbd-xs">1-5</kbd>
                          </span>
                          <span>Set Performance (Light to Extreme)</span>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="text-xs opacity-70 mt-2 text-center">
                    Keyboard shortcuts work when the video player is focused
                  </div>
                </div>

                {/* Scroll indicator */}
                {selectedFile && (
                  <div class="text-center animate-bounce mt-6">
                    <svg
                      class="w-6 h-6 mx-auto opacity-60"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M19 14l-7 7m0 0l-7-7m7 7V3"
                      />
                    </svg>
                    <p class="text-sm opacity-70 mt-2">
                      Scroll down to view video
                    </p>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Video Player Section */}
      {selectedFile && (
        <div class="snap-start h-screen">
          <VideoPlayer
            src={selectedFile}
            config={enabled ? config : null}
            compare={compare}
            onUpdateConfig={handleUpdateConfig}
            onUpdateCompare={setCompare}
            onLoadedMetadata={onLoadedMetadata}
          />
        </div>
      )}
    </div>
  );
}
