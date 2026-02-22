use std::ffi::c_void;

use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::*, Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D11::*,
    Win32::Graphics::DirectComposition::*, Win32::Graphics::DirectWrite::*,
    Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Dxgi::*, Win32::UI::WindowsAndMessaging::*,
};

use crate::scene::{Element, SceneGraph};
use crate::types::{DrawOp, Error, ImageElement, RectElement, TextElement};
use crate::window::get_dpi_scale;

// --- Shared GPU resources ---

/// Shared GPU resources used across all surfaces.
/// Caller must initialize COM before calling `GpuResources::new()`.
pub(crate) struct GpuResources {
    pub d3d_device: ID3D11Device,
    pub dxgi_device: IDXGIDevice,
    pub d2d_factory: ID2D1Factory1,
    pub d2d_device: ID2D1Device,
    pub dwrite_factory: IDWriteFactory,
}

impl GpuResources {
    pub unsafe fn new() -> Result<Self, Error> {
        // 1. Create D3D11 device (try hardware, fall back to WARP)
        let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
        let mut device = None;

        let result = D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        );

        if result.is_err() || device.is_none() {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                flags,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
            .map_err(|e| Error::DeviceCreation(format!("D3D11 WARP: {e}")))?;
        }

        let d3d_device = device.ok_or_else(|| Error::DeviceCreation("no D3D11 device".into()))?;

        // 2. Query IDXGIDevice
        let dxgi_device: IDXGIDevice = d3d_device
            .cast()
            .map_err(|e| Error::DeviceCreation(format!("IDXGIDevice cast: {e}")))?;

        // 3. D2D1 factory
        let d2d_factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
            .map_err(|e| Error::DeviceCreation(format!("D2D1 factory: {e}")))?;

        // 4. D2D1 device
        let d2d_device = d2d_factory
            .CreateDevice(&dxgi_device)
            .map_err(|e| Error::DeviceCreation(format!("D2D1 device: {e}")))?;

        // 5. DirectWrite factory
        let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
            .map_err(|e| Error::DeviceCreation(format!("DWrite factory: {e}")))?;

        Ok(GpuResources {
            d3d_device,
            dxgi_device,
            d2d_factory,
            d2d_device,
            dwrite_factory,
        })
    }
}

// --- Per-surface renderer ---

pub(crate) struct SurfaceRenderer {
    pub hwnd: HWND,
    pub swapchain: IDXGISwapChain1,
    pub dcomp_device: IDCompositionDevice,
    pub dcomp_target: IDCompositionTarget,
    pub dcomp_visual: IDCompositionVisual,
    pub dc: ID2D1DeviceContext,
    pub dpi_scale: f32,
    pub width: u32,
    pub height: u32,
}

impl SurfaceRenderer {
    pub unsafe fn new(
        gpu: &GpuResources,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, Error> {
        let dpi_scale = get_dpi_scale(hwnd);
        let phys_w = (width as f32 * dpi_scale) as u32;
        let phys_h = (height as f32 * dpi_scale) as u32;

        // Create DXGI swap chain for composition
        let adapter = gpu
            .dxgi_device
            .GetAdapter()
            .map_err(|e| Error::SwapChainCreation(format!("GetAdapter: {e}")))?;
        let factory: IDXGIFactory2 = adapter
            .GetParent()
            .map_err(|e| Error::SwapChainCreation(format!("GetParent: {e}")))?;

        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: phys_w,
            Height: phys_h,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };

        let swapchain = factory
            .CreateSwapChainForComposition(&gpu.d3d_device, &desc, None)
            .map_err(|e| Error::SwapChainCreation(format!("CreateSwapChainForComposition: {e}")))?;

        // DirectComposition: device -> target -> visual -> swap chain
        let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(&gpu.dxgi_device)
            .map_err(|e| Error::DeviceCreation(format!("DComposition device: {e}")))?;
        let dcomp_target = dcomp_device
            .CreateTargetForHwnd(hwnd, true)
            .map_err(|e| Error::DeviceCreation(format!("DComposition target: {e}")))?;
        let dcomp_visual = dcomp_device
            .CreateVisual()
            .map_err(|e| Error::DeviceCreation(format!("DComposition visual: {e}")))?;
        dcomp_visual
            .SetContent(&swapchain)
            .map_err(|e| Error::DeviceCreation(format!("SetContent: {e}")))?;
        dcomp_target
            .SetRoot(&dcomp_visual)
            .map_err(|e| Error::DeviceCreation(format!("SetRoot: {e}")))?;

        // D2D device context + bitmap target from swap chain back buffer
        let dc = gpu
            .d2d_device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(|e| Error::DeviceCreation(format!("D2D device context: {e}")))?;

        let surface: IDXGISurface = swapchain
            .GetBuffer(0)
            .map_err(|e| Error::SwapChainCreation(format!("GetBuffer: {e}")))?;

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * dpi_scale,
            dpiY: 96.0 * dpi_scale,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };

