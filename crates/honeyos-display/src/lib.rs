use std::sync::{Arc, Mutex, MutexGuard, Once};

use hashbrown::HashMap;
use uuid::Uuid;
use web_sys::{
    wasm_bindgen::{closure::Closure, JsCast},
    Document, HtmlElement, KeyboardEvent, Window,
};

/// The static instance of the display server
static mut DISPLAY_SERVER: Option<Arc<Mutex<DisplayServer>>> = None;

/// The display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Text,
    FrameBuffer,
}

/// A display for a process
#[derive(Debug)]
pub struct Display {
    text: String,
    pub mode: DisplayMode,
    pub keybuffer: KeyBuffer,
    pub updated: bool,
}

/// The keybuffer registered to the display
#[derive(Debug)]
pub struct KeyBuffer {
    pub key: i32,
    pub shift: bool,
    pub ctrl: bool,
}

/// The honeyos display server.
/// Manages the display for honeyos processes.
/// A display with a uuid nil value is also registerd for kernel output
#[derive(Debug)]
pub struct DisplayServer {
    root: Option<HtmlElement>,
    current: Uuid,
    displays: HashMap<Uuid, Display>,
}

impl DisplayServer {
    /// Get the static instance
    pub fn get<'a>() -> Option<MutexGuard<'a, Self>> {
        let display = unsafe {
            DISPLAY_SERVER
                .as_ref()
                .expect("Display server not initialized")
        };
        display.try_lock().ok()
    }

    /// Get the static instance.
    /// Blocks until locked.
    pub fn blocking_get<'a>() -> MutexGuard<'a, Self> {
        let display = unsafe {
            DISPLAY_SERVER
                .as_ref()
                .expect("Display server not initialized")
        };
        loop {
            if let Ok(display) = display.try_lock() {
                return display;
            }
        }
    }
}

impl DisplayServer {
    /// Initialize the display server.
    /// Setups up the html structure.
    /// Should only be called once.
    pub fn init_once() {
        static SET_HOOK: Once = Once::new();
        SET_HOOK.call_once(|| {
            let window = web_sys::window()
                .expect("Failed to get window. Display server must be run in main thread");
            let document = window.document().unwrap();

            let root = create_root_node(&document);
            register_callbacks(&window);

            // Register a display at Uuid::nil for kernel rendering
            let mut displays = HashMap::new();
            displays.insert(
                Uuid::nil(),
                Display {
                    text: String::new(),
                    mode: DisplayMode::Text,
                    keybuffer: KeyBuffer {
                        key: -1,
                        shift: false,
                        ctrl: false,
                    },
                    updated: false,
                },
            );

            unsafe {
                DISPLAY_SERVER = Some(Arc::new(Mutex::new(DisplayServer {
                    root: Some(root),
                    current: Uuid::nil(),
                    displays,
                })))
            }
        });
    }

    /// Set the current process to be displayed
    pub fn set_current(&mut self, id: Uuid) -> Option<()> {
        self.current = id;
        let display = self.display_mut(id)?;
        display.updated = true;
        Some(())
    }

    /// Register a display
    /// Does nothing if already registered
    pub fn register(&mut self, process: Uuid, mode: DisplayMode) {
        if self.displays.contains_key(&process) {
            return;
        }

        self.displays.insert(
            process,
            Display {
                text: String::new(),
                keybuffer: KeyBuffer {
                    key: -1,
                    shift: false,
                    ctrl: false,
                },
                mode,
                updated: false,
            },
        );
    }

    /// Check if a process has a display registered
    pub fn has_display(&self, id: Uuid) -> bool {
        self.displays.contains_key(&id)
    }

    /// Get a display
    pub fn display(&self, id: Uuid) -> Option<&Display> {
        self.displays.get(&id)
    }

    /// Get a display
    pub fn display_mut(&mut self, id: Uuid) -> Option<&mut Display> {
        self.displays.get_mut(&id)
    }

    /// Get the root element
    /// This is an implementation detail and should usually not be used outside of the kernel.
    pub fn root(&self) -> Option<&HtmlElement> {
        self.root.as_ref()
    }

