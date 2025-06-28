export type Anime4KPreset = "a" | "aa" | "b" | "bb" | "c" | "ca";

export type Anime4KPerformancePreset =
  | "light"
  | "medium"
  | "high"
  | "ultra"
  | "extreme";

/**
 * Returns the restore CNN pipeline name for the initial pass
 */
function getInitialRestoreName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "RESTORE_CNN_S",
      medium: "RESTORE_CNN_M",
      high: "RESTORE_CNN_L",
      ultra: "RESTORE_CNN_VL",
      extreme: "RESTORE_CNN_UL",
    } as const
  )[preset];
}

/**
 * Returns the soft restore CNN pipeline name for the initial pass
 */
function getInitialRestoreSoftName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "RESTORE_SOFT_CNN_S",
      medium: "RESTORE_SOFT_CNN_M",
      high: "RESTORE_SOFT_CNN_L",
      ultra: "RESTORE_SOFT_CNN_VL",
      extreme: "RESTORE_SOFT_CNN_UL",
    } as const
  )[preset];
}

/**
 * Returns the upscale+denoise CNN pipeline name for the initial 2x upscaling pass
 */
function getInitialUpscaleDenoiseName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "UPSCALE_DENOISE_CNN_X2_S",
      medium: "UPSCALE_DENOISE_CNN_X2_M",
      high: "UPSCALE_DENOISE_CNN_X2_L",
      ultra: "UPSCALE_DENOISE_CNN_X2_VL",
      extreme: "UPSCALE_DENOISE_CNN_X2_UL",
    } as const
  )[preset];
}

/**
 * Returns the restore CNN pipeline name for subsequent passes (typically smaller models)
 */
function getSubsequentRestoreName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "RESTORE_CNN_S",
      medium: "RESTORE_CNN_S",
      high: "RESTORE_CNN_M",
      ultra: "RESTORE_CNN_L",
      extreme: "RESTORE_CNN_L",
    } as const
  )[preset];
}

/**
 * Returns the soft restore CNN pipeline name for subsequent passes
 */
function getSubsequentRestoreSoftName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "RESTORE_SOFT_CNN_S",
      medium: "RESTORE_SOFT_CNN_S",
      high: "RESTORE_SOFT_CNN_M",
      ultra: "RESTORE_SOFT_CNN_L",
      extreme: "RESTORE_SOFT_CNN_L",
    } as const
  )[preset];
}

/**
 * Returns the upscale CNN pipeline name for the initial 2x upscaling pass
 */
function getInitialUpscaleName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "UPSCALE_CNN_X2_S",
      medium: "UPSCALE_CNN_X2_M",
      high: "UPSCALE_CNN_X2_L",
      ultra: "UPSCALE_CNN_X2_VL",
      extreme: "UPSCALE_CNN_X2_UL",
    } as const
  )[preset];
}

/**
 * Returns the upscale CNN pipeline name for subsequent 2x upscaling passes
 */
function getSubsequentUpscaleName(preset: Anime4KPerformancePreset) {
  return (
    {
      light: "UPSCALE_CNN_X2_S",
      medium: "UPSCALE_CNN_X2_S",
      high: "UPSCALE_CNN_X2_M",
      ultra: "UPSCALE_CNN_X2_L",
      extreme: "UPSCALE_CNN_X2_L",
    } as const
  )[preset];
}

export type PipelineName =
  | "CLAMP_HIGHLIGHTS"
  | ReturnType<
      | typeof getInitialRestoreName
      | typeof getInitialRestoreSoftName
      | typeof getInitialUpscaleDenoiseName
      | typeof getSubsequentRestoreName
      | typeof getSubsequentRestoreSoftName
      | typeof getInitialUpscaleName
      | typeof getSubsequentUpscaleName
    >;

/**
 * Creates a pipeline of shader names based on the preset and performance settings
 * @param preset - The Anime4K preset mode
 * @param performancePreset - The performance level preset
 * @param targetScaleFactor - The target scale factor (e.g., 2.0 for 2x, 4.0 for 4x)
 * @returns Array of shader pipeline names
 */
export function createPipelines(
  preset: Anime4KPreset,
  performancePreset: Anime4KPerformancePreset,
  targetScaleFactor: number
): PipelineName[] {
  let base: PipelineName[];

  switch (preset) {
    case "a":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialRestoreName(performancePreset),
        getInitialUpscaleName(performancePreset),
      ];
      break;
    case "b":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialRestoreSoftName(performancePreset),
        getInitialUpscaleName(performancePreset),
      ];
      break;
    case "c":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialUpscaleDenoiseName(performancePreset),
      ];
      break;
    case "aa":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialRestoreName(performancePreset),
        getInitialUpscaleName(performancePreset),
        getSubsequentRestoreName(performancePreset),
      ];
      break;
    case "bb":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialRestoreSoftName(performancePreset),
        getInitialUpscaleName(performancePreset),
        getSubsequentRestoreSoftName(performancePreset),
      ];
      break;
    case "ca":
      base = [
        "CLAMP_HIGHLIGHTS",
        getInitialUpscaleDenoiseName(performancePreset),
        getSubsequentRestoreName(performancePreset),
      ];
      break;
    default:
      return [];
  }

  let currentScaleFactor = 2.0;
  while (currentScaleFactor < targetScaleFactor) {
    base.push(getSubsequentUpscaleName(performancePreset));
    currentScaleFactor *= 2.0;
  }

  return base;
}
