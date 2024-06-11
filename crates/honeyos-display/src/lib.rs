use error::Error;
use std::sync::{Arc, Once, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;
use web_sys::{
    wasm_bindgen::{closure::Closure, JsCast},
    Document, HtmlElement, KeyboardEvent, Window,
};

pub mod error;

/// The static instance of the display
static mut DISPLAY: Option<Arc<RwLock<Display>>> = None;

/// The display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Text,
    FrameBuffer,
}

/// The control over the display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    None,
    /// Strict control. The display can only be taken over properly when the current process releases it.
    Strict(Uuid),
    /// Loose control. The display is being controlled, but processes can still take it over properly.
    Loose(Uuid),
}

/// The keybuffer registered to the display
#[derive(Debug)]
pub struct KeyBuffer {
    pub key: i32,
    pub shift: bool,
    pub ctrl: bool,
}

/// The honeyos display.
/// Manages the display for honeyos processes.
#[derive(Debug)]
pub struct Display {
    root: Option<HtmlElement>,
    text: String,
    control: Control,
    pub mode: Mode,
    pub keybuffer: KeyBuffer,
    pub updated: bool,
}

impl Display {
    /// Get the static instance with write access
    pub fn get_writer<'a>() -> Option<RwLockWriteGuard<'a, Self>> {
        let networking_manager =
            unsafe { DISPLAY.as_ref().expect("Display server not initialized") };
        networking_manager.try_write().ok()
    }

    /// Get the static instance with write access
    /// Blocks until locked.
    pub fn blocking_get_writer<'a>() -> RwLockWriteGuard<'a, Self> {
        let networking_manager =
            unsafe { DISPLAY.as_ref().expect("Display server not initialized") };
        loop {
            if let Ok(networking_manager) = networking_manager.try_write() {
                return networking_manager;
            }
        }
    }

    /// Get the static instance with read access
    pub fn get_reader<'a>() -> Option<RwLockReadGuard<'a, Self>> {
        let networking_manager =
            unsafe { DISPLAY.as_ref().expect("Display server not initialized") };
        networking_manager.try_read().ok()
    }

    /// Get the static instance with read access
    /// Blocks until aqcuired
    pub fn blocking_get_reader<'a>() -> RwLockReadGuard<'a, Self> {
        let networking_manager =
            unsafe { DISPLAY.as_ref().expect("Display server not initialized") };
        loop {
            if let Ok(networking_manager) = networking_manager.try_read() {
                return networking_manager;
            }
        }
    }
}

impl Display {
    /// Initialize the display.
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

            unsafe {
                DISPLAY = Some(Arc::new(RwLock::new(Display {
                    root: Some(root),
                    text: String::new(),
                    keybuffer: KeyBuffer {
                        key: -1,
                        shift: false,
                        ctrl: false,
                    },
                    mode: Mode::Text,
                    control: Control::None,
                    updated: false,
                })))
            }
        });
    }

    /// Get the root element
    /// This is an implementation detail and should usually not be used outside of the kernel.
    pub fn root(&self) -> Option<&HtmlElement> {
        self.root.as_ref()
    }

    /// Attempt to change the process in control
    pub fn assume_control(&mut self, pid: Uuid) -> Result<(), Error> {
        if let Control::Strict(_) = self.control {
            return Err(Error::DisplayOccupied);
        }
        self.control = Control::Strict(pid);
        Ok(())
    }

    /// Loosen control over the display
    pub fn loosen_control(&mut self) -> Result<(), Error> {
        match self.control {
            Control::None => Err(Error::CannotLoosen),
            Control::Loose(_) => Err(Error::AlreadyLoose),
            Control::Strict(pid) => {
                self.control = Control::Loose(pid);
                Ok(())
            }
        }
    }

    /// Override control of the current process
    pub fn override_control(&mut self, pid: Uuid) {
        // For when the situation is dire
        self.control = Control::Strict(pid);
    }

    /// Release the control from the display
    pub fn release_control(&mut self) {
        self.control = Control::None
    }

    /// Check if a process has control
    pub fn has_control(&self, pid: Uuid) -> bool {
        match self.control {
            Control::None => false,
            Control::Strict(current) | Control::Loose(current) => current == pid,
        }
    }

    /// Update the display and render to the screen
    pub fn render(&mut self) {
        if !self.updated {
            return;
        }
        self.updated = false;

        let root = self
            .root
            .as_ref()
            .expect("Display server not yet initialized!");

        match self.mode {
            Mode::Text => {
                let sanitized = html_escape(&self.text);
                let colored = text_to_terminal(&sanitized);

                root.set_inner_html(&colored);
            }
            Mode::FrameBuffer => unimplemented!("Only text mode is currently supported"),
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

                let Some(mut display) = Display::get_writer() else {
                    continue;
                };
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
