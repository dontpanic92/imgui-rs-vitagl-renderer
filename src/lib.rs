#![feature(offset_of)]

use std::ffi::c_void;
use std::mem::{offset_of, zeroed};

use bindings::c_ImGui_ImplVitaGL_InitTouch;
use imgui::sys::*;
use vitagl_sys::*;
use vitasdk_sys::psp2::kernel::processmgr::sceKernelGetProcessTimeWide;
use vitasdk_sys::psp2::{ctrl::*, touch::*};
use vitasdk_sys::psp2common::ctrl::SceCtrlButtons::*;
use vitasdk_sys::psp2common::ctrl::*;

use crate::bindings::c_ImGui_ImplVitaGL_PollTouch;

pub struct ImguiRenderer {
    imgui_mempool_size: usize,
    start_vertex: *mut f32,
    start_texcoord: *mut f32,
    start_color: *mut u8,
    vertex_buffer: *mut f32,
    texcoord_buffer: *mut f32,
    color_buffer: *mut u8,
    index_buffer: *mut u16,
    font_texture: Option<std::ffi::c_uint>,
    touch_usage: bool,
    gamepad_usage: bool,
    g_time: u64,
    mx: i32,
    my: i32,
    mouse_pressed: [i32; 3],
    hires_x: i32,
    hires_y: i32,
    counter: usize,
}

impl ImguiRenderer {
    pub fn new() -> Self {
        unsafe {
            sceTouchSetSamplingState(
                SceTouchPortType::SCE_TOUCH_PORT_FRONT,
                SceTouchSamplingState::SCE_TOUCH_SAMPLING_STATE_START,
            );
            sceCtrlSetSamplingMode(SceCtrlPadInputMode::SCE_CTRL_MODE_ANALOG_WIDE);

            let io = &mut *imgui::sys::igGetIO();
            io.MouseDrawCursor = false;

            let imgui_mempool_size = 0x200000;
            let vertex_buffer =
                imgui::sys::igMemAlloc(std::mem::size_of::<f32>() * imgui_mempool_size * 3)
                    as *mut f32;
            let texcoord_buffer =
                imgui::sys::igMemAlloc(std::mem::size_of::<f32>() * imgui_mempool_size * 2)
                    as *mut f32;
            let color_buffer: *mut u8 =
                imgui::sys::igMemAlloc(std::mem::size_of::<u8>() * imgui_mempool_size * 4)
                    as *mut u8;
            let index_buffer =
                imgui::sys::igMemAlloc(std::mem::size_of::<u16>() * 0xF000) as *mut u16;

            for i in 0..0xF000 {
                *index_buffer.add(i) = i as u16;
            }

            io.ClipboardUserData = std::ptr::null_mut();
            c_ImGui_ImplVitaGL_InitTouch();

            Self {
                imgui_mempool_size,
                start_color: color_buffer,
                start_texcoord: texcoord_buffer,
                start_vertex: vertex_buffer,
                color_buffer,
                texcoord_buffer,
                vertex_buffer,
                index_buffer,
                font_texture: None,
                touch_usage: true,
                gamepad_usage: true,
                g_time: 0,
                mx: 0,
                my: 0,
                mouse_pressed: [0; 3],
                hires_x: 0,
                hires_y: 0,
                counter: 0,
            }
        }
    }

