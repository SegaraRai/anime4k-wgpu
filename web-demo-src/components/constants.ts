import type { Anime4KConfig, ColorCorrectionConfig } from "../anime4k/player";
import type {
  Anime4KPerformancePreset,
  Anime4KPreset,
} from "../anime4k/presets";

export interface CompareConfig {
  readonly mode: "none" | "onyx" | "left" | "right" | "top" | "bottom";
  readonly ratio: number;
}

export const MIN_SCALE_FACTOR = 1;
export const MAX_SCALE_FACTOR = 8;

export const PRESETS: readonly {
  readonly value: Anime4KPreset;
  readonly label: string;
}[] = [
  { value: "a", label: "A (Restore → Upscale)" },
  { value: "b", label: "B (Restore Soft → Upscale)" },
  { value: "c", label: "C (Upscale Denoise)" },
  { value: "aa", label: "AA (Restore → Upscale → Restore)" },
  { value: "bb", label: "BB (Restore Soft → Upscale → Restore Soft)" },
  { value: "ca", label: "CA (Upscale Denoise → Restore)" },
];

export const PERFORMANCE_PRESETS: readonly {
  readonly value: Anime4KPerformancePreset;
  readonly label: string;
}[] = [
  { value: "light", label: "Light (Fast, Low Quality)" },
  { value: "medium", label: "Medium (Balanced)" },
  { value: "high", label: "High (Slow, High Quality)" },
  { value: "ultra", label: "Ultra (Very Slow, Very High Quality)" },
  { value: "extreme", label: "Extreme (Insane Quality)" },
];

export const DEFAULT_CONFIG: Anime4KConfig = {
  preset: "a",
  performance: "medium",
  scale: 2,
};

export const DEFAULT_COLOR_CORRECTION_CONFIG: ColorCorrectionConfig = {
  enabled: false,
  sourceYUV: "bt709",
  targetYUV: "bt709",
  sourceRange: "limited",
  targetRange: "full",
  sourceGamma: "rec709",
  targetGamma: "srgb",
};

export const YUV_STANDARDS = [
  { value: "bt601" as const, label: "BT.601 (SDTV)" },
  { value: "bt709" as const, label: "BT.709 (HDTV)" },
  { value: "bt2020" as const, label: "BT.2020 (UHDTV)" },
];

export const RANGE_TYPES = [
  { value: "limited" as const, label: "Limited (TV/Studio Range)" },
  { value: "full" as const, label: "Full (PC Range)" },
];

export const GAMMA_TYPES = [
  { value: "linear" as const, label: "Linear" },
  { value: "srgb" as const, label: "sRGB" },
  { value: "rec709" as const, label: "Rec.709" },
  { value: "gamma2.2" as const, label: "Gamma 2.2" },
];

export const COMPARE_MODES: readonly {
  value: CompareConfig["mode"];
  label: string;
}[] = [
  { value: "left", label: "Split Left/Right" },
  { value: "right", label: "Split Right/Left" },
  { value: "top", label: "Split Top/Bottom" },
  { value: "bottom", label: "Split Bottom/Top" },
  { value: "onyx", label: "Blend Overlay" },
  { value: "none", label: "No Comparison" },
];

export const DEFAULT_COMPARE: CompareConfig = {
  mode: "left",
  ratio: 0.5,
};
