import {
  createPipelineExecutor,
  executePipeline,
  type PipelineExecutor,
} from "./executor";
import {
  createPipelines,
  type Anime4KPreset,
  type Anime4KPerformancePreset,
} from "./presets";
import predefinedPipelines from "./predefinedPipelines.json" with { type: "json" };
import renderShader from "./render.wgsl?raw";
import colorCorrectionShader from "./color_correction.wgsl?raw";

export interface Anime4KConfig {
  readonly preset: Anime4KPreset;
  readonly performance: Anime4KPerformancePreset;
  readonly scale: number;
}

export interface ColorCorrectionConfig {
  readonly enabled: boolean;
  readonly sourceYUV: "bt601" | "bt709" | "bt2020";
  readonly targetYUV: "bt601" | "bt709" | "bt2020";
  readonly sourceRange: "limited" | "full";
  readonly targetRange: "limited" | "full";
  readonly sourceGamma: "srgb" | "linear" | "rec709" | "gamma2.2";
  readonly targetGamma: "srgb" | "linear" | "rec709" | "gamma2.2";
}

interface RenderingContextInit {
  renderPipeline: GPURenderPipeline;
  renderSampler: GPUSampler;
}

function createContextInit(device: GPUDevice): RenderingContextInit {
  // Create a render pipeline for copying float texture to canvas
  const renderShaderModule = device.createShaderModule({
    code: renderShader,
  });

  const renderPipeline = device.createRenderPipeline({
    layout: "auto",
    vertex: {
      module: renderShaderModule,
      entryPoint: "vs_main",
    },
    fragment: {
      module: renderShaderModule,
      entryPoint: "fs_main",
      targets: [
        {
          format: navigator.gpu.getPreferredCanvasFormat(),
        },
      ],
    },
    primitive: {
      topology: "triangle-list",
    },
  });

  const renderSampler = device.createSampler({
    magFilter: "linear",
    minFilter: "linear",
  });

  return {
    renderPipeline,
    renderSampler,
  };
}

interface RenderingContext {
  device: GPUDevice;
  video: HTMLVideoElement;
  canvasContext: GPUCanvasContext;
  config: Anime4KConfig | null;
  colorCorrectionConfig: ColorCorrectionConfig | null;
  renderPipeline: GPURenderPipeline;
  latestFrame: GPUTexture;
  colorCorrectedTexture: GPUTexture;
  outputTexture: GPUTexture;
  executor: PipelineExecutor;
  renderBindGroup: GPUBindGroup;
  colorCorrectionPipeline: GPUComputePipeline | null;
  colorCorrectionBindGroup: GPUBindGroup | null;
  colorCorrectionUniformBuffer: GPUBuffer | null;
}