    fn create_device_objects(&mut self) {
        unsafe {
            let io = &mut *imgui::sys::igGetIO();

            let mut pixels = std::ptr::null_mut();
            let mut width = 0;
            let mut height = 0;
            let mut bytes_per_pixel = 0;
            imgui::sys::ImFontAtlas_GetTexDataAsRGBA32(
                io.Fonts,
                &mut pixels,
                &mut width,
                &mut height,
                &mut bytes_per_pixel,
            );

            let mut last_texture = 0;
            glGetIntegerv(GL_TEXTURE_BINDING_2D, &mut last_texture);

            let mut new_texture = 0;
            glGenTextures(1, &mut new_texture);
            glBindTexture(GL_TEXTURE_2D, new_texture);
            self.font_texture = Some(new_texture);

            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
            //glPixelStorei(GL_UNPACK_ROW_LENGTH, 0);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA as i32,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                pixels as *const _,
            );

            imgui::sys::ImFontAtlas_SetTexID(
                io.Fonts,
                self.font_texture.unwrap() as usize as *mut _,
            );

            glBindTexture(GL_TEXTURE_2D, last_texture as u32);
        }
    }

    fn invalidate_device_objects(&mut self) {
        let font_texture = self.font_texture.take();
        if let Some(font_texture) = font_texture {
            unsafe {
                glDeleteTextures(1, &font_texture);

                let io = &mut *imgui::sys::igGetIO();
                imgui::sys::ImFontAtlas_SetTexID(io.Fonts, std::ptr::null_mut());
            }
        }
    }
}

