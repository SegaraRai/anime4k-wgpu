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

export interface Anime4KConfig {
  readonly preset: Anime4KPreset;
  readonly performance: Anime4KPerformancePreset;
  readonly scale: number;
}

interface RenderingContextInit {
  renderPipeline: GPURenderPipeline;
  renderSampler: GPUSampler;
}

function createContextInit(device: GPUDevice): RenderingContextInit {
  // Create a render pipeline for copying float texture to canvas
  const renderShaderModule = device.createShaderModule({
    code: `
      struct VertexOutput {
        @builtin(position) position: vec4<f32>,
        @location(0) texCoord: vec2<f32>,
      }

      @vertex
      fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
        var pos = array<vec2<f32>, 6>(
          vec2<f32>(-1.0, -1.0),
          vec2<f32>( 1.0, -1.0),
          vec2<f32>(-1.0,  1.0),
          vec2<f32>( 1.0, -1.0),
          vec2<f32>( 1.0,  1.0),
          vec2<f32>(-1.0,  1.0)
        );

        var texCoord = array<vec2<f32>, 6>(
          vec2<f32>(0.0, 1.0),
          vec2<f32>(1.0, 1.0),
          vec2<f32>(0.0, 0.0),
          vec2<f32>(1.0, 1.0),
          vec2<f32>(1.0, 0.0),
          vec2<f32>(0.0, 0.0)
        );

        var output: VertexOutput;
        output.position = vec4<f32>(pos[vertexIndex], 0.0, 1.0);
        output.texCoord = texCoord[vertexIndex];
        return output;
      }

      @group(0) @binding(0) var inputTexture: texture_2d<f32>;
      @group(0) @binding(1) var inputSampler: sampler;

      // Convert from TV range (16-235) to full range (0-255) for luma
      // and (16-240) to full range for chroma
      fn tvRangeToFullRange(color: vec3<f32>) -> vec3<f32> {
        // For RGB that was converted from YUV with TV range
        // Expand the limited range to full range
        let expanded = (color - vec3<f32>(16.0/255.0)) / ((235.0 - 16.0) / 255.0);
        return clamp(expanded, vec3<f32>(0.0), vec3<f32>(1.0));
      }

      // Rec.709 gamma correction (similar to sRGB but slightly different)
      fn rec709ToLinear(color: vec3<f32>) -> vec3<f32> {
        let alpha = 1.09929682680944;
        let beta = 0.018053968510807;
        return select(
          pow((color + alpha - 1.0) / alpha, vec3<f32>(1.0 / 0.45)),
          color / 4.5,
          color < vec3<f32>(beta)
        );
      }

      fn linearToRec709(linear: vec3<f32>) -> vec3<f32> {
        let alpha = 1.09929682680944;
        let beta = 0.018053968510807;
        return select(
          alpha * pow(linear, vec3<f32>(0.45)) - (alpha - 1.0),
          4.5 * linear,
          linear < vec3<f32>(beta / 4.5)
        );
      }

      // Convert from sRGB gamma for display
      fn linearToSrgb(linear: vec3<f32>) -> vec3<f32> {
        return select(
          pow(linear, vec3<f32>(1.0 / 2.4)) * 1.055 - 0.055,
          linear * 12.92,
          linear <= vec3<f32>(0.0031308)
        );
      }

      @fragment
      fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
        let rawColor = textureSampleLevel(inputTexture, inputSampler, input.texCoord, 0.0);

        // Toggle between different color handling approaches
        // Approach 1: Direct passthrough (original behavior)
        // let finalColor = rawColor.rgb;

        // Approach 2: TV range expansion only
        // let finalColor = tvRangeToFullRange(rawColor.rgb);

        // Approach 3: Full Rec.709 to sRGB conversion
        // let fullRangeColor = tvRangeToFullRange(rawColor.rgb);
        // let linearColor = rec709ToLinear(fullRangeColor);
        // let finalColor = linearToSrgb(linearColor);

        // Alternative approaches to test - comment/uncomment different approaches:
        //
        // APPROACH 1: Direct passthrough (for comparison)
        // Replace the finalColor line with: let finalColor = rawColor.rgb;
        //
        // APPROACH 2: Simple TV range expansion (most likely fix)
        // Replace the finalColor lines with:
        // let finalColor = tvRangeToFullRange(rawColor.rgb);
        //
        // APPROACH 3: Full color space conversion (current implementation)
        // Keep the current implementation
        //
        // APPROACH 4: Gamma-only correction (if TV range is not the issue)
        // Replace the finalColor lines with:
        let finalColor = linearToSrgb(rec709ToLinear(rawColor.rgb));

        return vec4<f32>(finalColor, 1.0);
      }
    `,
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
  renderPipeline: GPURenderPipeline;
  latestFrame: GPUTexture;
  outputTexture: GPUTexture;
  executor: PipelineExecutor;
  renderBindGroup: GPUBindGroup;
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
  canvas.width = Math.floor(video.videoWidth * Math.max(config?.scale ?? 1, 1));
  canvas.height = Math.floor(
    video.videoHeight * Math.max(config?.scale ?? 1, 1)
  );

  canvasContext.configure({
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    // Remove explicit colorSpace to let browser handle it naturally
    alphaMode: "opaque",
    colorSpace: "display-p3",
  });

  // Create a new texture for the latest frame (input to Anime4K)
  const latestFrame = device.createTexture({
    size: [video.videoWidth, video.videoHeight],
    format: "rgba8unorm", // Keep unorm for Anime4K compatibility
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

  // Use a single command encoder for both conversion and Anime4K pipeline
  const mainEncoder = device.createCommandEncoder();
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
  updateConfig: (config: Anime4KConfig | null) => void;
}

export function setupAnime4K(
  canvas: HTMLCanvasElement,
  video: HTMLVideoElement,
  config: Anime4KConfig | null = null
): Anime4KController {
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

    device.addEventListener("uncapturederror", (event) => {
      console.error("🚨 WebGPU uncaptured error:", event.error);
    });

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
        currentConfig
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
          if (!shouldRecreateContext(context, video, currentConfig)) {
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
  };

  const cleanup = () => {
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