function createColorCorrectionPipeline(
  device: GPUDevice,
  inputTexture: GPUTexture,
  outputTexture: GPUTexture
): [GPUComputePipeline, GPUBindGroup, GPUBuffer] {
  const shaderModule = device.createShaderModule({
    code: colorCorrectionShader,
  });

  const bindGroupLayout = device.createBindGroupLayout({
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.COMPUTE,
        texture: {
          sampleType: "float",
          viewDimension: "2d",
        },
      },
      {
        binding: 1,
        visibility: GPUShaderStage.COMPUTE,
        storageTexture: {
          access: "write-only",
          format: outputTexture.format,
          viewDimension: "2d",
        },
      },
      {
        binding: 2,
        visibility: GPUShaderStage.COMPUTE,
        buffer: {
          type: "uniform",
        },
      },
    ],
  });

  const pipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [bindGroupLayout],
  });

  const computePipeline = device.createComputePipeline({
    layout: pipelineLayout,
    compute: {
      module: shaderModule,
      entryPoint: "main",
    },
  });

  // Create uniform buffer for color correction parameters
  const uniformBuffer = device.createBuffer({
    size: 32, // 8 u32 values * 4 bytes
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const bindGroup = device.createBindGroup({
    layout: bindGroupLayout,
    entries: [
      {
        binding: 0,
        resource: inputTexture.createView(),
      },
      {
        binding: 1,
        resource: outputTexture.createView(),
      },
      {
        binding: 2,
        resource: {
          buffer: uniformBuffer,
        },
      },
    ],
  });

  return [computePipeline, bindGroup, uniformBuffer];
}

function updateColorCorrectionUniforms(
  device: GPUDevice,
  buffer: GPUBuffer,
  config: ColorCorrectionConfig | null
): void {
  const uniforms = new Uint32Array(8);

  if (config?.enabled) {
    // Map string values to numeric indices
    const matrixMap = { bt601: 0, bt709: 1, bt2020: 2 };
    const rangeMap = { limited: 0, full: 1 };
    const transferMap = { linear: 0, srgb: 1, rec709: 2, "gamma2.2": 3 };

    uniforms[0] = matrixMap[config.sourceYUV] ?? 1; // source_matrix
    uniforms[1] = matrixMap[config.targetYUV] ?? 1; // target_matrix
    uniforms[2] = rangeMap[config.sourceRange] ?? 0; // source_range
    uniforms[3] = rangeMap[config.targetRange] ?? 1; // target_range
    uniforms[4] = transferMap[config.sourceGamma] ?? 1; // source_transfer
    uniforms[5] = transferMap[config.targetGamma] ?? 1; // target_transfer
    uniforms[6] = 1; // enable_correction
    uniforms[7] = 0; // reserved
  } else {
    // Passthrough mode
    uniforms.fill(0);
  }

  device.queue.writeBuffer(buffer, 0, uniforms);
}

async function createContext(
  device: GPUDevice,
  { renderPipeline, renderSampler }: RenderingContextInit,
  video: HTMLVideoElement,
  canvas: HTMLCanvasElement,
  canvasContext: GPUCanvasContext,
  config: Anime4KConfig | null,
  colorCorrectionConfig: ColorCorrectionConfig | null
): Promise<RenderingContext> {
  // Update canvas dimensions
  const effectiveScale = Math.max(config?.scale ?? 1, 1);
  canvas.width = Math.floor(video.videoWidth * effectiveScale);
  canvas.height = Math.floor(video.videoHeight * effectiveScale);

  canvasContext.configure({
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    // Remove explicit colorSpace to let browser handle it naturally
    alphaMode: "opaque",
    colorSpace: "display-p3",
  });

  // Create a new texture for the latest frame (input to color correction)
  const latestFrame = device.createTexture({
    size: [video.videoWidth, video.videoHeight],
    format: "rgba8unorm", // Keep unorm for compatibility
    usage:
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.STORAGE_BINDING |
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });

  // Create a texture for color corrected output (input to Anime4K)
  const colorCorrectedTexture = device.createTexture({
    size: [video.videoWidth, video.videoHeight],
    format: "rgba32float", // Use float for better precision in Anime4K pipeline
    usage:
      GPUTextureUsage.STORAGE_BINDING |
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });

  // Create a new pipeline executor
  const pipelineIds = config
    ? createPipelines(config.preset, config.performance, config.scale)
    : [];

  const executablePipelines = pipelineIds.map(
    (id) => (predefinedPipelines as any)[id]
  );

  // Check if all pipelines are valid
  const invalidPipelines = executablePipelines.filter((pipeline) => !pipeline);
  if (invalidPipelines.length > 0) {
    console.error("❌ Found invalid pipelines:", invalidPipelines);
    throw new Error(`Invalid pipelines found: ${invalidPipelines.join(", ")}`);
  }

  let executor: PipelineExecutor;
  let outputTexture: GPUTexture;

  // Create color correction pipeline
  let colorCorrectionPipeline: GPUComputePipeline | null = null;
  let colorCorrectionBindGroup: GPUBindGroup | null = null;
  let colorCorrectionUniformBuffer: GPUBuffer | null = null;

  if (colorCorrectionConfig?.enabled) {
    [colorCorrectionPipeline, colorCorrectionBindGroup, colorCorrectionUniformBuffer] = 
      createColorCorrectionPipeline(device, latestFrame, colorCorrectedTexture);
    updateColorCorrectionUniforms(device, colorCorrectionUniformBuffer, colorCorrectionConfig);
  }

  // Use color corrected texture as input to Anime4K if color correction is enabled
  const anime4kInputTexture = colorCorrectionConfig?.enabled ? colorCorrectedTexture : latestFrame;

  try {
    [executor, outputTexture] = await createPipelineExecutor(
      executablePipelines,
      device,
      anime4kInputTexture
    );
  } catch (error) {
    console.error("❌ Failed to create pipeline executor:", error);
    throw error;
  }

  // Create render bind group for the output texture
  const outputTextureView = outputTexture.createView();

  const renderBindGroup = device.createBindGroup({
    layout: renderPipeline.getBindGroupLayout(0),
    entries: [
      {
        binding: 0,
        resource: outputTextureView,
      },
      {
        binding: 1,
        resource: renderSampler,
      },
    ],
  });

  return {
    device,
    video,
    canvasContext,
    config,
    colorCorrectionConfig,
    renderPipeline,
    latestFrame,
    colorCorrectedTexture,
    outputTexture,
    executor,
    renderBindGroup,
    colorCorrectionPipeline,
    colorCorrectionBindGroup,
    colorCorrectionUniformBuffer,
  };
}

function shouldRecreateContext(
  context: RenderingContext,
  video: HTMLVideoElement,
  config: Anime4KConfig | null,
  colorCorrectionConfig: ColorCorrectionConfig | null
): boolean {
  return (
    context.latestFrame.width !== video.videoWidth ||
    context.latestFrame.height !== video.videoHeight ||
    context.config?.preset !== config?.preset ||
    context.config?.performance !== config?.performance ||
    context.config?.scale !== config?.scale ||
    context.colorCorrectionConfig?.enabled !== colorCorrectionConfig?.enabled ||
    context.colorCorrectionConfig?.sourceYUV !== colorCorrectionConfig?.sourceYUV ||
    context.colorCorrectionConfig?.targetYUV !== colorCorrectionConfig?.targetYUV ||
    context.colorCorrectionConfig?.sourceRange !== colorCorrectionConfig?.sourceRange ||
    context.colorCorrectionConfig?.targetRange !== colorCorrectionConfig?.targetRange ||
    context.colorCorrectionConfig?.sourceGamma !== colorCorrectionConfig?.sourceGamma ||
    context.colorCorrectionConfig?.targetGamma !== colorCorrectionConfig?.targetGamma
  );
}

function cleanupContext(context?: RenderingContext | null): void {
  if (!context) {
    return;
  }

  context.latestFrame.destroy();
  context.colorCorrectedTexture.destroy();
  context.outputTexture.destroy();
  context.executor.cleanup();
  context.colorCorrectionUniformBuffer?.destroy();
}

function render({
  device,
  video,
  latestFrame,
  colorCorrectedTexture,
  executor,
  canvasContext,
  renderPipeline,
  renderBindGroup,
  colorCorrectionPipeline,
  colorCorrectionBindGroup,
  colorCorrectionUniformBuffer,
  colorCorrectionConfig,
}: RenderingContext): void {
  // Copy the external texture to the latestFrame
  // Preserve original color space and range
  device.queue.copyExternalImageToTexture(
    {
      source: video,
    },
    {
      texture: latestFrame,
      premultipliedAlpha: false,
    },
    [video.videoWidth, video.videoHeight]
  );

  // Use a single command encoder for color correction and Anime4K pipeline
  const mainEncoder = device.createCommandEncoder();

  // Execute color correction if enabled
  if (colorCorrectionConfig?.enabled && colorCorrectionPipeline && colorCorrectionBindGroup) {
    const colorCorrectionPass = mainEncoder.beginComputePass();
    colorCorrectionPass.setPipeline(colorCorrectionPipeline);
    colorCorrectionPass.setBindGroup(0, colorCorrectionBindGroup);
    
    const workgroupsX = Math.ceil(video.videoWidth / 8);
    const workgroupsY = Math.ceil(video.videoHeight / 8);
    colorCorrectionPass.dispatchWorkgroups(workgroupsX, workgroupsY);
    colorCorrectionPass.end();
  }

  // Execute Anime4K pipeline
  executePipeline(executor, mainEncoder);
  device.queue.submit([mainEncoder.finish()]);

  const commandEncoder = device.createCommandEncoder();

  // The Anime4K pipeline was already executed during video frame processing
  // Now just render the output texture to canvas
  const canvasTexture = canvasContext.getCurrentTexture();

  const renderPass = commandEncoder.beginRenderPass({
    colorAttachments: [
      {
        view: canvasTexture.createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 },
        loadOp: "clear",
        storeOp: "store",
      },
    ],
  });

  renderPass.setPipeline(renderPipeline);

  // Render the Anime4K output texture
  renderPass.setBindGroup(0, renderBindGroup);

  renderPass.draw(6);
  renderPass.end();

  const commands = commandEncoder.finish();
  device.queue.submit([commands]);
}