        let bitmap = dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))
            .map_err(|e| Error::SwapChainCreation(format!("CreateBitmapFromDxgiSurface: {e}")))?;
        dc.SetTarget(&bitmap);

        dcomp_device
            .Commit()
            .map_err(|e| Error::DeviceCreation(format!("DComposition commit: {e}")))?;

        Ok(SurfaceRenderer {
            hwnd,
            swapchain,
            dcomp_device,
            dcomp_target,
            dcomp_visual,
            dc,
            dpi_scale,
            width,
            height,
        })
    }

    pub unsafe fn render(&self, scene: &SceneGraph, gpu: &GpuResources) -> Result<(), Error> {
        let scale = self.dpi_scale;
        let phys_w = self.width as f32 * scale;
        let phys_h = self.height as f32 * scale;

        // Release current target (Present invalidates back buffer reference)
        self.dc.SetTarget(None);

        // Get new back buffer reference
        let surface: IDXGISurface = self
            .swapchain
            .GetBuffer(0)
            .map_err(|e| Error::RenderError(format!("GetBuffer: {e}")))?;

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * scale,
            dpiY: 96.0 * scale,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };

        let bitmap = self
            .dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))
            .map_err(|e| Error::RenderError(format!("CreateBitmapFromDxgiSurface: {e}")))?;
        self.dc.SetTarget(&bitmap);

        // Draw
        self.dc.BeginDraw();
        self.dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        for (_key, element) in scene.iter() {
            match element {
                Element::Rect(elem) => self.render_rect(elem, scale)?,
                Element::Text(elem) => self.render_text(elem, gpu, scale, phys_w, phys_h)?,
                Element::Image(elem) => self.render_image(elem, scale)?,
            }
        }

        self.dc
            .EndDraw(None, None)
            .map_err(|e| Error::RenderError(format!("EndDraw: {e}")))?;

        self.swapchain
            .Present(1, DXGI_PRESENT(0))
            .ok()
            .map_err(|e| Error::RenderError(format!("Present: {e}")))?;

        self.dcomp_device
            .Commit()
            .map_err(|e| Error::RenderError(format!("DComposition commit: {e}")))?;

        Ok(())
    }

    unsafe fn render_rect(&self, elem: &RectElement, scale: f32) -> Result<(), Error> {
        let left = elem.x * scale;
        let top = elem.y * scale;
        let right = (elem.x + elem.width) * scale;
        let bottom = (elem.y + elem.height) * scale;
        let rect = D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        };

        let fill_brush = self
            .dc
            .CreateSolidColorBrush(&elem.fill.to_d2d_premultiplied(), None)
            .map_err(|e| Error::RenderError(format!("CreateSolidColorBrush: {e}")))?;

        if elem.corner_radius > 0.0 {
            let rr = D2D1_ROUNDED_RECT {
                rect,
                radiusX: elem.corner_radius * scale,
                radiusY: elem.corner_radius * scale,
            };
            self.dc.FillRoundedRectangle(&rr, &fill_brush);

            if let Some(bc) = &elem.border_color {
                let border_brush = self
                    .dc
                    .CreateSolidColorBrush(&bc.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("border brush: {e}")))?;
                self.dc
                    .DrawRoundedRectangle(&rr, &border_brush, elem.border_width * scale, None);
            }
        } else {
            self.dc.FillRectangle(&rect, &fill_brush);

            if let Some(bc) = &elem.border_color {
                let border_brush = self
                    .dc
                    .CreateSolidColorBrush(&bc.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("border brush: {e}")))?;
                self.dc
                    .DrawRectangle(&rect, &border_brush, elem.border_width * scale, None);
            }
        }

        Ok(())
    }

    unsafe fn render_text(
        &self,
        elem: &TextElement,
        gpu: &GpuResources,
        scale: f32,
        surface_width: f32,
        surface_height: f32,
    ) -> Result<(), Error> {
        let font_family = elem.font_family.as_deref().unwrap_or("Segoe UI");
        let font_family_hstring = HSTRING::from(font_family);
        let weight = if elem.bold {
            DWRITE_FONT_WEIGHT_BOLD
        } else {
            DWRITE_FONT_WEIGHT_REGULAR
        };
        let style = if elem.italic {
            DWRITE_FONT_STYLE_ITALIC
        } else {
            DWRITE_FONT_STYLE_NORMAL
        };

        let format = gpu
            .dwrite_factory
            .CreateTextFormat(
                &font_family_hstring,
                None,
                weight,
                style,
                DWRITE_FONT_STRETCH_NORMAL,
                elem.font_size * scale,
                w!("en-us"),
            )
            .map_err(|e| Error::RenderError(format!("CreateTextFormat: {e}")))?;

        let text_utf16: Vec<u16> = elem.text.encode_utf16().collect();
        let brush = self
            .dc
            .CreateSolidColorBrush(&elem.color.to_d2d_premultiplied(), None)
            .map_err(|e| Error::RenderError(format!("text brush: {e}")))?;

        let layout_rect = D2D_RECT_F {
            left: elem.x * scale,
            top: elem.y * scale,
            right: surface_width,
            bottom: surface_height,
        };

        self.dc.DrawText(
            &text_utf16,
            &format,
            &layout_rect as *const D2D_RECT_F,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );

        Ok(())
    }

    unsafe fn render_image(&self, elem: &ImageElement, scale: f32) -> Result<(), Error> {
        let bgra_data = rgba_to_bgra(&elem.data);

        let bitmap = self
            .dc
            .CreateBitmap(
                D2D_SIZE_U {
                    width: elem.data_width,
                    height: elem.data_height,
                },
                Some(bgra_data.as_ptr() as *const c_void),
                elem.data_width * 4,
                &D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                    ..Default::default()
                },
            )
            .map_err(|e| Error::RenderError(format!("CreateBitmap: {e}")))?;

        let dest_rect = D2D_RECT_F {
            left: elem.x * scale,
            top: elem.y * scale,
            right: (elem.x + elem.width) * scale,
            bottom: (elem.y + elem.height) * scale,
        };

        self.dc.DrawBitmap(
            &bitmap,
            Some(&dest_rect as *const D2D_RECT_F),
            1.0,
            D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
            None,
            None,
        );

        Ok(())
    }

    pub unsafe fn resize(
        &mut self,
        gpu: &GpuResources,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        self.width = width;
        self.height = height;
        let phys_w = (width as f32 * self.dpi_scale) as u32;
        let phys_h = (height as f32 * self.dpi_scale) as u32;

        // Release reference to back buffer
        self.dc.SetTarget(None);

        self.swapchain
            .ResizeBuffers(2, phys_w, phys_h, DXGI_FORMAT_B8G8R8A8_UNORM, 0)
            .map_err(|e| Error::RenderError(format!("ResizeBuffers: {e}")))?;

        // Recreate bitmap target from new back buffer
        let surface: IDXGISurface = self
            .swapchain
            .GetBuffer(0)
            .map_err(|e| Error::RenderError(format!("GetBuffer after resize: {e}")))?;

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * self.dpi_scale,
            dpiY: 96.0 * self.dpi_scale,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };

        let bitmap = self
            .dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))
            .map_err(|e| Error::RenderError(format!("bitmap after resize: {e}")))?;
        self.dc.SetTarget(&bitmap);

        let _ = SetWindowPos(
            self.hwnd,
            None,
            0,
            0,
            phys_w as i32,
            phys_h as i32,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );

        // Recreate DComp binding is not needed since the visual still references the same swap chain.
        // But we need to notify DComp about the size change.
        let _ = gpu; // gpu not needed for resize beyond bitmap recreation

        Ok(())
    }

    pub unsafe fn set_opacity(&self, opacity: f32) -> Result<(), Error> {
        let effect_group: IDCompositionEffectGroup = self
            .dcomp_device
            .CreateEffectGroup()
            .map_err(|e| Error::RenderError(format!("CreateEffectGroup: {e}")))?;
        effect_group
            .SetOpacity2(opacity)
            .map_err(|e| Error::RenderError(format!("SetOpacity2: {e}")))?;
        self.dcomp_visual
            .SetEffect(&effect_group)
            .map_err(|e| Error::RenderError(format!("SetEffect: {e}")))?;
        self.dcomp_device
            .Commit()
            .map_err(|e| Error::RenderError(format!("DComposition commit: {e}")))?;
        Ok(())
    }

    /// Execute a batch of custom draw operations.
    /// Performs a full BeginDraw/EndDraw/Present cycle, rendering the scene
    /// graph first (if any), then the custom ops on top.
    pub unsafe fn execute_draw_ops(
        &self,
        scene: &SceneGraph,
        gpu: &GpuResources,
        ops: &[DrawOp],
    ) -> Result<(), Error> {
        let scale = self.dpi_scale;
        let phys_w = self.width as f32 * scale;
        let phys_h = self.height as f32 * scale;

        // Release current target
        self.dc.SetTarget(None);

        // Get new back buffer reference
        let surface: IDXGISurface = self
            .swapchain
            .GetBuffer(0)
            .map_err(|e| Error::RenderError(format!("GetBuffer: {e}")))?;

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * scale,
            dpiY: 96.0 * scale,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };

        let bitmap = self
            .dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))
            .map_err(|e| Error::RenderError(format!("CreateBitmapFromDxgiSurface: {e}")))?;
        self.dc.SetTarget(&bitmap);

        // Begin drawing
        self.dc.BeginDraw();
        self.dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        // Render retained-mode scene graph first (base layer)
        for (_key, element) in scene.iter() {
            match element {
                Element::Rect(elem) => self.render_rect(elem, scale)?,
                Element::Text(elem) => self.render_text(elem, gpu, scale, phys_w, phys_h)?,
                Element::Image(elem) => self.render_image(elem, scale)?,
            }
        }

        // Execute custom draw ops on top
        for op in ops {
            self.execute_single_draw_op(op, gpu, scale, phys_w, phys_h)?;
        }

        self.dc
            .EndDraw(None, None)
            .map_err(|e| Error::RenderError(format!("EndDraw: {e}")))?;

        self.swapchain
            .Present(1, DXGI_PRESENT(0))
            .ok()
            .map_err(|e| Error::RenderError(format!("Present: {e}")))?;

        self.dcomp_device
            .Commit()
            .map_err(|e| Error::RenderError(format!("DComposition commit: {e}")))?;

        Ok(())
    }

    /// Execute a single DrawOp against the active D2D context.
    /// Must be called between BeginDraw and EndDraw.
    unsafe fn execute_single_draw_op(
        &self,
        op: &DrawOp,
        gpu: &GpuResources,
        scale: f32,
        surface_width: f32,
        surface_height: f32,
    ) -> Result<(), Error> {
        match op {
            DrawOp::Clear(color) => {
                self.dc.Clear(Some(&color.to_d2d_premultiplied()));
            }
            DrawOp::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => {
                let rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                self.dc.FillRectangle(&rect, &brush);
            }
            DrawOp::StrokeRect {
                x,
                y,
                width,
                height,
                color,
                stroke_width,
            } => {
                let rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                self.dc
                    .DrawRectangle(&rect, &brush, stroke_width * scale, None);
            }
            DrawOp::DrawText {
                x,
                y,
                text,
                font_size,
                color,
            } => {
                let format = gpu
                    .dwrite_factory
                    .CreateTextFormat(
                        w!("Segoe UI"),
                        None,
                        DWRITE_FONT_WEIGHT_REGULAR,
                        DWRITE_FONT_STYLE_NORMAL,
                        DWRITE_FONT_STRETCH_NORMAL,
                        font_size * scale,
                        w!("en-us"),
                    )
                    .map_err(|e| Error::RenderError(format!("CreateTextFormat: {e}")))?;

                let text_utf16: Vec<u16> = text.encode_utf16().collect();
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;

                let layout_rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: surface_width,
                    bottom: surface_height,
                };

                self.dc.DrawText(
                    &text_utf16,
                    &format,
                    &layout_rect as *const D2D_RECT_F,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            DrawOp::DrawLine {
                x1,
                y1,
                x2,
                y2,
                color,
                stroke_width,
            } => {
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                // windows-rs 0.62: D2D_POINT_2F replaced by windows_numerics::Vector2
                let p0 = windows_numerics::Vector2 {
                    X: x1 * scale,
                    Y: y1 * scale,
                };
                let p1 = windows_numerics::Vector2 {
                    X: x2 * scale,
                    Y: y2 * scale,
                };
                self.dc.DrawLine(p0, p1, &brush, stroke_width * scale, None);
            }
            DrawOp::FillEllipse {
                cx,
                cy,
                rx,
                ry,
                color,
            } => {
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let ellipse = D2D1_ELLIPSE {
                    point: windows_numerics::Vector2 {
                        X: cx * scale,
                        Y: cy * scale,
                    },
                    radiusX: rx * scale,
                    radiusY: ry * scale,
                };
                self.dc.FillEllipse(&ellipse, &brush);
            }
            DrawOp::StrokeEllipse {
                cx,
                cy,
                rx,
                ry,
                color,
                stroke_width,
            } => {
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let ellipse = D2D1_ELLIPSE {
                    point: windows_numerics::Vector2 {
                        X: cx * scale,
                        Y: cy * scale,
                    },
                    radiusX: rx * scale,
                    radiusY: ry * scale,
                };
                self.dc
                    .DrawEllipse(&ellipse, &brush, stroke_width * scale, None);
            }
            DrawOp::DrawImage {
                x,
                y,
                width,
                height,
                rgba,
                img_width,
                img_height,
            } => {
                let bgra_data = rgba_to_bgra(rgba);
                let bmp = self
                    .dc
                    .CreateBitmap(
                        D2D_SIZE_U {
                            width: *img_width,
                            height: *img_height,
                        },
                        Some(bgra_data.as_ptr() as *const c_void),
                        *img_width * 4,
                        &D2D1_BITMAP_PROPERTIES1 {
                            pixelFormat: D2D1_PIXEL_FORMAT {
                                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                            },
                            dpiX: 96.0,
                            dpiY: 96.0,
                            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                            ..Default::default()
                        },
                    )
                    .map_err(|e| Error::RenderError(format!("CreateBitmap: {e}")))?;

                let dest = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                self.dc.DrawBitmap(
                    &bmp,
                    Some(&dest as *const D2D_RECT_F),
                    1.0,
                    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                    None,
                    None,
                );
            }
            DrawOp::FillRoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => {
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: x * scale,
                        top: y * scale,
                        right: (x + width) * scale,
                        bottom: (y + height) * scale,
                    },
                    radiusX: radius * scale,
                    radiusY: radius * scale,
                };
                self.dc.FillRoundedRectangle(&rr, &brush);
            }
            DrawOp::StrokeRoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
                stroke_width,
            } => {
                let brush = self
                    .dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: x * scale,
                        top: y * scale,
                        right: (x + width) * scale,
                        bottom: (y + height) * scale,
                    },
                    radiusX: radius * scale,
                    radiusY: radius * scale,
                };
                self.dc
                    .DrawRoundedRectangle(&rr, &brush, stroke_width * scale, None);
            }
        }
        Ok(())
    }
}

// --- RGBA to BGRA conversion ---

fn rgba_to_bgra(data: &[u8]) -> Vec<u8> {
    let mut bgra = data.to_vec();
    for chunk in bgra.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
    bgra
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_to_bgra_swaps_channels() {
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 128, // green, 50% alpha
            0, 0, 255, 255, // blue
        ];
        let bgra = rgba_to_bgra(&rgba);
        assert_eq!(
            bgra,
            vec![
                0, 0, 255, 255, // red -> B=0, G=0, R=255
                0, 255, 0, 128, // green unchanged (G stays in middle)
                255, 0, 0, 255, // blue -> B=255, G=0, R=0
            ]
        );
    }
}
