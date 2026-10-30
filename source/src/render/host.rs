//! Backdrop belongs to a clipped visual, never to the full transparent HWND.
use windows::core::{implement, Interface, Ref, Result};
use windows::Graphics::{IGeometrySource2D, IGeometrySource2D_Impl};
use windows::UI::Composition::*;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::{ID2D1Geometry, ID2D1Factory};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain3;
use windows::Win32::System::WinRT::Composition::{ICompositorDesktopInterop, ICompositorInterop};
use windows::Win32::System::WinRT::Graphics::Direct2D::{IGeometrySource2DInterop, IGeometrySource2DInterop_Impl};

#[implement(IGeometrySource2D, IGeometrySource2DInterop)]
struct GeometrySource { geometry: ID2D1Geometry }
impl IGeometrySource2D_Impl for GeometrySource_Impl {}
impl IGeometrySource2DInterop_Impl for GeometrySource_Impl {
    fn GetGeometry(&self) -> Result<ID2D1Geometry> { Ok(self.geometry.clone()) }
    fn TryGetGeometryUsingFactory(&self, _: Ref<ID2D1Factory>) -> Result<ID2D1Geometry> {
        Err(windows::core::Error::from_hresult(windows::Win32::Foundation::E_NOTIMPL))
    }
}

struct UiRuntime { queue: Option<windows::System::DispatcherQueueController> }
impl Drop for UiRuntime {
    fn drop(&mut self) {
        self.queue.take();
        unsafe { windows::Win32::System::WinRT::RoUninitialize(); }
    }
}

thread_local! {
    // Keep one dispatcher for the UI thread, also through painter/device recreation.
    static QUEUE: std::cell::RefCell<Option<UiRuntime>> = const { std::cell::RefCell::new(None) };
}

pub(super) struct HostComposition {
    _compositor: Compositor,
    _target: DesktopWindowTarget,
    _root: ContainerVisual,
    foreground: SpriteVisual,
    backdrop: SpriteVisual,
    path: CompositionPathGeometry,
    last: Option<[f32; 8]>,
}
impl HostComposition {
    pub unsafe fn new(hwnd: HWND, swap: &IDXGISwapChain3, width: u32, height: u32) -> Result<Self> { unsafe {
        use windows::Win32::System::WinRT::*;
        QUEUE.with(|cell| -> Result<()> {
            if cell.borrow().is_none() {
                RoInitialize(RO_INIT_SINGLETHREADED)?;
                let queue = match CreateDispatcherQueueController(DispatcherQueueOptions {
                    dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
                    threadType: DQTYPE_THREAD_CURRENT, apartmentType: DQTAT_COM_STA,
                }) { Ok(q)=>q,Err(e)=>{RoUninitialize();return Err(e)} };
                *cell.borrow_mut() = Some(UiRuntime{queue:Some(queue)});
            }
            Ok(())
        })?;
        let compositor = Compositor::new()?;
        let desktop: ICompositorDesktopInterop = compositor.cast()?;
        let target = desktop.CreateDesktopWindowTarget(hwnd, true)?;
        let root = compositor.CreateContainerVisual()?;
        let backdrop = compositor.CreateSpriteVisual()?;
        let enabled = windows::core::BOOL(1);
        windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd,
            windows::Win32::Graphics::Dwm::DWMWA_USE_HOSTBACKDROPBRUSH,
            &enabled as *const _ as *const _, std::mem::size_of_val(&enabled) as u32)?;
        backdrop.SetBrush(&compositor.CreateHostBackdropBrush()?)?;
        backdrop.SetIsVisible(false)?;
        let path = compositor.CreatePathGeometry()?;
        backdrop.SetClip(&compositor.CreateGeometricClipWithGeometry(&path)?)?;
        let interop: ICompositorInterop = compositor.cast()?;
        let surface = interop.CreateCompositionSurfaceForSwapChain(swap)?;
        let brush = compositor.CreateSurfaceBrushWithSurface(&surface)?;
        let foreground = compositor.CreateSpriteVisual()?;
        foreground.SetBrush(&brush)?;
        root.Children()?.InsertAtBottom(&backdrop)?;
        root.Children()?.InsertAtTop(&foreground)?;
        target.SetRoot(&root)?;
        let mut result = Self { _compositor: compositor, _target: target, _root: root, foreground, backdrop, path, last: None };
        result.resize(width, height)?;
        Ok(result)
    }}
    pub fn resize(&mut self, w: u32, h: u32) -> Result<()> {
        let size = windows_numerics::Vector2 { X: w as f32, Y: h as f32 };
        self.foreground.SetSize(size)?;
        self.backdrop.SetSize(size)?;
        self.last = None;
        Ok(())
    }
    pub fn contour(&mut self, geometry: ID2D1Geometry, key: [f32;8], enabled: bool) -> Result<()> {
        if enabled && self.last != Some(key) {
            let source: IGeometrySource2D = GeometrySource { geometry }.into();
            self.path.SetPath(&CompositionPath::Create(&source)?)?;
            self.last = Some(key);
        }
        self.backdrop.SetIsVisible(enabled)?;
        Ok(())
    }
    pub fn needs_contour(&self,key:[f32;8])->bool{self.last!=Some(key)}
    pub fn visible(&self,enabled:bool)->Result<()>{self.backdrop.SetIsVisible(enabled)}
}
