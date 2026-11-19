use crate::Event;
use nothingx_desktop::ui_state;
use serde_json::Value as Json;
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};
use tao::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event_loop::{EventLoopProxy, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};
use tray_icon::{
    Icon, TrayIcon,
    menu::{ContextMenu, Menu},
};
use wry::{WebView, WebViewBuilder};

include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

pub struct Panel {
    pub web: WebView,
    pub window: Window,
    _web_context: wry::WebContext,
    height: Cell<f64>,
    anchor: Cell<Option<tray_icon::Rect>>,
    motion: RefCell<Fade>,
    reduced_motion: Cell<bool>,
    #[cfg(target_os = "macos")]
    _monitors: OutsideMonitors,
}
#[cfg(target_os = "macos")]
struct OutsideMonitors {
    global: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
    local: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
}
#[cfg(target_os = "macos")]
impl OutsideMonitors {
    fn new(window: &Window, proxy: EventLoopProxy<Event>) -> Self {
        use objc2_app_kit::{NSEvent, NSEventMask};
        use tao::platform::macos::WindowExtMacOS;
        let number =
            unsafe { (*window.ns_window().cast::<objc2_app_kit::NSWindow>()).windowNumber() };
        let global_proxy = proxy.clone();
        let global = block2::RcBlock::new(move |_: std::ptr::NonNull<NSEvent>| {
            let _ = global_proxy.send_event(Event::OutsideClick);
        });
        let local = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| {
            if unsafe { event.as_ref().windowNumber() } != number {
                let _ = proxy.send_event(Event::OutsideClick);
            }
            event.as_ptr()
        });
        let mask = NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown;
        Self {
            global: NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global),
            local: unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) },
        }
    }
}
#[cfg(target_os = "macos")]
impl Drop for OutsideMonitors {
    fn drop(&mut self) {
        for monitor in [&self.global, &self.local].into_iter().flatten() {
            unsafe {
                objc2_app_kit::NSEvent::removeMonitor(monitor);
            }
        }
    }
}
struct Fade {
    from: f64,
    to: f64,
    started: Instant,
    duration: Duration,
    active: bool,
}
impl Fade {
    fn value(&self, now: Instant) -> f64 {
        if !self.active {
            return self.to;
        }
        let progress = (now.saturating_duration_since(self.started).as_secs_f64()
            / self.duration.as_secs_f64())
        .clamp(0., 1.);
        // cubic-bezier(.23,1,.32,1): the shared strong UI ease-out.
        let mut low = 0.;
        let mut high = 1.;
        for _ in 0..16 {
            let t = (low + high) / 2.;
            let x = 3. * (1. - t) * (1. - t) * t * 0.23 + 3. * (1. - t) * t * t * 0.32 + t * t * t;
            if x < progress { low = t } else { high = t }
        }
        let t = (low + high) / 2.;
        let eased = 1. - (1. - t).powi(3);
        self.from + (self.to - self.from) * eased
    }
}
impl Panel {
    pub fn new(
        target: &EventLoopWindowTarget<Event>,
        proxy: EventLoopProxy<Event>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_builder = WindowBuilder::new()
            .with_title("NothingX Desktop")
            .with_window_icon(Some(tao::window::Icon::from_rgba(
                include_bytes!("../assets/app-icon.rgba").to_vec(),
                32,
                32,
            )?))
            .with_inner_size(LogicalSize::new(300., 360.))
            .with_decorations(false)
            .with_resizable(false)
            .with_minimizable(false)
            .with_maximizable(false)
            .with_transparent(true)
            .with_always_on_top(true)
            .with_visible(false)
            .with_visible_on_all_workspaces(true);
        #[cfg(windows)]
        let window_builder = {
            use tao::platform::windows::WindowBuilderExtWindows;
            window_builder
                .with_skip_taskbar(true)
                .with_drag_and_drop(false)
                .with_undecorated_shadow(false)
                .with_window_classname("NothingXDesktopWindow")
        };
        let window = window_builder.build(target)?;
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::WindowExtMacOS;
            window.set_has_shadow(true);
        }
        #[cfg(windows)]
        apply_windows_backdrop(&window);
        #[cfg(target_os = "macos")]
        let url = "nx://app/src/popover/index.html";
        #[cfg(windows)]
        let url = "http://nx.app/src/popover/index.html";
        let ipc_proxy = proxy.clone();
        let mut web_context =
            wry::WebContext::new(Some(crate::platform::data_dir().join("webview")));
        let web = {
            let builder = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_transparent(true)
                .with_incognito(true)
                .with_devtools(false)
                .with_hotkeys_zoom(false)
                .with_general_autofill_enabled(false)
                .with_download_started_handler(|_, _| false)
                .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
                .with_drag_drop_handler(|_| true)
                .with_bounds(wry::Rect {
                    position: LogicalPosition::new(0., 0.).into(),
                    size: LogicalSize::new(300., 360.).into(),
                })
                .with_initialization_script(include_str!("../ui/bridge.js"))
                .with_custom_protocol("nx".into(), |_, request| {
                    if request.method() != wry::http::Method::GET {
                        return wry::http::Response::builder()
                            .status(405)
                            .header("Content-Type", "text/plain")
                            .header("X-Content-Type-Options", "nosniff")
                            .body(Cow::Borrowed(b"Method not allowed" as &[u8]))
                            .unwrap();
                    }
                    let path = request.uri().path().trim_start_matches('/');
                    let (status, mime, bytes) = match asset(path) {
                        Some((mime, bytes)) => (200, mime, bytes),
                        None => (404, "text/plain", b"Not found".as_slice()),
                    };
                    wry::http::Response::builder()
                        .status(status)
                        .header("Content-Type", mime)
                        .header("X-Content-Type-Options", "nosniff")
                        .header("Cross-Origin-Resource-Policy", "same-origin")
                        .body(Cow::Borrowed(bytes))
                        .unwrap()
                })
                .with_navigation_handler(move |candidate| candidate == url)
                .with_ipc_handler(move |request| {
                    #[cfg(target_os = "macos")]
                    let trusted = request.uri().scheme_str() == Some("nx")
                        && request.uri().host() == Some("app");
                    #[cfg(windows)]
                    let trusted = request.uri().scheme_str() == Some("http")
                        && request.uri().host() == Some("nx.app");
                    if trusted && request.body().len() <= 16_384 {
                        let _ = ipc_proxy.send_event(Event::Ipc(request.body().clone()));
                    }
                })
                .with_url(url);
            #[cfg(windows)]
            let builder = {
                use wry::WebViewBuilderExtWindows;
                builder
                    .with_browser_accelerator_keys(false)
                    .with_default_context_menus(false)
            };
            builder.build_as_child(&window)?
        };
        // The webview must exist before inserting the background material.
        // Otherwise replacing/attaching its host view can discard the material.
        #[cfg(target_os = "macos")]
        window_vibrancy::apply_vibrancy(
            &window,
            window_vibrancy::NSVisualEffectMaterial::Menu,
            Some(window_vibrancy::NSVisualEffectState::Active),
            Some(12.),
        )?;
        let panel = Self {
            #[cfg(target_os = "macos")]
            _monitors: OutsideMonitors::new(&window, proxy),
            web,
            window,
            _web_context: web_context,
            height: Cell::new(360.),
            anchor: Cell::new(None),
            motion: RefCell::new(Fade {
                from: 0.,
                to: 0.,
                started: Instant::now(),
                duration: Duration::from_millis(150),
                active: false,
            }),
            reduced_motion: Cell::new(false),
        };
        panel.opacity(0.);
        #[cfg(windows)]
        {
            use wry::WebViewExtWindows;
            let _ = panel.web.set_memory_usage_level(wry::MemoryUsageLevel::Low);
        }
        Ok(panel)
    }
    fn position_window(&self) {
        let rect = self.anchor.get();
        let monitor = rect
            .and_then(|r| {
                self.window.available_monitors().find(|m| {
                    let p = m.position();
                    let s = m.size();
                    let x = r.position.x;
                    let y = r.position.y;
                    x >= p.x as f64
                        && y >= p.y as f64
                        && x < (p.x as f64 + f64::from(s.width))
                        && y < (p.y as f64 + f64::from(s.height))
                })
            })
            .or_else(|| self.window.current_monitor())
            .or_else(|| self.window.primary_monitor());
        let Some(m) = monitor else {
            return;
        };
        let scale = m.scale_factor();
        let pos = m.position();
        let size = m.size();
        let width = 300. * scale;
        let height = self.height.get() * scale;
        let margin = 8. * scale;
        let (x, y) = if let Some(rect) = rect {
            let max_x = (f64::from(pos.x) + f64::from(size.width) - width - margin)
                .max(f64::from(pos.x) + margin);
            let x = (rect.position.x + f64::from(rect.size.width) / 2. - width / 2.)
                .clamp(f64::from(pos.x) + margin, max_x);
            #[cfg(target_os = "macos")]
            let y = rect.position.y + f64::from(rect.size.height) + 4. * scale;
            #[cfg(windows)]
            let y = {
                let icon_center_y = rect.position.y + f64::from(rect.size.height) / 2.;
                let monitor_mid_y = f64::from(pos.y) + f64::from(size.height) / 2.;
                if icon_center_y < monitor_mid_y {
                    rect.position.y + f64::from(rect.size.height) + 4. * scale
                } else {
                    (rect.position.y - height - 4. * scale).max(f64::from(pos.y) + margin)
                }
            };
            let max_y = (f64::from(pos.y) + f64::from(size.height) - height - margin)
                .max(f64::from(pos.y) + margin);
            (x, y.clamp(f64::from(pos.y) + margin, max_y))
        } else {
            let x = (f64::from(pos.x) + f64::from(size.width) - width - margin)
                .max(f64::from(pos.x) + margin);
            #[cfg(target_os = "macos")]
            let y = f64::from(pos.y) + 32. * scale;
            #[cfg(windows)]
            let y = (f64::from(pos.y) + f64::from(size.height) - height - 56. * scale)
                .max(f64::from(pos.y) + margin);
            (x, y)
        };
        self.window.set_outer_position(PhysicalPosition::new(x, y));
    }
    pub fn resize(&self, height: f64) {
        if height.is_finite() {
            let height = height.ceil().clamp(100., 800.);
            if (self.height.get() - height).abs() < 0.5 {
                return;
            }
            self.height.set(height);
            self.window.set_inner_size(LogicalSize::new(300., height));
            let _ = self.web.set_bounds(wry::Rect {
                position: LogicalPosition::new(0., 0.).into(),
                size: LogicalSize::new(300., height).into(),
            });
            #[cfg(windows)]
            self.position_window();
        }
    }
    pub fn update_accent(&self) {
        let _ = self.web.evaluate_script(&format!(
            "window.__nxAccent && window.__nxAccent({});",
            serde_json::json!(accent())
        ));
    }
    pub fn on_theme_changed(&self) {
        #[cfg(windows)]
        apply_windows_backdrop(&self.window);
        self.update_accent();
    }
    pub fn has_focus(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.window.is_focused()
        }
        #[cfg(windows)]
        {
            use tao::platform::windows::WindowExtWindows;
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsChild};
            let hwnd = self.window.hwnd() as *mut core::ffi::c_void;
            unsafe {
                let fg = GetForegroundWindow();
                fg == hwnd || (!fg.is_null() && IsChild(hwnd, fg) != 0)
            }
        }
    }
    pub fn show(&self, tray: &TrayIcon) {
        if let Some(rect) = tray
            .rect()
            .filter(|r| r.size.width > 0 || r.size.height > 0)
        {
            self.anchor.set(Some(rect));
        }
        self.position_window();
        self.update_accent();
        #[cfg(windows)]
        {
            use wry::WebViewExtWindows;
            let _ = self
                .web
                .set_memory_usage_level(wry::MemoryUsageLevel::Normal);
        }
        self.animate_to(1.);
        self.window.set_visible(true);
        self.opacity(self.motion.borrow().value(Instant::now()));
        self.window.set_focus();
        let _ = self.web.focus();
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::NSApplication;
            if let Some(mtm) = objc2::MainThreadMarker::new() {
                #[allow(deprecated)]
                NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
            }
        }
    }
    pub fn hide(&self) {
        self.animate_to(0.);
    }
    fn hidden(&self) {
        self.window.set_visible(false);
        #[cfg(windows)]
        {
            use wry::WebViewExtWindows;
            let _ = self.web.set_memory_usage_level(wry::MemoryUsageLevel::Low);
        }
        // WKWebView doesn't always dispatch visibilitychange for a hidden host window.
        // Reset using the renderer's existing listener, without changing its source.
        let _ = self.web.evaluate_script(
            "Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));delete document.hidden;",
        );
    }
    pub fn is_open(&self) -> bool {
        self.motion.borrow().to > 0.
    }
    pub fn set_reduced_motion(&self, reduced: bool) {
        self.reduced_motion.set(reduced);
    }
    fn animate_to(&self, target: f64) {
        let now = Instant::now();
        let mut fade = self.motion.borrow_mut();
        if fade.to == target {
            return;
        }
        let from = fade.value(now);
        *fade = Fade {
            from,
            to: target,
            started: now,
            duration: Duration::from_millis(if target > 0. && !self.reduced_motion.get() {
                150
            } else {
                125
            }),
            active: true,
        };
        self.opacity(from);
    }
    pub fn tick(&self, now: Instant) {
        let mut fade = self.motion.borrow_mut();
        if !fade.active {
            return;
        }
        if now.saturating_duration_since(fade.started) >= fade.duration {
            fade.active = false;
            self.opacity(fade.to);
            if fade.to == 0. {
                drop(fade);
                self.hidden();
            }
        } else {
            self.opacity(fade.value(now));
        }
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.motion
            .borrow()
            .active
            .then(|| Instant::now() + Duration::from_millis(16))
    }
    fn opacity(&self, alpha: f64) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::WindowExtMacOS;
            let ptr = self.window.ns_window().cast::<objc2_app_kit::NSWindow>();
            unsafe {
                (*ptr).setAlphaValue(alpha);
            }
        }
        #[cfg(windows)]
        {
            use tao::platform::windows::WindowExtWindows;
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let hwnd = self.window.hwnd() as *mut core::ffi::c_void;
            unsafe {
                let style = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & !(WS_EX_APPWINDOW as isize))
                    | WS_EX_TOOLWINDOW as isize
                    | WS_EX_LAYERED as isize;
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
                SetLayeredWindowAttributes(
                    hwnd,
                    0,
                    (alpha.clamp(0., 1.) * 255.).round() as u8,
                    LWA_ALPHA,
                );
            }
        }
    }
    pub fn reply(&self, id: u64, value: &Json) {
        let _ = self
            .web
            .evaluate_script(&format!("window.__nxReply({id},{});", script_json(value)));
    }
    pub fn state(&self, value: &Json) {
        let _ = self
            .web
            .evaluate_script(&format!("window.__nxState({});", script_json(value)));
    }
    pub fn popup(&self, menu: &Menu, position: Option<(f64, f64)>) {
        let position = position.map(|(x, y)| LogicalPosition::new(x, y).into());
        #[cfg(target_os = "macos")]
        {
            use wry::WebViewExtMacOS;
            let view = self.web.webview();
            unsafe {
                menu.show_context_menu_for_nsview(std::ptr::from_ref(&*view).cast(), position);
            }
        }
        #[cfg(windows)]
        {
            use tao::platform::windows::WindowExtWindows;
            unsafe {
                menu.show_context_menu_for_hwnd(self.window.hwnd(), position);
            }
        }
    }
}

