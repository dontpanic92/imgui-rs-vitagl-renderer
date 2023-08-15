use imgui_rs_vitagl_renderer::ImguiRenderer;

fn main() {
    let mut renderer = ImguiRenderer::new();
    renderer.new_frame();
    renderer.render();
}