    /// Update the display server and render to the screen
    pub fn render(&mut self) {
        let display = self.displays.get_mut(&self.current).unwrap();

        if !display.updated {
            return;
        }
        display.updated = false;

        let root = self
            .root
            .as_ref()
            .expect("Display server not yet initialized!");

        match display.mode {
            DisplayMode::Text => {
                let sanitized = html_escape(&display.text);
                let colored = text_to_terminal(&sanitized);

                root.set_inner_html(&colored);
            }
            DisplayMode::FrameBuffer => unimplemented!("Only text mode is currently supported"),
        }
    }
}

impl Display {
    /// Set the text mode buffer
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.updated = true;
    }
}

/// Escape HTML entities in a string.
/// To sanitize the display input
fn html_escape(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

/// Transform the text with ASCII color codes to HTML code that renders those colors and styles
fn text_to_terminal(input: &str) -> String {
    /// Map ASCII color codes to HTML color names
    fn map_color<'a>(color_code: &str) -> &str {
        let color_code = color_code.replace("[", "");
        let color_code = color_code.as_str();
        match color_code {
            "30" => "#000000",
            "31" => "#aa0000",
            "32" => "#00aa00",
            "33" => "#aa5500",
            "34" => "#0000aa",
            "35" => "#aa00aa",
            "36" => "#00aaaa",
            "37" => "#aaaaaa",
            "90" => "#555555",
            "91" => "#FF5555",
            "92" => "#55FF55",
            "93" => "#FFFF55",
            "94" => "#5555FF",
            "95" => "#FF55FF",
            "96" => "#55FFFF",
            "97" => "#FFFFFF",
            _ => "#FFFFFF",
        }
    }

    /// Map ASCII style codes to HTML style attributes
    fn map_style<'a>(style_code: &str) -> &'a str {
        let style_code = style_code.replace("[", "");
        let style_code = style_code.as_str();
        match style_code {
            "0" => "font-weight:normal;text-decoration:none;",
            "1" => "font-weight:bold;",
            "4" => "text-decoration:underline;",
            "7" => "filter:invert(100%);",
            _ => "",
        }
    }

    let mut html = String::new();
    let mut in_escape = false;
    let mut current_code = String::new();

    for c in input.chars() {
        match c {
            '\x1b' => {
                in_escape = true;
                current_code.clear();
            }
            'm' if in_escape => {
                in_escape = false;
                let mut code = map_style(&current_code);
                if code.is_empty() {
                    code = map_color(&current_code);
                    if !code.is_empty() {
                        html.push_str(&format!("<span style=\"color:{};\">", code));
                    }
                    continue;
                }
                html.push_str(&format!("<span style=\"{};\">", code));
            }
            ' ' => html.push(c),
            _ if in_escape => current_code.push(c),
            _ => html.push(c),
        }
    }

    html
}

/// Register callbacks
fn register_callbacks(window: &Window) {
    // Register the key callback
    window
        .add_event_listener_with_callback(
            "keydown",
            Closure::<dyn Fn(KeyboardEvent)>::new(|event: KeyboardEvent| loop {
                event.prevent_default();

                let Some(mut display_server) = DisplayServer::get() else {
                    continue;
                };
                let pid = display_server.current;
                let display = display_server.display_mut(pid).unwrap();
                display.keybuffer = KeyBuffer {
                    key: event.key_code() as i32,
                    shift: event.shift_key(),
                    ctrl: event.ctrl_key(),
                };
                break;
            })
            .into_js_value()
            .unchecked_ref(),
        )
        .unwrap();
}

/// Create the root node
fn create_root_node(document: &Document) -> HtmlElement {
    let root = document.create_element("div").unwrap();
    let root: HtmlElement = root.dyn_into().unwrap();
    root.set_id("display");
    document.body().unwrap().append_child(&root).unwrap();

    let root = document.get_element_by_id("display").unwrap();
    let root: HtmlElement = root.dyn_into().unwrap();
    root
}
