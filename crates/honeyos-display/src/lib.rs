use error::Error;
use std::sync::{Arc, Once, RwLock};
use text::TextMode;
use uuid::Uuid;
use web_sys::{
    wasm_bindgen::{closure::Closure, JsCast},
    Document, Event, HtmlElement, KeyboardEvent, Window,
};

pub mod error;
pub mod text;

/// The static instance of the display
static mut DISPLAY: Option<Arc<RwLock<Display>>> = None;

/// The display mode
#[derive(Debug, Clone, PartialEq, Eq)]
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
    control: Control,
    pub mode: Mode,
    pub keybuffer: KeyBuffer,
    pub updated: bool,
    // The text mode context
    text_mode: TextMode,
}

impl Display {
    pub fn get() -> Arc<RwLock<Display>> {
        unsafe { DISPLAY.as_ref().expect("Display not initialized").clone() }
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

            let (width, height) = (
                window.inner_width().unwrap().unchecked_into_f64(),
                window.inner_height().unwrap().unchecked_into_f64(),
            );

            let root = create_root_node(&document);
            register_callbacks(&window);

            unsafe {
                DISPLAY = Some(Arc::new(RwLock::new(Display {
                    root: Some(root),
                    keybuffer: KeyBuffer {
                        key: -1,
                        shift: false,
                        ctrl: false,
                    },
                    mode: Mode::Text,
                    control: Control::None,
                    updated: false,
                    text_mode: TextMode::new(width as u32, height as u32),
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

        match &self.mode {
            Mode::Text => {
                root.set_inner_html(&self.text_mode.render());
            }
            Mode::FrameBuffer => unimplemented!("Only text mode is currently supported"),
        }
    }
}

impl Display {
    /// Aquire text mode context
    pub fn text_mode(&self) -> &TextMode {
        &self.text_mode
    }

    /// Aquire text mode context
    pub fn text_mode_mut(&mut self) -> &mut TextMode {
        &mut self.text_mode
    }

    /// Notify the display that it needs to be updated
    pub fn notify_update(&mut self) {
        self.updated = true;
    }
}

/// Register callbacks
fn register_callbacks(window: &Window) {
    // Register the key callback
    window
        .add_event_listener_with_callback(
            "keydown",
            Closure::<dyn Fn(KeyboardEvent)>::new(|event: KeyboardEvent| {
                event.prevent_default();

                let display = Display::get();
                let Ok(mut display) = display.try_write() else {
                    return;
                };
                display.keybuffer = KeyBuffer {
                    key: event.key_code() as i32,
                    shift: event.shift_key(),
                    ctrl: event.ctrl_key(),
                };
            })
            .into_js_value()
            .unchecked_ref(),
        )
        .unwrap();

    // Register the resize callback
    window
        .add_event_listener_with_callback(
            "resize",
            Closure::<dyn Fn(Event)>::new(|_: Event| {
                let display = Display::get();
                let Ok(mut display) = display.try_write() else {
                    return;
                };
                let root = display.root().unwrap();
                let (width, height) = (root.client_width(), root.client_height());

                display.text_mode.resize(width as u32, height as u32);
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
