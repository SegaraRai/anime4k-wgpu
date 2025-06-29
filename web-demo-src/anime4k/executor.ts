/**
 * Shader pipeline execution engine for Anime4K-wgpu (TypeScript port)
 *
 * This module contains the core pipeline execution logic that binds shader passes
 * to WebGPU resources and executes them in sequence.
 */

/// <reference types="@webgpu/types" />

// Compute shader workgroup size constants
const COMPUTE_WORKGROUP_SIZE_X = 8;
const COMPUTE_WORKGROUP_SIZE_Y = 8;

// Type definitions matching the Rust structures

export interface ScaleFactor {
  /** The numerator of the scale factor fraction */
  numerator: number;
  /** The denominator of the scale factor fraction */
  denominator: number;
}

export type SamplerFilterMode = "nearest" | "linear";

export interface PhysicalTexture {
  /** Unique identifier for this texture */
  id: number;
  /** Number of color components (1=R, 2=RG, 4=RGBA) */
  components: number;
  /** Scale factors for width and height relative to input */
  scale_factor: [ScaleFactor, ScaleFactor];
  /** Whether this texture represents the source input */
  is_source: boolean;
}

export interface InputTextureBinding {
  /** Shader binding point index */
  binding: number;
  /** ID of the physical texture to bind */
  physical_id: number;
}

export interface OutputTextureBinding {
  /** Shader binding point index */
  binding: number;
  /** ID of the physical texture to bind */
  physical_id: number;
}

export interface SamplerBinding {
  /** Shader binding point index */
  binding: number;
  /** Filter mode for this sampler */
  filter_mode: SamplerFilterMode;
}

export interface ExecutablePass {
  /** Human-readable name for debugging */
  name: string;
  /** WGSL shader source code */
  shader: string;
  /** Compute dispatch scale factors (width, height) */
  compute_scale_factors: [number, number];
  /** Input texture bindings for this pass */
  input_textures: InputTextureBinding[];
  /** Output texture bindings for this pass */
  output_textures: OutputTextureBinding[];
  /** Sampler bindings for this pass */
  samplers: SamplerBinding[];
}

export interface ExecutablePipeline {
  /** Human-readable name for debugging */
  name: string;
  /** Physical textures used by this pipeline */
  physical_textures: PhysicalTexture[];
  /** Sampler filter modes required by this pipeline */
  required_samplers: SamplerFilterMode[];
  /** Shader passes to execute in sequence */
  passes: ExecutablePass[];
}

interface BoundExecutablePass {
  /** Human-readable name for debugging */
  name: string;
  /** Compute dispatch dimensions (width, height) */
  computeDimensions: [number, number];
  /** The WebGPU compute pipeline */
  computePipeline: GPUComputePipeline;
  /** Bind group containing all resources for this pass */
  bindGroup: GPUBindGroup;
}

interface BoundPipeline {
  /** Collection of executable passes with their bound resources */
  passes: BoundExecutablePass[];
  cleanup: () => void;
}

export interface PipelineExecutor {
  /** Collection of bound pipelines to execute in sequence */
  boundPipelines: BoundPipeline[];
  cleanup: () => void;
}

/**
 * Creates a texture format based on the number of components
 */
function getTextureFormat(components: number): GPUTextureFormat {
  switch (components) {
    case 1:
      return "r32float";
    case 2:
      return "rg32float";
    default:
      return "rgba32float";
  }
}

/**
 * Creates a WebGPU filter mode from our SamplerFilterMode
 */
function getGPUFilterMode(filterMode: SamplerFilterMode): GPUFilterMode {
  return filterMode === "nearest" ? "nearest" : "linear";
}

/**
 * Creates a new bound pipeline from an executable pipeline
 *
 * Binds the pipeline to GPU resources and creates all necessary textures,
 * samplers, and bind groups for execution.
 */