impl ImguiRenderer {
    pub fn new_frame(&mut self) {
        if self.font_texture.is_none() {
            self.create_device_objects();
        }

        unsafe {
            let io = &mut *imgui::sys::igGetIO();

            let mut viewport: [i32; 4] = [0; 4];
            glGetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());
            let w = viewport[2];
            let h = viewport[3];

            io.DisplaySize = imgui::sys::ImVec2::new(w as f32, h as f32);
            io.DisplayFramebufferScale = imgui::sys::ImVec2::new(1., 1.);

            const FREQUENCY: usize = 1000000;
            let current_time = sceKernelGetProcessTimeWide();
            io.DeltaTime = if self.g_time > 0 {
                (current_time - self.g_time) as f32 / FREQUENCY as f32
            } else {
                1.0 / 60.0
            };
            self.g_time = current_time;

            if self.touch_usage {
                let scale_x = 960.0 / io.DisplaySize.x;
                let scale_y = 544.0 / io.DisplaySize.y;
                let offset_x = 0.;
                let offset_y = 0.;
                c_ImGui_ImplVitaGL_PollTouch(
                    offset_x,
                    offset_y,
                    scale_x as f64,
                    scale_y as f64,
                    &mut self.mx,
                    &mut self.my,
                    self.mouse_pressed.as_mut_ptr(),
                );
            }

            if self.gamepad_usage {
                let mut pad = zeroed::<SceCtrlData>();
                let mut lstick_x = 0;
                let mut lstick_y = 0;
                self.poll_left_stick(&mut pad, &mut lstick_x, &mut lstick_y);

                io.NavInputs[ImGuiNavInput_Activate as usize] = if pad.buttons & SCE_CTRL_CROSS != 0
                {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_Cancel as usize] = if pad.buttons & SCE_CTRL_CIRCLE != 0
                {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_Input as usize] = if pad.buttons & SCE_CTRL_TRIANGLE != 0
                {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_Menu as usize] = if pad.buttons & SCE_CTRL_SQUARE != 0 {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_DpadLeft as usize] = if pad.buttons & SCE_CTRL_LEFT != 0
                {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_DpadRight as usize] =
                    if pad.buttons & SCE_CTRL_RIGHT != 0 {
                        1.
                    } else {
                        0.
                    };
                io.NavInputs[ImGuiNavInput_DpadUp as usize] = if pad.buttons & SCE_CTRL_UP != 0 {
                    1.
                } else {
                    0.
                };
                io.NavInputs[ImGuiNavInput_DpadDown as usize] = if pad.buttons & SCE_CTRL_DOWN != 0
                {
                    1.
                } else {
                    0.
                };

                // if !self.mousestick_usage || io.NavInputs[ImGuiNavInput_Menu] == 1.0f {
                io.NavInputs[ImGuiNavInput_FocusPrev as usize] =
                    if pad.buttons & SCE_CTRL_LTRIGGER != 0 {
                        1.
                    } else {
                        0.
                    };
                io.NavInputs[ImGuiNavInput_FocusNext as usize] =
                    if pad.buttons & SCE_CTRL_RTRIGGER != 0 {
                        1.
                    } else {
                        0.
                    };
                if lstick_x < 0 {
                    io.NavInputs[ImGuiNavInput_LStickLeft as usize] = -lstick_x as f32 / 16.
                };
                if lstick_x > 0 {
                    io.NavInputs[ImGuiNavInput_LStickRight as usize] = lstick_x as f32 / 16.
                };
                if lstick_y < 0 {
                    io.NavInputs[ImGuiNavInput_LStickUp as usize] = -lstick_y as f32 / 16.
                };
                if lstick_y > 0 {
                    io.NavInputs[ImGuiNavInput_LStickDown as usize] = lstick_y as f32 / 16.
                };
                // }
            }

            // Keys for mouse emulation
            /*if (mousestick_usage && !(io.NavInputs[ImGuiNavInput_Menu] == 1.0f)){
                SceCtrlData pad;
                ImGui_ImplVitaGL_PollLeftStick(&pad, &mx, &my);
                if ((pad.buttons & SCE_CTRL_LTRIGGER) != (g_OldPad.buttons & SCE_CTRL_LTRIGGER))
                    g_MousePressed[0] = pad.buttons & SCE_CTRL_LTRIGGER;
                if ((pad.buttons & SCE_CTRL_RTRIGGER) != (g_OldPad.buttons & SCE_CTRL_RTRIGGER))
                    g_MousePressed[1] = pad.buttons & SCE_CTRL_RTRIGGER;
                g_OldPad = pad;
            }
            */

            // Setup mouse inputs (we already got mouse wheel, keyboard keys & characters from our event handler)
            //Uint32 mouse_buttons = SDL_GetMouseState(&mx, &my);
            io.MousePos = imgui::sys::ImVec2::new(std::f32::MIN, std::f32::MIN);
            io.MouseDown[0] = self.mouse_pressed[0] != 0;
            io.MouseDown[1] = self.mouse_pressed[1] != 0;
            io.MouseDown[2] = self.mouse_pressed[2] != 0;

            if self.mx < 0 {
                self.mx = 0;
            } else if self.mx > 960 {
                self.mx = 960;
            }
            if self.my < 0 {
                self.my = 0;
            } else if self.my > 544 {
                self.my = 544;
            }

            io.MousePos = imgui::sys::ImVec2::new(self.mx as f32, self.my as f32);

            // imgui::sys::igNewFrame();
            vglIndexPointerMapped(self.index_buffer as *const _);
        }
    }

    pub fn poll_left_stick(&mut self, pad: *mut SceCtrlData, x: *mut i32, y: *mut i32) {
        unsafe {
            sceCtrlPeekBufferPositive(0, pad, 1);
            let mut lx = ((*pad).lx as i32 - 127) * 256;
            let mut ly = ((*pad).ly as i32 - 127) * 256;
            rescale_analog(&mut lx, &mut ly, 7680);
            self.hires_x += lx;
            self.hires_y += ly;
            if self.hires_x != 0 || self.hires_y != 0 {
                let slowdown = 2048;
                *x += self.hires_x / slowdown;
                *y += self.hires_y / slowdown;
                self.hires_x %= slowdown;
                self.hires_y %= slowdown;
            }
        }
    }

    pub fn render(&mut self) {
        unsafe {
            imgui::sys::igRender();
            let draw_data = imgui::sys::igGetDrawData();

            let io = &mut *imgui::sys::igGetIO();
            let fb_width = (io.DisplaySize.x * io.DisplayFramebufferScale.x) as i32;
            let fb_height = (io.DisplaySize.y * io.DisplayFramebufferScale.y) as i32;
            if fb_width == 0 || fb_height == 0 {
                return;
            }

            imgui::sys::ImDrawData_ScaleClipRects(draw_data, io.DisplayFramebufferScale);

            let mut last_texture = 0;
            glGetIntegerv(GL_TEXTURE_BINDING_2D, &mut last_texture);

            let mut last_polygon_mode = [0; 2];
            glGetIntegerv(GL_POLYGON_MODE, last_polygon_mode.as_mut_ptr());
            let mut last_viewport = [0; 4];
            glGetIntegerv(GL_VIEWPORT, last_viewport.as_mut_ptr());
            let mut last_scissor_box = [0; 4];
            glGetIntegerv(GL_SCISSOR_BOX, last_scissor_box.as_mut_ptr());

            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            glDisable(GL_CULL_FACE);
            glDisable(GL_DEPTH_TEST);
            glEnable(GL_SCISSOR_TEST);
            glEnableClientState(GL_VERTEX_ARRAY);
            glEnableClientState(GL_TEXTURE_COORD_ARRAY);
            glEnableClientState(GL_COLOR_ARRAY);
            glEnable(GL_TEXTURE_2D);
            glPolygonMode(GL_FRONT_AND_BACK, GL_FILL);

            glViewport(0, 0, fb_width, fb_height);
            glMatrixMode(GL_PROJECTION);
            glPushMatrix();
            glLoadIdentity();
            glOrtho(
                0.,
                io.DisplaySize.x as f64,
                io.DisplaySize.y as f64,
                0.,
                0.,
                1.,
            );
            glMatrixMode(GL_MODELVIEW);
            glPushMatrix();
            glLoadIdentity();

            let count = (*draw_data).CmdListsCount;

            for n in 0..count {
                let cmd_list = &mut **(*draw_data).CmdLists.add(n as usize);
                let vtx_buffer = cmd_list.VtxBuffer.Data as *mut u8;
                let mut idx_buffer = cmd_list.IdxBuffer.Data;

                for cmd_i in 0..cmd_list.CmdBuffer.Size {
                    let pcmd = &mut *cmd_list.CmdBuffer.Data.add(cmd_i as usize);
                    if pcmd.UserCallback.is_some() {
                        (pcmd.UserCallback.as_ref().unwrap())(cmd_list, pcmd);
                    } else {
                        glBindTexture(GL_TEXTURE_2D, pcmd.TextureId as u32);
                        glScissor(
                            pcmd.ClipRect.x as i32,
                            (fb_height as f32 - pcmd.ClipRect.w) as i32,
                            (pcmd.ClipRect.z - pcmd.ClipRect.x) as i32,
                            (pcmd.ClipRect.w - pcmd.ClipRect.y) as i32,
                        );

                        let vp = self.vertex_buffer;
                        let tp = self.texcoord_buffer;
                        let cp = self.color_buffer;
                        let indices = idx_buffer;
                        for idx in 0..pcmd.ElemCount {
                            let index = *(indices.add(idx as usize));
                            let vertices = vtx_buffer.add(
                                offset_of!(ImDrawVert, pos)
                                    + std::mem::size_of::<ImDrawVert>() * index as usize,
                            ) as *const _ as *const f32;
                            let texcoords = vtx_buffer.add(
                                offset_of!(ImDrawVert, uv)
                                    + std::mem::size_of::<ImDrawVert>() * index as usize,
                            ) as *const _ as *const f32;
                            let colors = vtx_buffer.add(
                                offset_of!(ImDrawVert, col)
                                    + std::mem::size_of::<ImDrawVert>() * index as usize,
                            ) as *const _ as *const u8;

                            *self.vertex_buffer = *vertices.add(0);
                            *self.vertex_buffer.add(1) = *vertices.add(1);
                            *self.vertex_buffer.add(2) = 0.;
                            *self.texcoord_buffer.add(0) = *texcoords.add(0);
                            *self.texcoord_buffer.add(1) = *texcoords.add(1);
                            *self.color_buffer.add(0) = *colors.add(0);
                            *self.color_buffer.add(1) = *colors.add(1);
                            *self.color_buffer.add(2) = *colors.add(2);
                            *self.color_buffer.add(3) = *colors.add(3);
                            self.vertex_buffer = self.vertex_buffer.add(3);
                            self.texcoord_buffer = self.texcoord_buffer.add(2);
                            self.color_buffer = self.color_buffer.add(4);
                        }

                        if false
                        /*shaders_usage*/
                        {
                            vglVertexAttribPointerMapped(0, vp as *const _);
                            vglVertexAttribPointerMapped(1, tp as *const _);
                            vglVertexAttribPointerMapped(2, cp as *const _);
                        } else {
                            vglVertexPointerMapped(3, vp as *const _);
                            vglTexCoordPointerMapped(tp as *const _);
                            vglColorPointerMapped(GL_UNSIGNED_BYTE, cp as *const _);
                        }
                        vglDrawObjects(GL_TRIANGLES, pcmd.ElemCount as i32, GL_TRUE as u8);
                    }

                    idx_buffer = idx_buffer.add(pcmd.ElemCount as usize);
                    self.counter += pcmd.ElemCount as usize;
                    if self.counter > self.imgui_mempool_size - 0x66700 {
                        self.vertex_buffer = self.start_vertex;
                        self.color_buffer = self.start_color;
                        self.texcoord_buffer = self.start_texcoord;
                        self.counter = 0;
                    }
                }
            }

            glDisableClientState(GL_COLOR_ARRAY);
            glDisableClientState(GL_TEXTURE_COORD_ARRAY);
            glDisableClientState(GL_VERTEX_ARRAY);
            glBindTexture(GL_TEXTURE_2D, last_texture as u32);
            glMatrixMode(GL_MODELVIEW);
            glPopMatrix();
            glMatrixMode(GL_PROJECTION);
            glPopMatrix();
            glPolygonMode(GL_FRONT, last_polygon_mode[0] as u32);
            glPolygonMode(GL_BACK, last_polygon_mode[1] as u32);
            glViewport(
                last_viewport[0],
                last_viewport[1],
                last_viewport[2],
                last_viewport[3],
            );
            glScissor(
                last_scissor_box[0],
                last_scissor_box[1],
                last_scissor_box[2],
                last_scissor_box[3],
            );
        }
    }
}

