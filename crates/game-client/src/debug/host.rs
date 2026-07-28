//! Thin host bridge — JS/agents call the same registry as the console.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::ClientInner;

/// Dev-only handle for `window.__DEBUG__` (or any host script).
#[wasm_bindgen]
pub struct DebugHost {
    inner: Rc<RefCell<ClientInner>>,
}

impl DebugHost {
    pub(crate) fn new(inner: Rc<RefCell<ClientInner>>) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen]
impl DebugHost {
    /// Run a console line (same path as the in-engine shell).
    #[wasm_bindgen]
    pub fn exec(&self, line: &str) -> String {
        self.inner.borrow_mut().debug_execute(line)
    }

    /// Whether the debug shell is open.
    #[wasm_bindgen(js_name = isOpen)]
    pub fn is_open(&self) -> bool {
        self.inner.borrow().debug.is_open()
    }

    /// Open or close the shell without typing backtick.
    #[wasm_bindgen]
    pub fn set_open(&self, open: bool) {
        self.inner.borrow_mut().debug.shell.open = open;
        if open {
            self.inner.borrow_mut().debug.shell.focus_input = true;
        }
    }

    /// Queue a frame capture (same path as F9 / `screenshot` command).
    #[wasm_bindgen]
    pub fn screenshot(&self) -> String {
        self.inner.borrow_mut().debug.request_screenshot();
        "screenshot queued".into()
    }
}
