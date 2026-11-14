import { selected } from './context.ts';

const GPU_TESTS = [
  'export::render::tests::export_contains_current_color_and_square_corners',
  'export::render::tests::rasterizer_honors_every_export_scale',
  'export::render::tests::export_restores_the_interface_font_language',
  'export::render::tests::export_uses_chart_variant_independently_of_interface',
  'export::tests::flow::chosen_path_exports_a_real_chart_via_the_background_worker',
  'export::tests::flow::flying_export_contains_the_four_default_colors',
  'export::tests::flow::export_matches_visual_fixture_after_worker_completes',
  'export::tests::flow::hidden_stem_or_palace_export_preserves_grid_layout',
  'export::copy::tests::copy_freezes_the_visible_chart_and_hidden_marks_are_excluded',
  'export::tests::flow::exported_image_freezes_edited_drawings_and_text_before_the_dialog',
  'app::view::tests::window_matches_wide_visual_fixture',
];

/** 运行真实图片导出和离屏视觉回归，逐个核对被选中的测试。 */
export function verifyGpu(env: NodeJS.ProcessEnv, offline: boolean): void {
  const locked = ['--locked', ...(offline ? ['--offline'] : [])];
  let count = 0;
  for (const name of GPU_TESTS) count += selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--bin', 'zhiji', '--all-features', name, '--', '--ignored', '--exact', '--test-threads=1'], env, true);
  count += selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--test', 'visual_regression', '--all-features', '--', '--ignored', '--test-threads=1'], env);
  console.log(`Desktop GPU checks passed: ${count} tests.`);
}