export interface Anime4KController {
  ready: Promise<void>;
  cleanup: () => void;
  updateConfig: (config: Anime4KConfig | null, colorCorrectionConfig?: ColorCorrectionConfig | null) => void;
}

export function setupAnime4K(
  canvas: HTMLCanvasElement,
  video: HTMLVideoElement,
  config: Anime4KConfig | null = null,
  colorCorrectionConfig: ColorCorrectionConfig | null = null
): Anime4KController {
  const abortController = new AbortController();
  const { signal } = abortController;

  let currentConfig: Anime4KConfig | null = config && { ...config };
  let currentColorCorrectionConfig: ColorCorrectionConfig | null = colorCorrectionConfig && { ...colorCorrectionConfig };
  let contextPromise: Promise<RenderingContext> | null = null;
  let timerId: number | null = null;
  let onConfigUpdate: (() => void) | null = null;

  const init = async () => {
    // Create a WebGPU adapter and device
    const adapter = await navigator.gpu?.requestAdapter();
    if (!adapter) {
      throw new Error("WebGPU adapter not available");
    }

    const device = await adapter.requestDevice({
      requiredFeatures: ["float32-filterable"],
    });

    device.addEventListener(
      "uncapturederror",
      (event) => {
        console.error("🚨 WebGPU uncaptured error:", event.error);
      },
      { signal }
    );

    // Configure the canvas for WebGPU
    const canvasContext = canvas.getContext("webgpu");
    if (!canvasContext) {
      throw new Error("WebGPU context not available on canvas");
    }

    const contextInit = createContextInit(device);
    const createNewContext = (): Promise<RenderingContext> => {
      return createContext(
        device,
        contextInit,
        video,
        canvas,
        canvasContext,
        currentConfig,
        currentColorCorrectionConfig
      ).catch((error): never => {
        console.error("❌ Failed to create rendering context:", error);
        contextPromise = null;

        throw error;
      });
    };

    const ensureContextAndRender = (): void => {
      if (timerId != null) {
        video.cancelVideoFrameCallback(timerId);
        timerId = null;
      }

      if (signal.aborted) {
        return;
      }

      if (
        video.readyState < 2 ||
        !isFinite(video.videoWidth) ||
        !isFinite(video.videoHeight)
      ) {
        console.warn(
          "⚠️ Video not ready or dimensions are invalid, waiting for next frame"
        );

        timerId = video.requestVideoFrameCallback(onNewVideoFrame);
        return;
      }

      contextPromise ??= createNewContext();
      contextPromise
        .then((context) => {
          if (!shouldRecreateContext(context, video, currentConfig, currentColorCorrectionConfig)) {
            return context;
          }

          cleanupContext(context);
          contextPromise = createNewContext();
          return contextPromise;
        })
        .then((context) => {
          // Render the frame
          render(context);
        })
        .catch((error) => {
          console.error("❌ Failed to render video frame:", error);
        })
        .finally(() => {
          // Request the next frame
          timerId = video.requestVideoFrameCallback(onNewVideoFrame);
        });
    };

    const onNewVideoFrame = (
      now: number,
      metadata: VideoFrameCallbackMetadata
    ): void => {
      timerId = null;
      ensureContextAndRender();
    };

    onConfigUpdate = (): void => {
      ensureContextAndRender();
    };

    ensureContextAndRender();

    video.addEventListener("loadedmetadata", ensureContextAndRender, {
      signal,
    });
    video.addEventListener("loadeddata", ensureContextAndRender, { signal });
    video.addEventListener("seeked", ensureContextAndRender, { signal });
  };

  const cleanup = (): void => {
    abortController.abort();

    if (timerId != null) {
      video.cancelVideoFrameCallback(timerId);
      timerId = null;
    }

    contextPromise?.then((context): void => {
      cleanupContext(context);
    });
    contextPromise = null;
  };

  return {
    ready: init(),
    cleanup,
    updateConfig: (config, colorCorrectionConfig): void => {
      currentConfig = config && { ...config };
      currentColorCorrectionConfig = colorCorrectionConfig ? { ...colorCorrectionConfig } : null;
      onConfigUpdate?.();
    },
  };
}
