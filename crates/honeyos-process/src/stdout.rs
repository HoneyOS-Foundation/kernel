use anyhow::anyhow;
use honeyos_atomics::rwlock::SpinRwLock;
use std::sync::{Arc, Mutex, RwLock};

/// A message sent to stdout
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdoutMessage {
    String(String),
    Clear,
    ClearLine,
    ClearLines(u32),
}

/// StdOut is a struct that represents a standard output stream of a shell
#[derive(Debug)]
pub struct ProcessStdOut {
    process_buffer: Arc<Mutex<Vec<StdoutMessage>>>, // The process-side buffer
    eventual_buffer: RwLock<String>,                // The eventual buffer
}

impl ProcessStdOut {
    pub fn new() -> Self {
        Self {
            process_buffer: Arc::new(Mutex::new(Vec::new())),
            eventual_buffer: RwLock::new(String::new()),
        }
    }

    /// Write a string to the kernel buffer. Locks the buffer
    pub fn write(&self, string: impl Into<String>) -> anyhow::Result<()> {
        let string = string.into();
        let mut process_buffer = self
            .process_buffer
            .try_lock()
            .map_err(|e| anyhow!("Failed to lock stdout buffer: {}", e))?;
        process_buffer.push(StdoutMessage::String(string));
        Ok(())
    }

    /// Write a line to the kernel buffer
    pub fn writeln(&self, string: impl Into<String>) -> anyhow::Result<()> {
        let string = string.into();
        let string = format!("{}\n", string);
        let mut process_buffer = self
            .process_buffer
            .try_lock()
            .map_err(|e| anyhow!("Failed to lock stdout buffer: {}", e))?;
        process_buffer.push(StdoutMessage::String(string));
        Ok(())
    }

    /// Sync buffer to the local copy
    pub fn sync(&self) {
        // Get the eventual buffer
        let mut eventual_buffer;
        loop {
            let Ok(e) = self.eventual_buffer.try_write() else {
                continue;
            };
            eventual_buffer = e;
            break;
        }

        if let Ok(mut process_buffer) = self.process_buffer.try_lock() {
            for message in process_buffer.iter() {
                match message {
                    StdoutMessage::String(s) => {
                        *eventual_buffer = format!("{}{}", eventual_buffer, &s)
                    }
                    StdoutMessage::Clear => eventual_buffer.clear(),
                    // Clear the last line
                    StdoutMessage::ClearLine => {
                        let mut lines = eventual_buffer
                            .split("\n")
                            .filter(|c| *c != "")
                            .collect::<Vec<&str>>();
                        if lines.len() >= 1 {
                            lines.remove(lines.len() - 1);
                            let mut result = String::new();
                            for line in lines {
                                result = format!("{}{}\n", result, line);
                            }
                            *eventual_buffer = result;
                        }
                    }
                    StdoutMessage::ClearLines(num) => {
                        let mut lines = eventual_buffer
                            .split("\n")
                            .filter(|c| *c != "")
                            .collect::<Vec<&str>>();

                        for _ in 0..*num {
                            if lines.len() <= 0 {
                                break;
                            }
                            lines.remove(lines.len() - 1);
                        }

                        let mut result = String::new();
                        for line in lines {
                            result = format!("{}{}\n", result, line);
                        }
                        *eventual_buffer = result;
                    }
                }
            }
            process_buffer.clear();
        }
    }

    /// Clear the local buffer
    pub fn clear(&self) {
        let mut eventual_buffer = self.eventual_buffer.spin_write().unwrap();
        eventual_buffer.clear();
    }

    /// Clear N number of lines in the processes's stdout.
    /// Will only clear up to the amount of lines.
    pub fn clear_lines(&self, num: u32) {
        let mut process_buffer = self.process_buffer.lock().unwrap();
        process_buffer.push(StdoutMessage::ClearLines(num));
    }

    /// Return the local buffer
    pub fn buffer(&self) -> String {
        let eventual_buffer = self.eventual_buffer.spin_read().unwrap();
        eventual_buffer.clone()
    }

    /// Return an arc reference to the process buffer
    pub fn process_buffer(&self) -> Arc<Mutex<Vec<StdoutMessage>>> {
        self.process_buffer.clone()
    }
}
