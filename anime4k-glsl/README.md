# Anime4K GLSL Shaders

This directory contains the original GLSL shaders from the [Anime4K repository](https://github.com/bloc97/Anime4K/tree/master). Git submodules are not used because the repository contains many files and is too heavy.

The CNN/GAN-based neural network shaders are automatically converted to WGSL and Rust code by the build script in the `../crates/build/` directory. Auxiliary shaders are manually ported to WGSL in the `../wgsl/` directory.

## License

These shaders are from the original Anime4K project by bloc97 and are licensed under the MIT License. See [LICENSE](LICENSE) for details.
