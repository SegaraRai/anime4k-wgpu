import { useRef, useState } from "preact/hooks";
import type { Anime4KConfig } from "../anime4k/player";
import {
  COMPARE_MODES,
  DEFAULT_CONFIG,
  DEFAULT_COMPARE,
  MAX_SCALE_FACTOR,
  MIN_SCALE_FACTOR,
  PERFORMANCE_PRESETS,
  PRESETS,
  type CompareConfig,
} from "./constants";
import { VideoPlayer } from "./VideoPlayer";

export function VideoPlayerPage() {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [config, setConfig] = useState<Anime4KConfig | null>(DEFAULT_CONFIG);
  const [compare, setCompare] = useState<CompareConfig>(DEFAULT_COMPARE);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (event: Event) => {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];

    if (file) {
      const url = URL.createObjectURL(file);
      setSelectedFile(url);
    }
  };

  const handleFileClick = () => {
    fileInputRef.current?.click();
  };

  return (
    <div class="bg-gradient-to-b from-base-200 to-base-400">
      {/* Header Section */}
      <div class="snap-start min-h-screen hero">
        <div class="hero-content text-center">
          <div class="max-w-4xl space-y-8">
            {/* Title and Description */}
            <div class="space-y-4">
              <h1 class="text-5xl font-bold">Anime4K-wgpu Web Demo</h1>
              <p class="text-xl max-w-2xl mx-auto">
                Real-time anime upscaling powered by WebGPU. Upload a video file
                and experience high-quality AI upscaling directly in your
                browser.
              </p>
            </div>

            {/* File Input */}
            <div class="space-y-6">
              <div class="flex flex-col items-center space-y-4">
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="video/*"
                  onChange={handleFileChange}
                  class="hidden"
                />
                <button
                  onClick={handleFileClick}
                  class="btn btn-primary btn-lg gap-3"
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
                {selectedFile && (
                  <p class="text-sm opacity-70">Video file selected</p>
                )}
              </div>
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
                      checked={config !== null}
                      onChange={(event) => {
                        if (event.currentTarget.checked) {
                          setConfig(DEFAULT_CONFIG);
                        } else {
                          setConfig(null);
                        }
                      }}
                    />
                    <span class="label-text text-lg font-medium">
                      Enable Anime4K
                    </span>
                  </label>
                </div>

                {/* Anime4K Settings */}
                <div
                  class={`mb-6 ${!config ? "opacity-50 pointer-events-none" : ""}`}
                >
                  <h4 class="text-lg font-medium mb-3">Anime4K Settings</h4>
                  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    {/* Preset Selection */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">Preset</legend>
                      <select
                        class="w-full select select-bordered"
                        value={config?.preset ?? ""}
                        disabled={!config}
                        onChange={(e) => {
                          if (!config) return;
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
                        value={config?.performance ?? ""}
                        disabled={!config}
                        onInput={(e) => {
                          if (!config) return;
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
                        Scale Factor: {config?.scale ?? MIN_SCALE_FACTOR}x
                      </legend>
                      <input
                        type="range"
                        min={MIN_SCALE_FACTOR}
                        max={MAX_SCALE_FACTOR}
                        step="1"
                        value={config?.scale ?? MIN_SCALE_FACTOR}
                        disabled={!config}
                        class="w-full range range-primary"
                        onInput={(e) => {
                          if (!config) return;
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

                {/* Compare Settings */}
                <div class="mb-6">
                  <h4 class="text-lg font-medium mb-3">Comparison Mode</h4>
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {/* Compare Mode Selection */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">Mode</legend>
                      <select
                        class="w-full select select-bordered"
                        value={compare.mode}
                        onInput={(e) =>
                          setCompare({
                            ...compare,
                            mode: (e.target as HTMLSelectElement)
                              .value as CompareConfig["mode"],
                          })
                        }
                      >
                        {COMPARE_MODES.map(({ value, label }) => (
                          <option key={value} value={value}>
                            {label}
                          </option>
                        ))}
                      </select>
                    </fieldset>

                    {/* Compare Ratio */}
                    <fieldset class="fieldset">
                      <legend class="fieldset-legend">
                        Ratio: {Math.round(compare.ratio * 100)}%
                      </legend>
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.01"
                        value={compare.ratio}
                        class="w-full range range-primary"
                        onInput={(e) =>
                          setCompare({
                            ...compare,
                            ratio: parseFloat(
                              (e.target as HTMLInputElement).value
                            ),
                          })
                        }
                      />
                      <div class="flex justify-between text-xs opacity-60 px-2 mt-1">
                        <span>0%</span>
                        <span>100%</span>
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
            config={config}
            compare={compare}
            onUpdateConfig={setConfig}
            onUpdateCompare={setCompare}
          />
        </div>
      )}
    </div>
  );
}
