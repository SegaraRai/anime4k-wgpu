# CLAUDE.md for wgsl/

## Coding Style Guide

### Shader Code

Follow the existing coding style. The following shaders may be helpful for reference:

- wgsl/auxiliary/deblur_dog_pass3_kernel_y.wgsl
- wgsl/auxiliary/effects_darken_pass2_gaussian_x_scaling.wgsl

Remember to:

- Practice basic optimization. Perform type conversions and calculations in advance, and define constants such as texture dimensions outside of loops.
- Do not modify the alpha channel. The alpha channel should either maintain the same value as the source (for the same dimension) or be linearly sampled by the sampler (for different dimensions).
- The workgroup size must be 8x8x1, and you need a `main` entry point that performs boundary checks and a `main_unchecked` entry point that does not.
- Boundary checks are required when using `textureLoad`. Define the maximum texture coordinates in advance as `let bound = vec2i(textureDimensions(tex)) - 1;`.
- Use `textureLoad` for the same dimensions and `textureSampleLevel` for different dimensions. This distinction must be made except in special cases where linear sampling is always required.
  Within the arguments of `textureLoad`, use only one of `min`, `max`, or `clamp` to clip the coordinates.
- Use `clamp` instead of `min(max(...), ...)` or `max(min(...), ...)`.
- Do not perform boundary checks in `textureSampleLevel`. The driver handles it appropriately.
- Always use the `textureSampleLevel` function at level `0.0` instead of `textureSample`, since it cannot be used in the `@compute` shader.
- Always use float32 textures for better precision.
- For intermediate textures with three or fewer components, set the values of unused components to `0.0` for R, G, and B, and `1.0` for A.
