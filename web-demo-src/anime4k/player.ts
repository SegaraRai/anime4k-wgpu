import {
  createPipelineExecutor,
  executePipeline,
  type ExecutablePipeline,
  type PipelineExecutor,
} from "./executor";
import {
  createPipelines,
  type Anime4KPreset,
  type Anime4KPerformancePreset,
} from "./presets";
import renderShader from "./render.wgsl?raw";

type PredefinedPipelines = Record<string, ExecutablePipeline>;

let gPredefinedPipelinesPromise: Promise<PredefinedPipelines> | undefined;

function fetchPredefinedPipelines(): Promise<PredefinedPipelines> {
  gPredefinedPipelinesPromise ??= (
    import(
      "./predefinedPipelines.json"
    ) as unknown as Promise<PredefinedPipelines>
  ).catch((error) => {
    console.error("❌ Failed to load predefined pipelines:", error);
    throw error;
  });
  return gPredefinedPipelinesPromise;
}

export async function preloadPredefinedPipelines(): Promise<void> {
  await fetchPredefinedPipelines();
}

export interface Anime4KConfig {
  readonly preset: Anime4KPreset;
  readonly performance: Anime4KPerformancePreset;
  readonly scale: number;
}

interface RenderingContextInit {
  readonly renderPipeline: GPURenderPipeline;
  readonly renderSampler: GPUSampler;
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
  readonly device: GPUDevice;
  readonly video: HTMLVideoElement;
  readonly canvasContext: GPUCanvasContext;
  readonly config: Anime4KConfig | null;
  readonly renderPipeline: GPURenderPipeline;
  readonly latestFrame: GPUTexture;
  readonly outputTexture: GPUTexture;
  readonly executor: PipelineExecutor;
  readonly renderBindGroup: GPUBindGroup;
}

async function createContext(
  device: GPUDevice,
  { renderPipeline, renderSampler }: RenderingContextInit,
  video: HTMLVideoElement,
  canvas: HTMLCanvasElement,
  canvasContext: GPUCanvasContext,
  config: Anime4KConfig | null
): Promise<RenderingContext> {
  // Update canvas dimensions
  const effectiveScale = Math.max(config?.scale ?? 1, 1);
  canvas.width = Math.floor(video.videoWidth * effectiveScale);
  canvas.height = Math.floor(video.videoHeight * effectiveScale);

  canvasContext.configure({
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    alphaMode: "opaque",
    colorSpace: "display-p3",
  });

  // Create a new texture for the latest frame (input to Anime4K)
  const latestFrame = device.createTexture({
    size: [video.videoWidth, video.videoHeight],
    format: "rgba8unorm",
    usage:
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.STORAGE_BINDING |
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });

  // Create a new pipeline executor
  const pipelineIds = config
    ? createPipelines(config.preset, config.performance, config.scale)
    : [];

  const predefinedPipelines = await fetchPredefinedPipelines();
  const executablePipelines = pipelineIds.map((id) => predefinedPipelines[id]);

  // Check if all pipelines are valid
  const invalidPipelines = pipelineIds.filter((id) => !predefinedPipelines[id]);
  if (invalidPipelines.length > 0) {
    console.error("❌ Found invalid pipelines:", invalidPipelines);
    throw new Error(`Invalid pipelines found: ${invalidPipelines.join(", ")}`);
  }

  let executor: PipelineExecutor;
  let outputTexture: GPUTexture;

  try {
    [executor, outputTexture] = await createPipelineExecutor(
      executablePipelines,
      device,
      latestFrame
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
    renderPipeline,
    latestFrame,
    outputTexture,
    executor,
    renderBindGroup,
  };
}

function shouldRecreateContext(
  context: RenderingContext,
  video: HTMLVideoElement,
  config: Anime4KConfig | null
): boolean {
  return (
    context.latestFrame.width !== video.videoWidth ||
    context.latestFrame.height !== video.videoHeight ||
    context.config?.preset !== config?.preset ||
    context.config?.performance !== config?.performance ||
    context.config?.scale !== config?.scale
  );
}

function cleanupContext(context?: RenderingContext | null): void {
  if (!context) {
    return;
  }

  context.latestFrame.destroy();
  context.outputTexture.destroy();
  context.executor.cleanup();
}

function render({
  device,
  video,
  latestFrame,
  executor,
  canvasContext,
  renderPipeline,
  renderBindGroup,
}: RenderingContext): void {
  // Copy the external texture (video frame) to the latestFrame texture
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

  const encoder = device.createCommandEncoder();

  // Execute the Anime4K pipeline
  executePipeline(executor, encoder);

  // Create a render pass to output the Anime4K result to the canvas
  const canvasTexture = canvasContext.getCurrentTexture();
  const renderPass = encoder.beginRenderPass({
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
  renderPass.setBindGroup(0, renderBindGroup);
  renderPass.draw(6);
  renderPass.end();

  // Submit the commands to the GPU
  device.queue.submit([encoder.finish()]);
}

export interface Anime4KController {
  ready: Promise<void>;
  cleanup: () => void;
  updateConfig: (config: Anime4KConfig | null) => void;
}

export function setupAnime4K(
  canvas: HTMLCanvasElement,
  video: HTMLVideoElement,
  config: Anime4KConfig | null = null
): Anime4KController {
  const abortController = new AbortController();
  const { signal } = abortController;

  let currentConfig: Anime4KConfig | null = config && { ...config };
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
    const createNewContext = async (): Promise<RenderingContext> => {
      if (signal.aborted) {
        throw new Error("Aborted");
      }

      try {
        return await createContext(
          device,
          contextInit,
          video,
          canvas,
          canvasContext,
          currentConfig
        );
      } catch (error) {
        console.error("❌ Failed to create rendering context:", error);
        contextPromise = null;

        throw error;
      }
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
          if (
            signal.aborted ||
            !shouldRecreateContext(context, video, currentConfig)
          ) {
            return context;
          }

          cleanupContext(context);
          contextPromise = createNewContext();
          return contextPromise;
        })
        .then((context) => {
          if (signal.aborted) {
            cleanupContext(context);
            return;
          }

          // Render the frame
          render(context);
        })
        .catch((error) => {
          if (signal.aborted) {
            return;
          }

          console.error("❌ Failed to render video frame:", error);
        })
        .finally(() => {
          if (signal.aborted) {
            return;
          }

          // Request the next frame
          timerId = video.requestVideoFrameCallback(onNewVideoFrame);
        });
    };

    const onNewVideoFrame = (
      _now: number,
      _metadata: VideoFrameCallbackMetadata
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
    updateConfig: (config): void => {
      currentConfig = config && { ...config };
      onConfigUpdate?.();
    },
  };
}
