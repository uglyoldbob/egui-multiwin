//! This is an example of a popup window. It is likely very crude on the opengl_after function and could probably be optimized
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};
use egui_multiwin::egui_glow::glow;
use egui_multiwin::egui_glow::EguiGlow;

use crate::AppCommon;

pub struct PopupWindow {
    pub input: String,
}

impl PopupWindow {
    pub fn request(label: String) -> NewWindowRequest {
        NewWindowRequest::new(
            super::MyWindows::Popup(PopupWindow {
                input: label.clone(),
            }),
            egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(false)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 400.0,
                    height: 200.0,
                })
                .with_title(label),
            egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            egui_multiwin::multi_window::new_id(),
        )
    }
}

impl TrackedWindow for PopupWindow {
    unsafe fn opengl_after(
        &mut self,
        _c: &mut AppCommon,
        gl: &std::sync::Arc<egui_multiwin::egui_glow::painter::Context>,
    ) {
        use glow::HasContext;
        let shader_version = egui_multiwin::egui_glow::ShaderVersion::get(gl);
        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));
        let program = gl.create_program().expect("Cannot create program");
        let (vertex_shader_source, fragment_shader_source) = (
            r#"const vec2 verts[3] = vec2[3](
                vec2(0.5f, 1.0f),
                vec2(0.0f, 0.0f),
                vec2(1.0f, 0.0f)
            );
            out vec2 vert;
            void main() {
                vert = verts[gl_VertexID];
                gl_Position = vec4(vert - 0.5, 0.0, 1.0);
            }"#,
            r#"precision mediump float;
            in vec2 vert;
            out vec4 color;
            void main() {
                color = vec4(vert, 0.5, 1.0);
            }"#,
        );

        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];
        let mut shaders = Vec::with_capacity(shader_sources.len());
        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(
                shader,
                &format!(
                    "{}\n{}",
                    shader_version.version_declaration(),
                    shader_source
                ),
            );
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("{}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        gl.use_program(Some(program));

        gl.draw_arrays(glow::TRIANGLES, 0, 3);
    }

    fn can_quit(&mut self, c: &mut AppCommon) -> bool {
        (c.clicks & 1) == 0
    }

    fn redraw(
        &mut self,
        c: &mut AppCommon,
        egui: &mut EguiGlow,
        window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut egui_multiwin::arboard::Clipboard,
    ) -> RedrawResponse {
        let mut quit = false;

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if ui.button("Increment").clicked() {
                c.clicks += 1;
                window.set_title(&format!("Title update {}", c.clicks));
            }
            let response = ui.add(egui_multiwin::egui::TextEdit::singleline(&mut self.input));
            if response.changed() {
                // …
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui_multiwin::egui::Key::Enter))
            {
                // …
            }
            if ui.button("Quit").clicked() {
                quit = true;
            }
        });
        RedrawResponse {
            quit,
            new_windows: Vec::new(),
        }
    }
}