async function createBoundPipeline(
  pipeline: ExecutablePipeline,
  device: GPUDevice,
  inputTexture: GPUTexture
): Promise<[BoundPipeline, GPUTexture]> {
  const inputSize = [inputTexture.width, inputTexture.height] as const;

  // Create physical texture map
  const physicalTextureMap = new Map<number, [GPUTexture, GPUTextureView]>();

  for (const pt of pipeline.physical_textures) {
    let texture: GPUTexture;

    if (pt.is_source) {
      // Use the input texture directly for source textures
      texture = inputTexture;
    } else {
      const width = Math.floor(
        (inputSize[0] * pt.scale_factor[0].numerator) /
          pt.scale_factor[0].denominator
      );
      const height = Math.floor(
        (inputSize[1] * pt.scale_factor[1].numerator) /
          pt.scale_factor[1].denominator
      );

      const format = getTextureFormat(pt.components);

      texture = device.createTexture({
        label: `Physical Texture ${pt.id}`,
        size: { width, height, depthOrArrayLayers: 1 },
        mipLevelCount: 1,
        sampleCount: 1,
        dimension: "2d",
        format: format,
        usage:
          GPUTextureUsage.STORAGE_BINDING |
          GPUTextureUsage.TEXTURE_BINDING |
          GPUTextureUsage.COPY_DST |
          GPUTextureUsage.RENDER_ATTACHMENT |
          GPUTextureUsage.COPY_SRC,
      });
    }

    const textureView = texture.createView();
    physicalTextureMap.set(pt.id, [texture, textureView]);
  }

  // Create sampler map
  const samplerMap = new Map<SamplerFilterMode, GPUSampler>();

  for (const filterMode of pipeline.required_samplers) {
    const sampler = device.createSampler({
      label: `Sampler ${filterMode}`,
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
      addressModeW: "clamp-to-edge",
      magFilter: getGPUFilterMode(filterMode),
      minFilter: getGPUFilterMode(filterMode),
      mipmapFilter: "nearest",
    });
    samplerMap.set(filterMode, sampler);
  }

  const passes: BoundExecutablePass[] = [];

  for (const shaderPass of pipeline.passes) {
    const computeDimensions: [number, number] = [
      Math.floor(inputSize[0] * shaderPass.compute_scale_factors[0]),
      Math.floor(inputSize[1] * shaderPass.compute_scale_factors[1]),
    ];

    const skipBoundCheck =
      computeDimensions[0] % COMPUTE_WORKGROUP_SIZE_X === 0 &&
      computeDimensions[1] % COMPUTE_WORKGROUP_SIZE_Y === 0;

    const shaderModule = device.createShaderModule({
      label: shaderPass.name,
      code: shaderPass.shader,
    });

    // Check for shader compilation errors
    const compilationInfo = await shaderModule.getCompilationInfo();
    if (compilationInfo.messages.some((msg) => msg.type === "error")) {
      const errors = compilationInfo.messages.filter(
        (msg) => msg.type === "error"
      );
      throw new Error(
        `Shader compilation failed for ${shaderPass.name}: ${errors.map((e) => e.message).join(", ")}`
      );
    }

    // Create explicit bind group layout based on the pass requirements
    const bindGroupLayoutEntries: GPUBindGroupLayoutEntry[] = [];

    // Add input texture bindings
    for (const input of shaderPass.input_textures) {
      bindGroupLayoutEntries.push({
        binding: input.binding,
        visibility: GPUShaderStage.COMPUTE,
        texture: {
          sampleType: "float",
          viewDimension: "2d",
          multisampled: false,
        },
      });
    }

    // Add output texture bindings
    for (const output of shaderPass.output_textures) {
      const [texture] = physicalTextureMap.get(output.physical_id)!;
      const storageFormat = texture.format;

      bindGroupLayoutEntries.push({
        binding: output.binding,
        visibility: GPUShaderStage.COMPUTE,
        storageTexture: {
          access: "write-only",
          format: storageFormat,
          viewDimension: "2d",
        },
      });
    }

    // Add sampler bindings
    for (const samplerBinding of shaderPass.samplers) {
      bindGroupLayoutEntries.push({
        binding: samplerBinding.binding,
        visibility: GPUShaderStage.COMPUTE,
        sampler: {
          type: "filtering",
        },
      });
    }

    // Sort by binding number
    bindGroupLayoutEntries.sort((a, b) => a.binding - b.binding);

    const bindGroupLayout = device.createBindGroupLayout({
      label: shaderPass.name,
      entries: bindGroupLayoutEntries,
    });

    const pipelineLayout = device.createPipelineLayout({
      label: shaderPass.name,
      bindGroupLayouts: [bindGroupLayout],
    });

    // Create compute pipeline with explicit layout
    const computePipeline = device.createComputePipeline({
      label: shaderPass.name,
      layout: pipelineLayout,
      compute: {
        module: shaderModule,
        entryPoint: skipBoundCheck ? "main_unchecked" : "main",
      },
    });

    // Create bind group using the analyzed texture bindings
    const bindGroupEntries: GPUBindGroupEntry[] = [];

    for (const input of shaderPass.input_textures) {
      const [, textureView] = physicalTextureMap.get(input.physical_id)!;
      bindGroupEntries.push({
        binding: input.binding,
        resource: textureView,
      });
    }

    for (const output of shaderPass.output_textures) {
      const [, textureView] = physicalTextureMap.get(output.physical_id)!;
      bindGroupEntries.push({
        binding: output.binding,
        resource: textureView,
      });
    }

    for (const samplerBinding of shaderPass.samplers) {
      const sampler = samplerMap.get(samplerBinding.filter_mode)!;
      bindGroupEntries.push({
        binding: samplerBinding.binding,
        resource: sampler,
      });
    }

    bindGroupEntries.sort((a, b) => a.binding - b.binding);

    const bindGroup = device.createBindGroup({
      label: shaderPass.name,
      layout: computePipeline.getBindGroupLayout(0),
      entries: bindGroupEntries,
    });

    passes.push({
      name: shaderPass.name,
      computeDimensions,
      computePipeline,
      bindGroup,
    });
  }

  const outputTexture = physicalTextureMap.get(
    pipeline.passes[pipeline.passes.length - 1].output_textures[0].physical_id
  )![0];

  return [
    {
      passes,
      cleanup: (): void => {
        for (const [texture] of physicalTextureMap.values()) {
          texture.destroy();
        }
      },
    },
    outputTexture,
  ];
}

