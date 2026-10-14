// Linux software-GPU CI needs Chromium's compositor, ANGLE and WebGPU on
// a compatible Vulkan path. ANGLE-only SwiftShader fails shared-image creation
// for this viewer. Hardware browser acceptance retains its original arguments.
export function browserArguments({headless, softwareGpu, platform = process.platform}) {
  if (!headless && !softwareGpu) return ['--enable-unsafe-webgpu'];
  if (platform !== 'linux' || !softwareGpu) return ['--enable-unsafe-webgpu', '--use-angle=swiftshader'];
  return [
    '--enable-unsafe-webgpu',
    // Full-document capture suppresses Chromium scrollbars. Suppress them
    // from launch too, so capture cannot change the centered layout by 7.5px.
    '--hide-scrollbars',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--use-vulkan=swiftshader',
    '--enable-features=Vulkan,VulkanFromANGLE,DefaultANGLEVulkan',
    '--disable-vulkan-surface',
  ];
}