fn script_json(value: &Json) -> String {
    value
        .to_string()
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}
pub fn tray_icon() -> Icon {
    let [width, height]: [u32; 2] =
        serde_json::from_str(include_str!("../assets/tray-size.json")).unwrap();
    Icon::from_rgba(
        include_bytes!("../assets/tray.rgba").to_vec(),
        width,
        height,
    )
    .expect("original tray logo")
}
pub fn update_tray_icon(tray: &TrayIcon, window: &Window) {
    #[cfg(target_os = "macos")]
    {
        use objc2::{AnyThread, runtime::Bool};
        use objc2_app_kit::{
            NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSImage, NSImageCacheMode,
        };
        use objc2_foundation::{NSArray, NSData, NSRect, NSSize};

        thread_local! {static INSTALLED: Cell<bool> = const {Cell::new(false)};}
        let _ = window;
        if INSTALLED.get() {
            return;
        }
        let Some(button) = objc2::MainThreadMarker::new()
            .and_then(|mtm| tray.ns_status_item().and_then(|item| item.button(mtm)))
        else {
            return;
        };
        let load = |bytes: &[u8]| {
            NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes))
                .expect("embedded menu-bar artwork")
        };
        let black = load(include_bytes!("../assets/nothingx-menubar.png"));
        let white = load(include_bytes!("../assets/nothingx-menubar-dark.png"));
        let draw = block2::RcBlock::new(move |rect: NSRect| {
            // AppKit supplies the menu bar's drawing appearance, which can
            // differ from the app's theme because of the desktop background.
            let names =
                unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
            let dark = NSAppearance::currentDrawingAppearance()
                .bestMatchFromAppearancesWithNames(&names)
                .is_some_and(|name| unsafe { &*name == NSAppearanceNameDarkAqua });
            if dark { &white } else { &black }.drawInRect(rect);
            Bool::YES
        });
        let image =
            NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(18., 18.), false, &draw);
        // Re-evaluate contrast on native redraw, without polling or waiting
        // for hover. Keep the red accent rather than template-tinting it.
        image.setCacheMode(NSImageCacheMode::Never);
        image.setTemplate(false);
        button.setImage(Some(&image));
        INSTALLED.set(true);
    }
    #[cfg(windows)]
    update_windows_tray_icon(tray, window);
}
#[cfg(windows)]
fn apply_windows_backdrop(window: &Window) {
    use std::mem::size_of;
    use tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
    };
    let hwnd = window.hwnd() as *mut core::ffi::c_void;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            std::ptr::from_ref(&DWMWCP_ROUND).cast(),
            size_of::<u32>() as u32,
        );
    }
    let dark = match read_hkcu_dword(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "AppsUseLightTheme",
    ) {
        Some(light) => light == 0,
        None => window.theme() == tao::window::Theme::Dark,
    };
    let tint = if dark {
        (36, 36, 36, 190)
    } else {
        (240, 240, 240, 190)
    };
    let _ = window_vibrancy::apply_acrylic(window, Some(tint));
}
#[cfg(windows)]
fn read_hkcu_dword(subkey: &str, value: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let key: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    let mut data = 0u32;
    let mut size = std::mem::size_of_val(&data) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut size,
        )
    };
    (result == 0).then_some(data)
}
#[cfg(windows)]
fn update_windows_tray_icon(tray: &TrayIcon, window: &Window) {
    thread_local! {static LAST:std::cell::Cell<Option<bool>>=const {std::cell::Cell::new(None)};}
    let dark = match read_hkcu_dword(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "SystemUsesLightTheme",
    ) {
        Some(light) => light == 0,
        None => window.theme() == tao::window::Theme::Dark,
    };
    if LAST.with(|last| last.replace(Some(dark)) == Some(dark)) {
        return;
    }
    let bytes = if dark {
        include_bytes!("../assets/tray-dark.rgba").as_slice()
    } else {
        include_bytes!("../assets/tray.rgba").as_slice()
    };
    if let Ok(icon) = Icon::from_rgba(bytes.to_vec(), 36, 36) {
        let _ = tray.set_icon(Some(icon));
    }
}
pub fn accent() -> String {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSColor, NSColorSpace};
        let color = NSColor::controlAccentColor();
        if let Some(rgb) = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) {
            return format!(
                "#{:02x}{:02x}{:02x}",
                (rgb.redComponent() * 255.).round() as u8,
                (rgb.greenComponent() * 255.).round() as u8,
                (rgb.blueComponent() * 255.).round() as u8
            );
        }
    }
    #[cfg(windows)]
    if let Some(val) = read_hkcu_dword("Software\\Microsoft\\Windows\\DWM", "AccentColor") {
        let r = (val & 0xff) as u8;
        let g = ((val >> 8) & 0xff) as u8;
        let b = ((val >> 16) & 0xff) as u8;
        return format!("#{r:02x}{g:02x}{b:02x}");
    }
    ui_state::appearance()["accent"]
        .as_str()
        .unwrap_or("#007aff")
        .into()
}