/**
 * Executes all passes in a bound pipeline
 */
function executeBoundPipeline(
  boundPipeline: BoundPipeline,
  encoder: GPUCommandEncoder
): void {
  for (const pass of boundPipeline.passes) {
    const computePass = encoder.beginComputePass({
      label: pass.name,
    });

    computePass.setPipeline(pass.computePipeline);
    computePass.setBindGroup(0, pass.bindGroup);

    const [computeWidth, computeHeight] = pass.computeDimensions;
    const workgroupX = Math.ceil(computeWidth / COMPUTE_WORKGROUP_SIZE_X);
    const workgroupY = Math.ceil(computeHeight / COMPUTE_WORKGROUP_SIZE_Y);

    computePass.dispatchWorkgroups(workgroupX, workgroupY, 1);
    computePass.end();
  }
}

/**
 * Creates a new shader pipeline from executable pipelines
 *
 * Binds all pipelines to GPU resources and chains them together so that
 * the output of one pipeline becomes the input of the next.
 */
export async function createPipelineExecutor(
  executablePipelines: ExecutablePipeline[],
  device: GPUDevice,
  sourceTexture: GPUTexture
): Promise<[PipelineExecutor, GPUTexture]> {
  const boundPipelines: BoundPipeline[] = [];
  let currentInputTexture = sourceTexture;

  for (const pipeline of executablePipelines) {
    const [boundPipeline, outputTexture] = await createBoundPipeline(
      pipeline,
      device,
      currentInputTexture
    );
    currentInputTexture = outputTexture;
    boundPipelines.push(boundPipeline);
  }

  return [
    {
      boundPipelines,
      cleanup: (): void => {
        for (const pipeline of boundPipelines) {
          pipeline.cleanup();
        }
      },
    },
    currentInputTexture,
  ];
}

/**
 * Executes the entire shader pipeline
 */
export function executePipeline(
  executor: PipelineExecutor,
  encoder: GPUCommandEncoder
): void {
  for (const boundPipeline of executor.boundPipelines) {
    executeBoundPipeline(boundPipeline, encoder);
  }
}
