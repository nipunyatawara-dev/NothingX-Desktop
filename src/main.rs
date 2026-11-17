#![cfg_attr(windows, windows_subsystem = "windows")]
mod panel;
mod platform;
mod transport;

use nothingx_desktop::{
    device::{Action, Device},
    ui_state,
};
use serde_json::{Value as Json, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tao::{
    event::{Event as TaoEvent, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent},
};

#[derive(Debug)]
pub enum Event {
    Connected(String, String),
    Data(Vec<u8>),
    Disconnected,
    Log(String),
    TransportExited,
    Tray(TrayIconEvent),
    Ipc(String),
    OutsideClick,
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|s| s == "--version") {
        println!("NothingX Desktop {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|s| s == "--help") {
        println!(
            "NothingX Desktop\n--version\n--simulate [model]\n--open\n--status-file PATH\n--quit-after SECONDS"
        );
        return;
    }
    let simulation = args.iter().position(|s| s == "--simulate");
    let model = simulation
        .and_then(|i| args.get(i + 1))
        .filter(|s| !s.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("CMF Buds 2 Plus");
    let quit_at = args
        .iter()
        .position(|s| s == "--quit-after")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u64>().ok())
        .map(|s| Instant::now() + Duration::from_secs(s));
    let status_path = args
        .iter()
        .position(|s| s == "--status-file")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);
    let auto_open = args.iter().any(|s| s == "--open");
    let _lock = match platform::single_instance(simulation.is_some()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("App already running or lock unavailable: {e}");
            return;
        }
    };
    let mut builder = EventLoopBuilder::<Event>::with_user_event();
    let event_loop = builder.build();
    #[cfg(target_os = "macos")]
    let event_loop = {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        let mut loop_ = event_loop;
        loop_.set_activation_policy(ActivationPolicy::Accessory);
        loop_
    };
    let proxy = event_loop.create_proxy();
    let popup_selection = Arc::new(Mutex::new(None::<String>));
    let selected = popup_selection.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id.0.starts_with("popup:") {
            *selected.lock().unwrap() = Some(e.id.0);
        }
    }));
    let tray_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |e| {
        let _ = tray_proxy.send_event(Event::Tray(e));
    }));
    let mut d = if simulation.is_some() {
        Device::simulate(model)
    } else {
        Device::default()
    };
    let mut tray = None;
    let mut panel: Option<panel::Panel> = None;
    let mut bluetooth: Option<transport::Transport> = None;
    let mut restart = None;
    let mut ready = false;
    let mut last_hidden = Instant::now() - Duration::from_secs(1);
    let mut signature = Json::Null;
    let mut ignore_blur_until = Instant::now();
    let mut previous_visibility = (false, false);
    event_loop.run(move |event, target, flow| {
        let now = Instant::now();
        let mut changed = false;
        let mut quit = false;
        match event {
            TaoEvent::NewEvents(StartCause::Init) => {
                tray = Some(
                    TrayIconBuilder::new()
                        .with_icon(panel::tray_icon())
                        .with_icon_as_template(false)
                        .with_tooltip("NothingX Desktop")
                        .with_menu_on_left_click(false)
                        .build()
                        .expect("tray icon"),
                );
                panel =
                    Some(panel::Panel::new(target, proxy.clone()).expect("system webview panel"));
                if let (Some(tray), Some(p)) = (&tray, &panel) {
                    panel::update_tray_icon(tray, &p.window);
                }
                changed = true;
                if simulation.is_none() {
                    match transport::Transport::start(proxy.clone()) {
                        Ok(t) => bluetooth = Some(t),
                        Err(e) => {
                            d.message = e;
                            restart = Some(now + Duration::from_secs(10));
                        }
                    }
                }
            }
            TaoEvent::WindowEvent {
                event: WindowEvent::Focused(false),
                ..
            } => {
                if let Some(p) = &panel
                    && p.window.is_visible()
                    && now >= ignore_blur_until
                {
                    p.hide();
                    last_hidden = now;
                }
            }
            TaoEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Some(p) = &panel {
                    p.hide();
                    last_hidden = now;
                }
            }
            TaoEvent::WindowEvent {
                event: WindowEvent::ThemeChanged(_),
                ..
            } => {
                if let (Some(tray), Some(p)) = (&tray, &panel) {
                    panel::update_tray_icon(tray, &p.window);
                }
            }
            TaoEvent::Reopen { .. } => {
                d.action(Action::Refresh, now);
                if !d.connected
                    && let Some(t) = &mut bluetooth
                {
                    t.refresh();
                }
                if let (Some(p), Some(tray)) = (&panel, &tray) {
                    p.show(tray);
                }
                changed = true;
            }
            TaoEvent::UserEvent(event) => {
                changed = true;
                match event {
                    Event::Connected(name, address) => {
                        eprintln!("Connected to {name}");
                        d.on_connected(name, address);
                    }
                    Event::Data(bytes) => d.receive(&bytes),
                    Event::Disconnected => d.disconnect(),
                    Event::OutsideClick => {
                        changed=false;
                        if let Some(p)=&panel && p.is_open() && now>=ignore_blur_until {p.hide();last_hidden=now;}
                    }
                    Event::Tray(TrayIconEvent::Enter { .. }) => {
                        if let (Some(tray), Some(p)) = (&tray, &panel) {
                            panel::update_tray_icon(tray, &p.window);
                        }
                    }
                    Event::Log(message) => {
                        eprintln!("{message}");
                        d.message = message;
                    }
                    Event::TransportExited => {
                        bluetooth.take();
                        d.disconnect();
                        restart = Some(now + Duration::from_secs(5));
                    }
                    Event::Tray(TrayIconEvent::Click {
                        button,
                        button_state: MouseButtonState::Up,
                        ..
                    }) => {
                        if let (Some(p), Some(tray)) = (&panel, &tray)
                            && button == MouseButton::Left {
                                if p.is_open() {
                                    p.hide();
                                    last_hidden = now;
                                } else if now.duration_since(last_hidden)
                                    > Duration::from_millis(250)
                                {
                                    d.action(Action::Refresh, now);
                                    if !d.connected
                                        && let Some(t) = &mut bluetooth
                                    {
                                        t.refresh();
                                    }
                                    p.show(tray);
                                }
                            }
                    }
                    Event::Ipc(body) => {
                        if let Ok(message) = serde_json::from_str::<Json>(&body) {
                            let command = message["command"].as_str().unwrap_or("");
                            let id = message["id"].as_u64().unwrap_or(0);
                            let args = message["args"].as_array().map(Vec::as_slice).unwrap_or(&[]);
                            if let Some(p) = &panel {
                                match command {
                                    "init" => {
                                        let mut init = ui_state::init(&d);
                                        init["accent"] = json!(panel::accent());
                                        p.reply(id, &init);
                                        ready = true;
                                        if auto_open && let Some(tray) = &tray {
                                            p.show(tray);
                                        }
                                    }
                                    "motionPreference" => {
                                        if let Some(reduced) = args.first().and_then(Json::as_bool)
                                        {
                                            p.set_reduced_motion(reduced);
                                        }
                                    }
                                    "setHeight" => {
                                        if let Some(h) = args.first().and_then(Json::as_f64) {
                                            p.resize(h);
                                        }
                                    }
                                    "quit" => quit = true,
                                    "popupMenu" => {
                                        let menu = Menu::new();
                                        let items = args.first().and_then(Json::as_array);
                                        if let Some(items) = items.filter(|items| items.len() <= 32)
                                        {
                                            *popup_selection.lock().unwrap() = None;
                                            for (i, item) in items.iter().enumerate() {
                                                if let Some(label) = item["label"].as_str() {
                                                    let _ = menu.append(&CheckMenuItem::with_id(
                                                        format!("popup:{i}"),
                                                        label,
                                                        true,
                                                        item["checked"].as_bool().unwrap_or(false),
                                                        None,
                                                    ));
                                                }
                                            }
                                            let x =
                                                args.get(1).and_then(Json::as_f64).unwrap_or(0.);
                                            let y =
                                                args.get(2).and_then(Json::as_f64).unwrap_or(0.);
                                            p.popup(&menu, Some((x, y)));
                                            ignore_blur_until =
                                                Instant::now() + Duration::from_millis(250);
                                            let chosen = popup_selection
                                                .lock()
                                                .unwrap()
                                                .take()
                                                .and_then(|s| {
                                                    s.strip_prefix("popup:")?.parse::<usize>().ok()
                                                })
                                                .and_then(|i| items.get(i))
                                                .map(|v| v["value"].clone())
                                                .unwrap_or(Json::Null);
                                            p.reply(id, &chosen);
                                        } else {
                                            p.reply(id, &Json::Null);
                                        }
                                    }
                                    "uiError" => eprintln!("Panel error: {:?}", args.first()),
                                    _ => {
                                        if let Some(action) = ui_state::command(&d, command, args) {
                                            d.action(action, now);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            TaoEvent::LoopDestroyed => {
                bluetooth.take();
                return;
            }
            _ => {}
        }
        if quit || quit_at.is_some_and(|at| now >= at) {
            d.stop_ring();
            for bytes in d.tick(now + Duration::from_secs(1)) {
                if let Some(t) = &mut bluetooth {
                    let _ = t.write(&bytes);
                }
            }
            bluetooth.take();
            *flow = ControlFlow::Exit;
            return;
        }
        if restart.is_some_and(|at| now >= at) {
            restart = None;
            match transport::Transport::start(proxy.clone()) {
                Ok(t) => bluetooth = Some(t),
                Err(e) => {
                    d.message = e;
                    restart = Some(now + Duration::from_secs(10));
                }
            }
        }
        let due = d.deadline().is_some_and(|at| now >= at);
        changed |= due;
        for bytes in d.tick(now) {
            if let Some(t) = &mut bluetooth
                && let Err(e) = t.write(&bytes)
            {
                d.message = e;
            }
        }
        if changed {
            let state = ui_state::snapshot(&d);
            if state != signature {
                signature = state.clone();
                if ready && let Some(p) = &panel {
                    p.state(&state);
                }
                if let Some(path) = &status_path {
                    let _ = std::fs::write(
                        path,
                        serde_json::to_vec_pretty(
                            &json!({"device":state,"message":d.message,"simulation":d.simulation}),
                        )
                        .unwrap(),
                    );
                }
                if let Some(tray) = &tray {
                    let _ = tray.set_tooltip(Some(if d.connected {
                        format!("NothingX Desktop · {}", d.name)
                    } else {
                        "NothingX Desktop".into()
                    }));
                }
            }
        }
        if let Some(p) = &panel {
            p.tick(now);
        }
        let visibility=panel.as_ref().map(|p|(p.window.is_visible(),p.is_open())).unwrap_or_default();
        if visibility!=previous_visibility {
            previous_visibility=visibility;
            if let Some(path)=&status_path {let _=std::fs::write(path,serde_json::to_vec_pretty(&json!({"device":ui_state::snapshot(&d),"message":d.message,"simulation":d.simulation,"popoverVisible":visibility.0,"popoverOpen":visibility.1})).unwrap());}
        }
        *flow = d
            .deadline()
            .into_iter()
            .chain(restart)
            .chain(quit_at)
            .chain(panel.as_ref().and_then(|p| p.deadline()))
            .min()
            .map_or(ControlFlow::Wait, ControlFlow::WaitUntil);
    });
}
