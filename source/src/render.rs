//! Direct2D/DirectWrite drawing through a DirectComposition swap chain.
//!
//! The shell window is created with `WS_EX_NOREDIRECTIONBITMAP`, so nothing the user sees
//! comes from GDI: every frame is drawn into a D3D11 swap chain back buffer through a D2D
//! device context and composited by DirectComposition with per-pixel alpha. That is what
//! removes the 1-bit `SetWindowRgn` clip (and with it the jagged capsule edge) and the
//! `WM_PAINT` round trip.
//!
//! Device loss is expected, not exceptional: a driver update or a TDR can remove the D3D11
//! device at any time. `end()` therefore reports failure for any draw or present error and
//! the shell drops this painter; the next frame rebuilds the whole stack from scratch.

use std::collections::BTreeMap;
use windows::core::{w, Interface, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::DirectWrite::*;
pub use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

pub const BG:u32=0x000000;
#[allow(dead_code)] pub const CARD:u32=0x12141a;
pub const TEXT:u32=0xffffff;
pub const MUTED:u32=0x9ca3af;
pub const ACCENT:u32=0x2fd4a3;
pub const DIM:u32=0x1e2129;
/// Interactive surfaces. Kept next to DIM so the two states read as one family.
pub const HOVER:u32=0x2a2f3a;
/// Pressed / active. Matches UI-DESIGN-DIRECTION spec — slightly lighter than DIM so the
/// "down" state reads as tactile without looking broken.
pub const PRESSED:u32=0x363d4a;

pub struct Painter {
    d2d:ID2D1Factory1,
    _device:ID3D11Device,
    _immediate:ID3D11DeviceContext,
    swap_chain:IDXGISwapChain3,
    _composition:IDCompositionDevice,
    _comp_target:IDCompositionTarget,
    _visual:IDCompositionVisual,
    context:ID2D1DeviceContext,
    artwork:Option<ID2D1Bitmap1>,artwork_key:u64,
    factory:IDWriteFactory,
    brush:ID2D1SolidColorBrush,
    formats:BTreeMap<(u32,bool),IDWriteTextFormat>,
    weather_gradient:Option<(u32,u32,ID2D1LinearGradientBrush)>,
    // Outer None = not loaded; Some(None) = failed, do not retry on every paint.
    weather_icons:[Option<Option<ID2D1Bitmap1>>;2],
    layers:u32,
    /// One layer per nesting depth. A layer object cannot be pushed twice at the same time,
    /// and the opacity scene now wraps the morph cross-fade, so pushes nest up to four deep.
    layer_pool:Vec<Option<ID2D1Layer>>,
}
fn color(rgb:u32,alpha:f32)->D2D1_COLOR_F{D2D1_COLOR_F{r:((rgb>>16)&255)as f32/255.,g:((rgb>>8)&255)as f32/255.,b:(rgb&255)as f32/255.,a:alpha}}
fn lerp_color(rgb:u32,factor:f32,alpha:f32)->D2D1_COLOR_F{
    let f=factor.clamp(0.0,1.0);
    D2D1_COLOR_F{
        r:(((rgb>>16)&255)as f32/255.)*f,
        g:(((rgb>>8)&255)as f32/255.)*f,
        b:((rgb&255)as f32/255.)*f,
        a:alpha,
    }
}
pub fn rect(x:f32,y:f32,width:f32,height:f32)->D2D_RECT_F{D2D_RECT_F{left:x,top:y,right:x+width,bottom:y+height}}
#[inline]
fn pt(x: f32, y: f32) -> windows_numerics::Vector2 {
    windows_numerics::Vector2 { X: x, Y: y }
}

impl Painter {
    pub unsafe fn new(hwnd:HWND,width:u32,height:u32,dpi:f32,software:bool)->Result<Self>{unsafe{
        let mut device:Option<ID3D11Device>=None;
        let mut immediate:Option<ID3D11DeviceContext>=None;
        let mut level=D3D_FEATURE_LEVEL_11_0;
        let preferred=if software || std::env::args().any(|arg|arg=="--software-renderer"){D3D_DRIVER_TYPE_WARP}else{D3D_DRIVER_TYPE_HARDWARE};
        let first=D3D11CreateDevice(None::<&IDXGIAdapter>,preferred,Default::default(),D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0,D3D_FEATURE_LEVEL_10_1,D3D_FEATURE_LEVEL_10_0]),D3D11_SDK_VERSION,
            Some(&mut device),Some(&mut level),Some(&mut immediate));
        if first.is_err(){device=None;immediate=None;D3D11CreateDevice(None::<&IDXGIAdapter>,D3D_DRIVER_TYPE_WARP,Default::default(),D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0,D3D_FEATURE_LEVEL_10_1,D3D_FEATURE_LEVEL_10_0]),D3D11_SDK_VERSION,
            Some(&mut device),Some(&mut level),Some(&mut immediate))?;}
        let device=match device{Some(d)=>d,None=>return Err(windows::core::Error::from(windows::core::HRESULT(0x80004005u32 as i32)))};
        let immediate=match immediate{Some(d)=>d,None=>return Err(windows::core::Error::from(windows::core::HRESULT(0x80004005u32 as i32)))};
        let dxgi_device:IDXGIDevice=device.cast()?;
        let adapter=dxgi_device.GetAdapter()?;
        let factory:IDXGIFactory2=adapter.GetParent()?;
        let description=DXGI_SWAP_CHAIN_DESC1{
            Width:width.max(1),Height:height.max(1),Format:DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo:windows::core::BOOL(0),SampleDesc:DXGI_SAMPLE_DESC{Count:1,Quality:0},
            BufferUsage:DXGI_USAGE_RENDER_TARGET_OUTPUT,BufferCount:2,
            Scaling:DXGI_SCALING_STRETCH,SwapEffect:DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode:DXGI_ALPHA_MODE_PREMULTIPLIED,Flags:0,
        };
        let swap_chain:IDXGISwapChain3=factory.CreateSwapChainForComposition(&device,&description,None)?.cast()?;
        let composition:IDCompositionDevice=DCompositionCreateDevice(&dxgi_device)?;
        let comp_target=composition.CreateTargetForHwnd(hwnd,true)?;
        let visual=composition.CreateVisual()?;
        visual.SetContent(&swap_chain)?;
        comp_target.SetRoot(&visual)?;
        composition.Commit()?;
        let d2d:ID2D1Factory1=D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,None)?;
        let d2d_device=d2d.CreateDevice(&dxgi_device)?;
        let context=d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        context.SetDpi(dpi,dpi);
        let brush=context.CreateSolidColorBrush(&color(TEXT,1.),None)?;
        let factory:IDWriteFactory=DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        Ok(Self{d2d,_device:device,_immediate:immediate,swap_chain,_composition:composition,_comp_target:comp_target,_visual:visual,context,artwork:None,artwork_key:0,factory,brush,formats:BTreeMap::new(),weather_gradient:None,weather_icons:[None,None],layers:0,layer_pool:vec![None;8]})
    }}
    /// Binds the current back buffer as the draw target. The bitmap is rebuilt every frame on
    /// purpose: a cached D2D bitmap wrapping a swap chain buffer goes stale once that buffer
    /// has been presented, and drawing into it then silently produces nothing — the window
    /// shows whatever the stale buffer happened to hold, which reads as random transparency.
    /// Recreating costs one small allocation per frame; the pacing spike measured 2835 fps
    /// doing exactly this, so it is not the bottleneck.
    unsafe fn bind(&mut self)->Result<()>{unsafe{
        let index=(self.swap_chain.GetCurrentBackBufferIndex() as usize).min(1);
        let surface:IDXGISurface=self.swap_chain.GetBuffer(index as u32)?;
        let properties=D2D1_BITMAP_PROPERTIES1{
            pixelFormat:D2D1_PIXEL_FORMAT{format:DXGI_FORMAT_B8G8R8A8_UNORM,alphaMode:D2D1_ALPHA_MODE_PREMULTIPLIED},
            dpiX:96.,dpiY:96.,bitmapOptions:D2D1_BITMAP_OPTIONS_TARGET|D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            colorContext:std::mem::ManuallyDrop::new(None),
        };
        let bitmap=self.context.CreateBitmapFromDxgiSurface(&surface,Some(&properties))?;
        self.context.SetTarget(&bitmap);
        Ok(())
    }}
    pub unsafe fn resize(&mut self,width:u32,height:u32,dpi:f32){unsafe{
        // Must release target reference before resizing DXGI buffers, otherwise
        // ResizeBuffers fails with DXGI_ERROR_INVALID_CALL.
        self.context.SetTarget(None);
        let _=self.swap_chain.ResizeBuffers(0,width.max(1),height.max(1),DXGI_FORMAT_B8G8R8A8_UNORM,DXGI_SWAP_CHAIN_FLAG(0));
        self.context.SetDpi(dpi,dpi);
    }}
    /// Clears to transparent: the capsule silhouette is drawn by the shell, not clipped by a
    /// region, so everything outside it stays see-through.
    pub unsafe fn begin(&mut self)->Result<()>{unsafe{self.bind()?;self.context.BeginDraw();self.context.Clear(None);Ok(())}}
    /// Ends the draw and presents. Any error here means the device is gone or broken; the
    /// caller drops the painter and the next frame rebuilds it.
    pub unsafe fn end(&mut self)->Result<()>{unsafe{
        self.context.EndDraw(None,None)?;
        self.context.SetTarget(None);
        self.swap_chain.Present(1,DXGI_PRESENT(0)).ok()
    }}
    pub unsafe fn fill(&self,x:f32,y:f32,width:f32,height:f32,radius:f32,rgb:u32){unsafe{
        self.brush.SetColor(&color(rgb,1.));
        self.context.FillRoundedRectangle(&D2D1_ROUNDED_RECT{rect:rect(x,y,width,height),radiusX:radius,radiusY:radius},&self.brush);
    }}
    pub unsafe fn fill_alpha(&self,x:f32,y:f32,width:f32,height:f32,radius:f32,rgb:u32,alpha:f32){unsafe{
        self.brush.SetColor(&color(rgb,alpha));
        self.context.FillRoundedRectangle(&D2D1_ROUNDED_RECT{rect:rect(x,y,width,height),radiusX:radius,radiusY:radius},&self.brush);
    }}
    /// One device-owned brush, rebuilt only when weather/daylight changes. No bitmap or blur.
    pub unsafe fn weather_background(&mut self,width:f32,height:f32,mid:u32,bottom:u32){unsafe{
        if self.weather_gradient.as_ref().is_none_or(|(m,b,_)|*m!=mid||*b!=bottom){
            let stops=[D2D1_GRADIENT_STOP{position:0.,color:color(BG,1.)},D2D1_GRADIENT_STOP{position:0.28,color:color(mid,1.)},D2D1_GRADIENT_STOP{position:1.,color:color(bottom,1.)}];
            let rt:Result<ID2D1RenderTarget>=self.context.cast();
            self.weather_gradient=rt.and_then(|rt|rt.CreateGradientStopCollection(&stops,D2D1_GAMMA_2_2,D2D1_EXTEND_MODE_CLAMP).and_then(|stops|rt.CreateLinearGradientBrush(&D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES{startPoint:pt(0.,44.),endPoint:pt(0.,height)},None,&stops))).ok().map(|brush|(mid,bottom,brush));
        }
        if let Some((_,_,brush))=&self.weather_gradient{
            brush.SetStartPoint(pt(0.,44.));brush.SetEndPoint(pt(0.,height.max(45.)));
            self.context.FillRoundedRectangle(&D2D1_ROUNDED_RECT{rect:rect(0.,0.,width,height),radiusX:24.,radiusY:24.},brush);
        }
    }}
    pub unsafe fn text_light(&mut self,text:&str,x:f32,y:f32,width:f32,height:f32,size:f32,rgb:u32){unsafe{
        // Reserve the high bit for the light weight; ordinary text keys remain unchanged.
        let key=(((size*10.)as u32)|(1<<31),false);
        if !self.formats.contains_key(&key){if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,DWRITE_FONT_WEIGHT_LIGHT,DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){self.formats.insert(key,format);}}
        if let Some(format)=self.formats.get(&key){let text:Vec<u16>=text.encode_utf16().collect();self.brush.SetColor(&color(rgb,1.));self.context.DrawText(&text,format,&rect(x,y,width,height),&self.brush,D2D1_DRAW_TEXT_OPTIONS_CLIP,DWRITE_MEASURING_MODE_NATURAL);}
    }}
    /// Smooth physical notch contour with vector concave ear fillets and rounded bottom corners.
    pub unsafe fn fill_notch(&self,width:f32,height:f32,ear:f32,radius:f32,rgb:u32){unsafe{
        self.fill_notch_alpha(width,height,ear,radius,rgb,1.);
    }}
    pub unsafe fn fill_notch_alpha(&self,width:f32,height:f32,ear:f32,radius:f32,rgb:u32,alpha:f32){unsafe{
        let mut drawn=false;
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(-ear,0.),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(-ear*0.5,0.),point2:pt(0.,ear*0.5),point3:pt(0.,ear)});
                sink.AddLine(pt(0.,height-radius));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(0.,height-radius*0.45),point2:pt(radius*0.45,height),point3:pt(radius,height)});
                sink.AddLine(pt(width-radius,height));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width-radius*0.45,height),point2:pt(width,height-radius*0.45),point3:pt(width,height-radius)});
                sink.AddLine(pt(width,ear));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width,ear*0.5),point2:pt(width+ear*0.5,0.),point3:pt(width+ear,0.)});
                sink.AddLine(pt(-ear,0.));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                self.brush.SetColor(&color(rgb,alpha));
                self.context.FillGeometry(&path,&self.brush,None);
                drawn=true;
            }
        }
        if !drawn{
            self.fill_alpha(0.,0.,width,height,radius,rgb,alpha);
        }
    }}
    pub unsafe fn fill_notch_gradient(&self,width:f32,height:f32,ear:f32,radius:f32,top_rgb:u32,bottom_rgb:u32,top_alpha:f32,bottom_alpha:f32){unsafe{
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(-ear,0.),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(-ear*0.5,0.),point2:pt(0.,ear*0.5),point3:pt(0.,ear)});
                sink.AddLine(pt(0.,height-radius));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(0.,height-radius*0.45),point2:pt(radius*0.45,height),point3:pt(radius,height)});
                sink.AddLine(pt(width-radius,height));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width-radius*0.45,height),point2:pt(width,height-radius*0.45),point3:pt(width,height-radius)});
                sink.AddLine(pt(width,ear));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width,ear*0.5),point2:pt(width+ear*0.5,0.),point3:pt(width+ear,0.)});
                sink.AddLine(pt(-ear,0.));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                let stops=[
                    D2D1_GRADIENT_STOP{position:0.0,color:color(top_rgb,top_alpha)},
                    D2D1_GRADIENT_STOP{position:0.55,color:color(bottom_rgb,bottom_alpha*0.35)},
                    D2D1_GRADIENT_STOP{position:1.0,color:color(bottom_rgb,bottom_alpha)},
                ];
                let rt:Result<ID2D1RenderTarget>=self.context.cast();
                if let Ok(rt)=rt{
                    if let Ok(collection)=rt.CreateGradientStopCollection(&stops,D2D1_GAMMA_2_2,D2D1_EXTEND_MODE_CLAMP){
                        if let Ok(brush)=rt.CreateLinearGradientBrush(
                            &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES{
                                startPoint:windows_numerics::Vector2{X:width*0.5,Y:0.},
                                endPoint:windows_numerics::Vector2{X:width*0.5,Y:height},
                            },
                            None,
                            &collection,
                        ){
                            self.context.FillGeometry(&path,&brush,None);
                        }
                    }
                }
            }
        }
    }}
    #[allow(dead_code)] pub unsafe fn fill_notch_radial_gradient(&self,width:f32,height:f32,ear:f32,radius:f32,cx:f32,cy:f32,rx:f32,ry:f32,rgb:u32,inner_alpha:f32,outer_alpha:f32){unsafe{
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(-ear,0.),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(-ear*0.5,0.),point2:pt(0.,ear*0.5),point3:pt(0.,ear)});
                sink.AddLine(pt(0.,height-radius));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(0.,height-radius*0.45),point2:pt(radius*0.45,height),point3:pt(radius,height)});
                sink.AddLine(pt(width-radius,height));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width-radius*0.45,height),point2:pt(width,height-radius*0.45),point3:pt(width,height-radius)});
                sink.AddLine(pt(width,ear));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width,ear*0.5),point2:pt(width+ear*0.5,0.),point3:pt(width+ear,0.)});
                sink.AddLine(pt(-ear,0.));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                let stops=[
                    D2D1_GRADIENT_STOP{position:0.0,color:color(rgb,inner_alpha)},
                    D2D1_GRADIENT_STOP{position:0.55,color:color(rgb,inner_alpha*0.4)},
                    D2D1_GRADIENT_STOP{position:1.0,color:color(rgb,outer_alpha)},
                ];
                let rt:Result<ID2D1RenderTarget>=self.context.cast();
                if let Ok(rt)=rt{
                    if let Ok(collection)=rt.CreateGradientStopCollection(&stops,D2D1_GAMMA_2_2,D2D1_EXTEND_MODE_CLAMP){
                        if let Ok(brush)=rt.CreateRadialGradientBrush(
                            &D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES{
                                center:windows_numerics::Vector2{X:cx,Y:cy},
                                gradientOriginOffset:windows_numerics::Vector2{X:0.,Y:0.},
                                radiusX:rx,
                                radiusY:ry,
                            },
                            None,
                            &collection,
                        ){
                            self.context.FillGeometry(&path,&brush,None);
                        }
                    }
                }
            }
        }
    }}
    pub unsafe fn draw_triangle(&self,p1:(f32,f32),p2:(f32,f32),p3:(f32,f32),rgb:u32){unsafe{
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(p1.0,p1.1),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(pt(p2.0,p2.1));
                sink.AddLine(pt(p3.0,p3.1));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                self.brush.SetColor(&color(rgb,1.));
                self.context.FillGeometry(&path,&self.brush,None);
            }
        }
    }}
    pub unsafe fn icon_chevron_up(&self,cx:f32,cy:f32,size:f32,rgb:u32,thickness:f32){unsafe{
        let half_w=size*0.5;
        let half_h=size*0.3;
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(cx-half_w,cy+half_h),D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddLine(pt(cx,cy-half_h));
                sink.AddLine(pt(cx+half_w,cy+half_h));
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
                let _=sink.Close();
                self.brush.SetColor(&color(rgb,1.));
                self.context.DrawGeometry(&path,&self.brush,thickness,None);
            }
        }
    }}
    pub unsafe fn icon_prev(&self,cx:f32,cy:f32,size:f32,rgb:u32){unsafe{
        let bw=2.;let bh=size*0.75;
        self.fill(cx-size*0.45,cy-bh*0.5,bw,bh,1.,rgb);
        self.draw_triangle((cx+size*0.45,cy-size*0.42),(cx-size*0.15,cy),(cx+size*0.45,cy+size*0.42),rgb);
    }}
    pub unsafe fn icon_next(&self,cx:f32,cy:f32,size:f32,rgb:u32){unsafe{
        let bw=2.;let bh=size*0.75;
        self.draw_triangle((cx-size*0.45,cy-size*0.42),(cx+size*0.15,cy),(cx-size*0.45,cy+size*0.42),rgb);
        self.fill(cx+size*0.30,cy-bh*0.5,bw,bh,1.,rgb);
    }}
    pub unsafe fn icon_play(&self,cx:f32,cy:f32,size:f32,rgb:u32){unsafe{
        self.draw_triangle((cx-size*0.35,cy-size*0.48),(cx+size*0.48,cy),(cx-size*0.35,cy+size*0.48),rgb);
    }}
    pub unsafe fn icon_pause(&self,cx:f32,cy:f32,size:f32,rgb:u32){unsafe{
        let bw=3.;let bh=size*0.82;let gap=2.5;
        self.fill(cx-gap-bw*0.5,cy-bh*0.5,bw,bh,1.,rgb);
        self.fill(cx+gap-bw*0.5,cy-bh*0.5,bw,bh,1.,rgb);
    }}
    pub unsafe fn icon_lyrics_bubble(&self,cx:f32,cy:f32,rgb:u32,active:bool){unsafe{
        let left=cx-8.5;let top=cy-7.0;let right=cx+8.5;let bottom=cy+4.5;let r=3.5;
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(left+r,top),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(pt(right-r,top));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(right-r*0.45,top),point2:pt(right,top+r*0.45),point3:pt(right,top+r)});
                sink.AddLine(pt(right,bottom-r));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(right,bottom-r*0.45),point2:pt(right-r*0.45,bottom),point3:pt(right-r,bottom)});
                sink.AddLine(pt(left+6.0,bottom));
                sink.AddLine(pt(left+1.5,bottom+4.0));
                sink.AddLine(pt(left+3.2,bottom));
                sink.AddLine(pt(left+r,bottom));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(left+r*0.45,bottom),point2:pt(left,bottom-r*0.45),point3:pt(left,bottom-r)});
                sink.AddLine(pt(left,top+r));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(left,top+r*0.45),point2:pt(left+r*0.45,top),point3:pt(left+r,top)});
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                if active{
                    self.brush.SetColor(&color(0xffffff,1.));
                    self.context.FillGeometry(&path,&self.brush,None);
                    self.fill(cx-3.2,cy-3.0,2.0,2.2,0.8,0x000000);
                    self.fill(cx-3.8,cy-1.2,1.5,1.8,0.5,0x000000);
                    self.fill(cx+1.6,cy-3.0,2.0,2.2,0.8,0x000000);
                    self.fill(cx+1.0,cy-1.2,1.5,1.8,0.5,0x000000);
                }else{
                    self.brush.SetColor(&color(rgb,0.92));
                    self.context.DrawGeometry(&path,&self.brush,1.4,None);
                    self.fill(cx-3.2,cy-3.0,2.0,2.2,0.8,rgb);
                    self.fill(cx-3.8,cy-1.2,1.5,1.8,0.5,rgb);
                    self.fill(cx+1.6,cy-3.0,2.0,2.2,0.8,rgb);
                    self.fill(cx+1.0,cy-1.2,1.5,1.8,0.5,rgb);
                }
            }
        }
    }}
    pub unsafe fn icon_media_app(&mut self,cx:f32,cy:f32,source_app:&str,playing:bool){unsafe{
        let app=source_app.to_lowercase();
        let r=8.0;let d=16.0;
        let left=cx-r;let top=cy-r;

        if app.contains("spotify"){
            // Spotify: green disc (#1db954) + 3 signature curved soundwave arcs
            self.fill(left,top,d,d,r,0x1db954);
            if let Ok(path)=self.d2d.CreatePathGeometry(){
                if let Ok(sink)=path.Open(){
                    sink.BeginFigure(pt(cx-4.4,cy-2.0),D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(cx-1.5,cy-3.7),point2:pt(cx+1.5,cy-3.7),point3:pt(cx+4.4,cy-1.8)});
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);

                    sink.BeginFigure(pt(cx-3.6,cy+0.6),D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(cx-1.2,cy-0.8),point2:pt(cx+1.2,cy-0.8),point3:pt(cx+3.6,cy+0.8)});
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);

                    sink.BeginFigure(pt(cx-2.8,cy+3.0),D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(cx-0.9,cy+1.9),point2:pt(cx+0.9,cy+1.9),point3:pt(cx+2.8,cy+3.2)});
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    let _=sink.Close();

                    self.brush.SetColor(&color(0x000000,1.0));
                    self.context.DrawGeometry(&path,&self.brush,1.4,None);
                }
            }
        }else if app.contains("chrome")||app.contains("google"){
            // Google Chrome: Red/Yellow/Green 3-color wheel with white ring & blue center
            self.fill(left,top,d,d,r,0x34a853);
            if let Ok(path)=self.d2d.CreatePathGeometry(){
                if let Ok(sink)=path.Open(){
                    sink.BeginFigure(pt(left,top),D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddLine(pt(cx+r,top));
                    sink.AddLine(pt(cx+r,cy-1.0));
                    sink.AddLine(pt(cx-2.5,cy-1.0));
                    sink.AddLine(pt(left,cy+2.0));
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _=sink.Close();
                    self.brush.SetColor(&color(0xea4335,1.0));
                    self.context.FillGeometry(&path,&self.brush,None);
                }
            }
            if let Ok(path)=self.d2d.CreatePathGeometry(){
                if let Ok(sink)=path.Open(){
                    sink.BeginFigure(pt(cx+r,cy-1.0),D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddLine(pt(cx+r,top+d));
                    sink.AddLine(pt(cx-1.0,top+d));
                    sink.AddLine(pt(cx-1.0,cy+2.5));
                    sink.AddLine(pt(cx+2.5,cy-1.0));
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _=sink.Close();
                    self.brush.SetColor(&color(0xfbbc05,1.0));
                    self.context.FillGeometry(&path,&self.brush,None);
                }
            }
            self.stroke(left+0.5,top+0.5,d-1.0,d-1.0,r-0.5,0x1b1b1b,0.8);
            self.fill(cx-4.2,cy-4.2,8.4,8.4,4.2,0xffffff);
            self.fill(cx-3.0,cy-3.0,6.0,6.0,3.0,0x4285f4);
        }else if app.contains("edge")||app.contains("msedge"){
            // Microsoft Edge: Blue & cyan wave swirl
            self.fill(left,top,d,d,r,0x0078d4);
            self.fill(cx-2.0,top+2.0,8.0,10.0,4.0,0x00c7bb);
            self.fill(cx-5.0,cy-1.0,7.0,7.0,3.5,0x25b5e8);
            self.fill(cx-2.5,cy-2.5,5.0,5.0,2.5,0xffffff);
        }else if app.contains("firefox")||app.contains("mozilla"){
            // Mozilla Firefox: Orange flame circle with purple/blue core
            self.fill(left,top,d,d,r,0xff7139);
            self.fill(cx-3.5,cy-3.5,7.0,7.0,3.5,0x902c8e);
            self.fill(cx-1.5,cy-1.5,3.0,3.0,1.5,0xffa400);
        }else if app.contains("apple")||app.contains("itunes"){
            // Apple Music: Pink/red badge with white note
            self.fill(left,top,d,d,4.0,0xfa2d48);
            self.text("♫",cx-4.5,cy-6.0,10.0,12.0,10.0,0xffffff,true);
        }else if app.contains("youtube")||app.contains("ytm"){
            // YouTube / YouTube Music: Red circle with white play triangle
            self.fill(left,top,d,d,r,0xff0000);
            if let Ok(path)=self.d2d.CreatePathGeometry(){
                if let Ok(sink)=path.Open(){
                    sink.BeginFigure(pt(cx-2.2,cy-3.6),D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddLine(pt(cx+3.4,cy));
                    sink.AddLine(pt(cx-2.2,cy+3.6));
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _=sink.Close();
                    self.brush.SetColor(&color(0xffffff,1.0));
                    self.context.FillGeometry(&path,&self.brush,None);
                }
            }
        }else{
            // Generic / Fallback audio player icon
            let bg=if playing{0x16342b}else{0x222a30};
            let fg=if playing{0x19ff58}else{0x8a96a0};
            self.fill(left,top,d,d,r,bg);
            self.text("♫",cx-4.5,cy-6.0,10.0,12.0,9.5,fg,true);
        }
    }}
    pub unsafe fn fill_notch_half_gradient(&self,width:f32,height:f32,ear:f32,radius:f32,accent:u32,strength:f32){unsafe{
        if let Ok(path)=self.d2d.CreatePathGeometry(){
            let closed=if let Ok(sink)=path.Open(){
                sink.BeginFigure(pt(-ear,0.),D2D1_FIGURE_BEGIN_FILLED);
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(-ear*0.5,0.),point2:pt(0.,ear*0.5),point3:pt(0.,ear)});
                sink.AddLine(pt(0.,height-radius));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(0.,height-radius*0.45),point2:pt(radius*0.45,height),point3:pt(radius,height)});
                sink.AddLine(pt(width-radius,height));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width-radius*0.45,height),point2:pt(width,height-radius*0.45),point3:pt(width,height-radius)});
                sink.AddLine(pt(width,ear));
                sink.AddBezier(&D2D1_BEZIER_SEGMENT{point1:pt(width,ear*0.5),point2:pt(width+ear*0.5,0.),point3:pt(width+ear,0.)});
                sink.AddLine(pt(-ear,0.));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().is_ok()
            }else{false};
            if closed{
                // First fill base geometry with 100% solid OLED black to guarantee no transparent gaps!
                self.brush.SetColor(&color(0x000000,1.0));
                self.context.FillGeometry(&path,&self.brush,None);

                // 7-step cubic feather: top 60% solid OLED black, smooth imperceptible departure into soft ambient glow
                let stops=[
                    D2D1_GRADIENT_STOP{position:0.00,color:color(0x000000,1.0)},
                    D2D1_GRADIENT_STOP{position:0.60,color:color(0x000000,1.0)},
                    D2D1_GRADIENT_STOP{position:0.72,color:lerp_color(accent,0.04*strength,1.0)},
                    D2D1_GRADIENT_STOP{position:0.82,color:lerp_color(accent,0.11*strength,1.0)},
                    D2D1_GRADIENT_STOP{position:0.90,color:lerp_color(accent,0.22*strength,1.0)},
                    D2D1_GRADIENT_STOP{position:0.96,color:lerp_color(accent,0.34*strength,1.0)},
                    D2D1_GRADIENT_STOP{position:1.00,color:lerp_color(accent,0.44*strength,1.0)},
                ];
                let rt:Result<ID2D1RenderTarget>=self.context.cast();
                if let Ok(rt)=rt{
                    if let Ok(collection)=rt.CreateGradientStopCollection(&stops,D2D1_GAMMA_2_2,D2D1_EXTEND_MODE_CLAMP){
                        if let Ok(brush)=rt.CreateLinearGradientBrush(
                            &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES{
                                startPoint:windows_numerics::Vector2{X:width*0.5,Y:0.},
                                endPoint:windows_numerics::Vector2{X:width*0.5,Y:height},
                            },
                            None,
                            &collection,
                        ){
                            self.context.FillGeometry(&path,&brush,None);
                        }
                    }
                }
            }
        }
    }}
    pub unsafe fn stroke(&self,x:f32,y:f32,width:f32,height:f32,radius:f32,rgb:u32,thickness:f32){unsafe{
        self.brush.SetColor(&color(rgb,1.));
        self.context.DrawRoundedRectangle(&D2D1_ROUNDED_RECT{rect:rect(x,y,width,height),radiusX:radius,radiusY:radius},&self.brush,thickness,None);
    }}
    /// A bounded, device-owned atlas per style, decoded only on its first visible use.
    pub unsafe fn weather_icon(&mut self,code:u32,is_day:bool,compact:bool,x:f32,y:f32,size:f32){unsafe{
        use crate::weather_icons::{Icon,Style};
        let index=usize::from(compact);let style=if compact{Style::Compact}else{Style::Gradient};
        if self.weather_icons[index].is_none(){
            self.weather_icons[index]=Some(crate::weather_icons::decode(style).and_then(|pixels|{
                let props=D2D1_BITMAP_PROPERTIES1{pixelFormat:D2D1_PIXEL_FORMAT{format:DXGI_FORMAT_B8G8R8A8_UNORM,alphaMode:D2D1_ALPHA_MODE_PREMULTIPLIED},dpiX:96.,dpiY:96.,bitmapOptions:D2D1_BITMAP_OPTIONS_NONE,colorContext:std::mem::ManuallyDrop::new(None)};
                self.context.CreateBitmapFromWicBitmap(&pixels,Some(&props))
            }).ok());
        }
        if let Some(Some(bitmap))=&self.weather_icons[index]{
            let cell=Icon::select(code,is_day)as u32;let tile=style.tile()as f32;
            let source=rect((cell%7)as f32*tile,(cell/7)as f32*tile,tile,tile);
            self.context.DrawBitmap(bitmap,Some(&rect(x,y,size,size)),1.,D2D1_INTERPOLATION_MODE_LINEAR,Some(&source),None);
        }else{self.outline_icon("cloud",x+size*0.1,y+size*0.1,size*0.8,0xd4e3ee);}
    }}
    pub fn clear_weather_icons(&mut self){self.weather_icons=[None,None];self.weather_gradient=None;}
    pub unsafe fn cover(&mut self,art:Option<&crate::artwork::Artwork>,key:u64,x:f32,y:f32,size:f32,glow:bool)->bool{unsafe{
        if self.artwork_key!=key{self.artwork=None;self.artwork_key=key;
            if let Some(art)=art{let props=D2D1_BITMAP_PROPERTIES1{pixelFormat:D2D1_PIXEL_FORMAT{format:DXGI_FORMAT_B8G8R8A8_UNORM,alphaMode:D2D1_ALPHA_MODE_PREMULTIPLIED},dpiX:96.,dpiY:96.,bitmapOptions:D2D1_BITMAP_OPTIONS_NONE,colorContext:std::mem::ManuallyDrop::new(None)};self.artwork=self.context.CreateBitmap(D2D_SIZE_U{width:art.width,height:art.height},Some(art.pixels.as_ptr().cast()),art.width*4,&props).ok();}}
        if let(Some(bitmap),Some(art))=(&self.artwork,art){
            if glow{for i in (1..=14).rev(){let spread=i as f32*1.6;self.brush.SetColor(&color(art.accent,0.032));self.context.FillRoundedRectangle(&D2D1_ROUNDED_RECT{rect:rect(x-spread,y-spread,size+spread*2.,size+spread*2.),radiusX:14.+spread,radiusY:14.+spread},&self.brush);}}
            let target=rect(x,y,size,size);
            let _=self.context.DrawBitmap(bitmap,Some(&target),1.,D2D1_INTERPOLATION_MODE_LINEAR,None,None);true
        }else{false}
    }}
    pub unsafe fn text(&mut self,text:&str,x:f32,y:f32,width:f32,height:f32,size:f32,rgb:u32,bold:bool){unsafe{
        let key=((size*10.)as u32,bold);
        if !self.formats.contains_key(&key){
            if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,if bold{DWRITE_FONT_WEIGHT_SEMI_BOLD}else{DWRITE_FONT_WEIGHT_NORMAL},DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){
                let _=format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);self.formats.insert(key,format);
            }
        }
        if let Some(format)=self.formats.get(&key){let text:Vec<u16>=text.encode_utf16().collect();self.brush.SetColor(&color(rgb,1.));self.context.DrawText(&text,format,&rect(x,y,width,height),&self.brush,D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,DWRITE_MEASURING_MODE_NATURAL);}
    }}
    /// Like `text` but clips overflowing content with an ellipsis ("…") instead of hard clipping.
    /// Uses a single-line IDWriteTextLayout with character-granularity trimming so long titles
    /// never get sliced mid-glyph. Falls back to the regular `text` path silently if the layout
    /// cannot be created (device-lost, out-of-memory, etc.).
    pub unsafe fn text_ellipsis(&mut self,text:&str,x:f32,y:f32,width:f32,height:f32,size:f32,rgb:u32,bold:bool){unsafe{
        let key=((size*10.)as u32,bold);
        if !self.formats.contains_key(&key){
            if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,if bold{DWRITE_FONT_WEIGHT_SEMI_BOLD}else{DWRITE_FONT_WEIGHT_NORMAL},DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){
                let _=format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);self.formats.insert(key,format);
            }
        }
        // Keep a reference to the format before borrowing self below.
        let has_format=self.formats.contains_key(&key);
        if !has_format{
            // Could not create format; fall back to regular clipped text.
            self.text(text,x,y,width,height,size,rgb,bold);
            return;
        }
        let text_utf16:Vec<u16>=text.encode_utf16().collect();
        let format=self.formats.get(&key).unwrap();
        // Build a single-line text layout so we can attach an ellipsis trimming sign.
        if let Ok(layout)=self.factory.CreateTextLayout(&text_utf16,format,width,height){
            let _=layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            let trimming=DWRITE_TRIMMING{
                granularity:DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter:0,
                delimiterCount:0,
            };
            if let Ok(sign)=self.factory.CreateEllipsisTrimmingSign(format){
                let _=layout.SetTrimming(&trimming,&sign);
            }
            self.brush.SetColor(&color(rgb,1.));
            let origin=windows_numerics::Vector2{X:x,Y:y};
            self.context.DrawTextLayout(origin,&layout,&self.brush,D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
        }else{
            // Layout creation failed; fall back to regular text.
            self.text(text,x,y,width,height,size,rgb,bold);
        }
    }}
    pub unsafe fn text_ellipsis_vcenter(&mut self,text:&str,x:f32,y:f32,width:f32,height:f32,size:f32,rgb:u32,bold:bool){unsafe{
        let key=((size*10.)as u32,bold);
        if !self.formats.contains_key(&key){
            if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,if bold{DWRITE_FONT_WEIGHT_SEMI_BOLD}else{DWRITE_FONT_WEIGHT_NORMAL},DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){
                let _=format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);self.formats.insert(key,format);
            }
        }
        let text_utf16:Vec<u16>=text.encode_utf16().collect();
        if let Some(format)=self.formats.get(&key){
            if let Ok(layout)=self.factory.CreateTextLayout(&text_utf16,format,width,height){
                let _=layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                let _=layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                let trimming=DWRITE_TRIMMING{
                    granularity:DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter:0,
                    delimiterCount:0,
                };
                if let Ok(sign)=self.factory.CreateEllipsisTrimmingSign(format){
                    let _=layout.SetTrimming(&trimming,&sign);
                }
                self.brush.SetColor(&color(rgb,1.));
                let origin=windows_numerics::Vector2{X:x,Y:y};
                self.context.DrawTextLayout(origin,&layout,&self.brush,D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
            }
        }
    }}
    pub unsafe fn text_centered(&mut self,text:&str,x:f32,y:f32,width:f32,height:f32,size:f32,rgb:u32,alpha:f32,bold:bool){unsafe{
        let key=((size*10.)as u32,bold);
        if !self.formats.contains_key(&key){
            if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,if bold{DWRITE_FONT_WEIGHT_SEMI_BOLD}else{DWRITE_FONT_WEIGHT_NORMAL},DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){
                let _=format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);
                self.formats.insert(key,format);
            }
        }
        let text_utf16:Vec<u16>=text.encode_utf16().collect();
        if let Some(format)=self.formats.get(&key){
            if let Ok(layout)=self.factory.CreateTextLayout(&text_utf16,format,width,height){
                let _=layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                let _=layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                let _=layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                let trimming=DWRITE_TRIMMING{
                    granularity:DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter:0,
                    delimiterCount:0,
                };
                if let Ok(sign)=self.factory.CreateEllipsisTrimmingSign(format){
                    let _=layout.SetTrimming(&trimming,&sign);
                }
                self.brush.SetColor(&color(rgb,alpha));
                let origin=windows_numerics::Vector2{X:x,Y:y};
                self.context.DrawTextLayout(origin,&layout,&self.brush,D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
            }
        }
    }}
    pub unsafe fn measure_text(&mut self,text:&str,size:f32,bold:bool)->f32{unsafe{
        let key=((size*10.)as u32,bold);
        if !self.formats.contains_key(&key){
            if let Ok(format)=self.factory.CreateTextFormat(w!("Segoe UI"),None,if bold{DWRITE_FONT_WEIGHT_SEMI_BOLD}else{DWRITE_FONT_WEIGHT_NORMAL},DWRITE_FONT_STYLE_NORMAL,DWRITE_FONT_STRETCH_NORMAL,size,w!("id-ID")){
                let _=format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);
                self.formats.insert(key,format);
            }
        }
        let text_utf16:Vec<u16>=text.encode_utf16().collect();
        if let Some(format)=self.formats.get(&key){
            if let Ok(layout)=self.factory.CreateTextLayout(&text_utf16,format,1000.,100.){let _=layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                let mut metrics=DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut metrics).is_ok(){
                    return metrics.widthIncludingTrailingWhitespace;
                }
            }
        }
        text.chars().count() as f32 * (size * 0.58)
    }}
    pub unsafe fn bars(&self,levels:&[f32],x:f32,y:f32,width:f32,height:f32,rgb:u32){unsafe{
        if levels.is_empty(){return}let step=width/levels.len()as f32;
        for (i,&value)in levels.iter().enumerate(){let h=(value.clamp(0.,1.)*height).max(2.);self.fill(x+i as f32*step,y+height-h,(step-2.).max(1.),h,1.5,rgb);}
    }}
    /// Fades a whole scene without touching every draw call inside it.
    /// A layer that cannot be created degrades to fully opaque drawing, never to a crash.
    /// Scales everything drawn afterwards around a logical-space point. Used so the panel
    /// grows out of the pill instead of being revealed from its top-left corner.
    #[allow(dead_code)] pub unsafe fn scale_about(&self,scale:f32,center_x:f32,center_y:f32){unsafe{
        let matrix=windows_numerics::Matrix3x2{M11:scale,M12:0.,M21:0.,M22:scale,M31:center_x*(1.-scale),M32:center_y*(1.-scale)};
        self.context.SetTransform(&matrix);
    }}
    pub unsafe fn reset_transform(&self){unsafe{
        self.context.SetTransform(&windows_numerics::Matrix3x2{M11:1.,M12:0.,M21:0.,M22:1.,M31:0.,M32:0.});
    }}
    /// Shifts everything drawn afterwards. Used to move from window space (which includes the
    /// shadow margin) into capsule space, so the page code can keep using capsule coordinates.
    pub unsafe fn translate(&self,x:f32,y:f32){unsafe{
        self.context.SetTransform(&windows_numerics::Matrix3x2{M11:1.,M12:0.,M21:0.,M22:1.,M31:x,M32:y});
    }}
    #[allow(dead_code)] pub unsafe fn translate_relative(&self,dx:f32,dy:f32){unsafe{
        let mut matrix=windows_numerics::Matrix3x2::default();
        self.context.GetTransform(&mut matrix);
        matrix.M31+=dx;
        matrix.M32+=dy;
        self.context.SetTransform(&matrix);
    }}
    pub unsafe fn push_alpha(&mut self,opacity:f32){unsafe{
        let depth=self.layers as usize;
        if depth>=self.layer_pool.len(){return} // never nest deeper than the pool
        if self.layer_pool[depth].is_none(){self.layer_pool[depth]=self.context.CreateLayer(None).ok();}
        if let Some(layer)=&self.layer_pool[depth]{
            let params=D2D1_LAYER_PARAMETERS1{
                contentBounds:rect(-100000.,-100000.,200000.,200000.),
                geometricMask:std::mem::ManuallyDrop::new(None),
                maskAntialiasMode:D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform:windows_numerics::Matrix3x2{M11:1.,M12:0.,M21:0.,M22:1.,M31:0.,M32:0.},
                opacity:opacity.clamp(0.,1.),
                opacityBrush:std::mem::ManuallyDrop::new(None),
                layerOptions:D2D1_LAYER_OPTIONS1_NONE,
            };
            self.context.PushLayer(&params,layer);
            self.layers+=1;
        }
    }}
    pub unsafe fn pop_alpha(&mut self){unsafe{if self.layers>0{self.context.PopLayer();self.layers-=1;}}}
    pub unsafe fn clip(&self,x:f32,y:f32,width:f32,height:f32){unsafe{self.context.PushAxisAlignedClip(&rect(x,y,width,height),D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);}}
    pub unsafe fn unclip(&self){unsafe{self.context.PopAxisAlignedClip();}}
}
// Thin vector primitives used by the Reminder design, in logical pixels.
impl Painter {
    pub unsafe fn polyline(&self,points:&[(f32,f32)],rgb:u32,width:f32,closed:bool){unsafe{
        if points.len()<2{return}if let Ok(path)=self.d2d.CreatePathGeometry(){if let Ok(sink)=path.Open(){
            sink.BeginFigure(pt(points[0].0,points[0].1),D2D1_FIGURE_BEGIN_HOLLOW);
            for &(x,y) in &points[1..]{sink.AddLine(pt(x,y));}
            sink.EndFigure(if closed{D2D1_FIGURE_END_CLOSED}else{D2D1_FIGURE_END_OPEN});let _=sink.Close();
            self.brush.SetColor(&color(rgb,1.));self.context.DrawGeometry(&path,&self.brush,width,None);
        }}
    }}
    pub unsafe fn circle_outline(&self,x:f32,y:f32,r:f32,rgb:u32,width:f32){unsafe{
        self.brush.SetColor(&color(rgb,1.));self.context.DrawEllipse(&D2D1_ELLIPSE{point:pt(x,y),radiusX:r,radiusY:r},&self.brush,width,None);
    }}
    pub unsafe fn progress_ring(&self,x:f32,y:f32,r:f32,ratio:f32,rgb:u32){unsafe{
        self.circle_outline(x,y,r,0x25252d,4.);
        let ratio=ratio.clamp(0.,1.);if ratio<=0.{return}
        let n=(ratio*240.).ceil()as usize;let points:Vec<_>=(0..=n).map(|i|{let a=-std::f32::consts::FRAC_PI_2+std::f32::consts::TAU*ratio*i as f32/n as f32;(x+a.cos()*r,y+a.sin()*r)}).collect();
        self.polyline(&points,rgb,4.,false);
        for &(cx,cy) in [points[0],points[n]].iter(){self.fill(cx-2.,cy-2.,4.,4.,2.,rgb);}
    }}
    pub unsafe fn outline_icon(&self,name:&str,x:f32,y:f32,size:f32,rgb:u32){unsafe{
        let k=size/24.;let line=|ps:&[(f32,f32)]|{let ps:Vec<_>=ps.iter().map(|&(a,b)|(x+a*k,y+b*k)).collect();self.polyline(&ps,rgb,1.6*k,false);};
        let circle=|a:f32,b:f32,r:f32|self.circle_outline(x+a*k,y+b*k,r*k,rgb,1.6*k);
        match name{
            "music"=>{line(&[(9.,18.),(9.,5.),(20.,3.),(20.,16.)]);line(&[(9.,9.),(20.,7.)]);circle(6.,18.,3.);circle(17.,16.,3.);},
            "sliders"=>{for (a,b,c) in [(4.,7.,7.),(4.,12.,16.),(4.,17.,10.)]{line(&[(a,b),(20.,b)]);self.fill(x+(c-1.5)*k,y+(b-2.5)*k,3.*k,5.*k,1.*k,0x060608);line(&[(c,b-3.),(c,b+3.)]);}},
            "calendar"=>{line(&[(5.,5.),(19.,5.),(20.,6.),(20.,20.),(4.,20.),(4.,6.),(5.,5.)]);line(&[(4.,10.),(20.,10.)]);line(&[(8.,2.),(8.,7.)]);line(&[(16.,2.),(16.,7.)]);for a in [8.,12.,16.]{for b in [13.,16.]{self.fill(x+(a-0.7)*k,y+(b-0.7)*k,1.4*k,1.4*k,0.3,rgb);}}},
            "zap"=>line(&[(13.,2.),(4.,14.),(11.,14.),(10.,22.),(20.,9.),(13.,9.),(13.,2.)]),
            "bell"=>{line(&[(4.,17.),(6.,14.),(6.,8.),(8.,5.),(12.,4.),(16.,5.),(18.,8.),(18.,14.),(20.,17.),(4.,17.)]);line(&[(9.,20.),(10.,21.),(14.,21.),(15.,20.)]);},
            "clipboard"=>{line(&[(9.,4.),(5.,4.),(5.,21.),(19.,21.),(19.,4.),(15.,4.)]);line(&[(9.,2.),(15.,2.),(15.,6.),(9.,6.),(9.,2.)]);line(&[(9.,11.),(15.,11.)]);line(&[(9.,15.),(15.,15.)]);},
            "timer"=>{circle(12.,14.,8.);line(&[(9.,2.),(15.,2.)]);line(&[(12.,11.),(15.,8.)]);},
            "droplet"=>{line(&[(12.,2.),(7.,9.),(4.5,13.),(4.,16.),(6.,20.),(10.,22.),(14.,22.),(18.,20.),(20.,16.),(19.,13.),(17.,9.),(12.,2.)]);},
            "person"=>{circle(12.,4.,2.);line(&[(4.,8.),(12.,11.),(20.,8.)]);line(&[(12.,10.),(12.,15.),(7.,22.)]);line(&[(12.,15.),(17.,22.)]);},
            "moon"=>{line(&[(19.,15.),(15.,16.),(11.,14.),(9.,11.),(9.,7.),(10.,4.),(6.,6.),(3.,11.),(4.,17.),(8.,21.),(14.,21.),(18.,18.),(19.,15.)]);line(&[(17.,2.),(18.,5.),(21.,6.),(18.,7.),(17.,10.),(16.,7.),(13.,6.),(16.,5.),(17.,2.)]);},
            "monitor"=>{line(&[(3.,4.),(21.,4.),(21.,17.),(3.,17.),(3.,4.)]);line(&[(12.,17.),(12.,21.)]);line(&[(8.,21.),(16.,21.)]);},
            "layout"=>{line(&[(3.,4.),(21.,4.),(21.,20.),(3.,20.),(3.,4.)]);line(&[(9.,4.),(9.,20.)]);line(&[(12.,9.),(18.,9.)]);line(&[(12.,14.),(18.,14.)]);},
            "left"=>line(&[(15.,5.),(8.,12.),(15.,19.)]),
            "right"=>line(&[(9.,5.),(16.,12.),(9.,19.)]),
            "up"=>line(&[(6.,15.),(12.,9.),(18.,15.)]),
            "down"=>line(&[(6.,9.),(12.,15.),(18.,9.)]),
            "reset"|"undo"=>{line(&[(4.,9.),(8.,5.),(14.,5.),(19.,9.),(20.,14.),(17.,19.),(11.,20.),(7.,18.)]);line(&[(4.,3.),(4.,9.),(10.,9.)]);},
            "check"=>line(&[(5.,12.),(10.,17.),(20.,7.)]),
            "pin"=>{line(&[(8.,3.),(18.,3.),(16.,8.),(19.,13.),(7.,13.),(10.,8.),(8.,3.)]);line(&[(13.,13.),(10.,21.)]);},
            "edit"=>{line(&[(18.,2.),(22.,6.),(7.,21.),(3.,21.),(3.,17.),(18.,2.)]);line(&[(15.,5.),(19.,9.)]);},
            "package"=>{line(&[(12.,2.),(21.,7.),(21.,17.),(12.,22.),(3.,17.),(3.,7.),(12.,2.)]);line(&[(12.,12.),(21.,7.)]);line(&[(12.,12.),(3.,7.)]);line(&[(12.,12.),(12.,22.)]);},
            "cloud"=>{line(&[(6.,19.),(18.,19.)]);circle(15.,15.,4.);circle(9.,15.,3.5);circle(12.,11.,4.);},
            "navigation"=>line(&[(3.,11.),(21.,3.),(13.,21.),(11.,13.),(3.,11.)]),
            "wind"=>{line(&[(3.,7.),(15.,7.),(18.,5.),(17.,3.),(14.,3.)]);line(&[(3.,12.),(20.,12.),(22.,10.),(21.,8.)]);line(&[(3.,17.),(13.,17.),(16.,19.),(15.,21.),(12.,21.)]);},
            "volume"=>{line(&[(4.,9.),(8.,9.),(13.,5.),(13.,19.),(8.,15.),(4.,15.),(4.,9.)]);line(&[(16.,9.),(18.,12.),(16.,15.)]);},
            "search"=>{circle(10.,10.,6.);line(&[(14.,14.),(20.,20.)]);},
            "download"=>{line(&[(12.,4.),(12.,15.)]);line(&[(7.,11.),(12.,16.),(17.,11.)]);line(&[(5.,19.),(19.,19.)]);},
            "settings"=>{circle(12.,12.,4.);circle(12.,12.,9.);for i in 0..8{let a=i as f32*std::f32::consts::TAU/8.;line(&[(12.+a.cos()*8.,12.+a.sin()*8.),(12.+a.cos()*11.,12.+a.sin()*11.)]);}},
            "terminal"=>{line(&[(3.,5.),(21.,5.),(21.,19.),(3.,19.),(3.,5.)]);line(&[(7.,10.),(10.,13.),(7.,16.)]);line(&[(13.,16.),(17.,16.)]);},
            "grid"=>{for (a,b) in [(4.,4.),(14.,4.),(4.,14.),(14.,14.)]{line(&[(a,b),(a+6.,b),(a+6.,b+6.),(a,b+6.),(a,b)]);}},
            "star"=>line(&[(12.,2.),(15.,9.),(22.,9.),(16.,14.),(18.,21.),(12.,17.),(6.,21.),(8.,14.),(2.,9.),(9.,9.),(12.,2.)]),
            "mic"=>{line(&[(12.,3.),(12.,11.)]);circle(12.,7.,3.);line(&[(8.,9.),(8.,11.),(16.,11.),(16.,9.)]);line(&[(12.,14.),(12.,19.)]);line(&[(9.,19.),(15.,19.)]);},
            "camera"=>{line(&[(4.,8.),(8.,8.),(9.,6.),(15.,6.),(16.,8.),(20.,8.),(20.,18.),(4.,18.),(4.,8.)]);circle(12.,13.,3.);},
            "screen_share"|"share"=>{line(&[(3.,4.),(21.,4.),(21.,16.),(3.,16.),(3.,4.)]);line(&[(12.,16.),(12.,20.)]);line(&[(8.,20.),(16.,20.)]);circle(12.,10.,2.);},
            "record"=>{circle(12.,12.,6.);},
            "shield"|"privacy"=>{line(&[(12.,2.),(20.,5.),(20.,11.),(12.,21.),(4.,11.),(4.,5.),(12.,2.)]);},
            // Unknown manifest icons still need an affordance instead of an empty slot.
            _=>{line(&[(12.,2.),(21.,7.),(21.,17.),(12.,22.),(3.,17.),(3.,7.),(12.,2.)]);line(&[(12.,12.),(21.,7.)]);line(&[(12.,12.),(3.,7.)]);line(&[(12.,12.),(12.,22.)]);},
        }
    }}
}