impl Drop for ImguiRenderer {
    fn drop(&mut self) {
        unsafe {
            imgui::sys::igMemFree(self.color_buffer as *mut c_void);
            imgui::sys::igMemFree(self.texcoord_buffer as *mut c_void);
            imgui::sys::igMemFree(self.vertex_buffer as *mut c_void);
            imgui::sys::igMemFree(self.index_buffer as *mut c_void);
        }

        self.invalidate_device_objects();
    }
}

#[allow(dead_code)]
mod bindings {
    extern "C" {
        pub fn c_ImGui_ImplVitaGL_InitTouch();
        pub fn c_ImGui_ImplVitaGL_PollTouch(
            x0: std::ffi::c_double,
            y0: std::ffi::c_double,
            sx: std::ffi::c_double,
            sy: std::ffi::c_double,
            mx: *mut std::ffi::c_int,
            my: *mut std::ffi::c_int,
            mbuttons: *mut std::ffi::c_int,
        );
    }
}

fn rescale_analog(x: &mut i32, y: &mut i32, dead: i32) {
    let analog_x = *x as f32;
    let analog_y = *y as f32;
    let dead_zone = dead as f32;
    let maximum = 32768.;
    let magnitude = (analog_x * analog_x + analog_y * analog_y).sqrt();
    if magnitude >= dead_zone {
        let scaling_factor = maximum / magnitude * (magnitude - dead_zone) / (maximum - dead_zone);
        *x = (analog_x * scaling_factor) as i32;
        *y = (analog_y * scaling_factor) as i32;
    } else {
        *x = 0;
        *y = 0;
    }
}

#[link(name = "vitaGL", kind = "static")]
extern "C" {}

#[link(name = "vitashark", kind = "static")]
extern "C" {}

#[link(name = "SceShaccCg_stub", kind = "static")]
extern "C" {}

#[link(name = "SceShaccCgExt", kind = "static")]
extern "C" {}

#[link(name = "taihen_stub", kind = "static")]
extern "C" {}

#[link(name = "mathneon", kind = "static")]
extern "C" {}
