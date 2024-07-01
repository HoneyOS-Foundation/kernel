const FONT_SIZE: (u32, u32) = (8, 16);

/// The textmode display
#[derive(Debug)]
pub struct TextMode {
    pub width: u32,
    pub height: u32,
    pub cursor: Cursor,
    pub buffer: Vec<char>,
}

/// The textmode cursor
#[derive(Debug)]
pub struct Cursor {
    pub position: u32,
    pub visible: bool,
}

impl TextMode {
    /// Create a new textmode display
    pub fn new(width: u32, height: u32) -> Self {
        let mut buffer = Vec::new();
        for _ in 0..(width / FONT_SIZE.0 * height / FONT_SIZE.1) {
            buffer.push(' ');
        }
        Self {
            width: width / FONT_SIZE.0,
            height: height / FONT_SIZE.1,
            cursor: Cursor {
                position: 0,
                visible: false,
            },
            buffer,
        }
    }

    /// Resize the textmode display
    /// This will clear the buffer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width / FONT_SIZE.0;
        self.height = height / FONT_SIZE.1;
        self.buffer = Vec::new();
        for _ in 0..(self.width * self.height) {
            self.buffer.push(' ');
        }
        log::info!("{} : {}", self.width, self.height);
    }

    /// Write a character to the display
    /// This will not write if the position is out of bounds
    pub fn write(&mut self, x: u32, c: char) {
        if x < self.width * self.height {
            self.buffer[x as usize] = c;
        }
    }

    /// Write a string to the display
    /// This will not write if the position is out of bounds
    pub fn write_str(&mut self, x: u32, s: &str) {
        for (i, c) in s.chars().enumerate() {
            self.write(x + i as u32, c);
        }
    }

    /// Append a character to the display
    /// This will not write if the position is out of bounds
    /// This will move the cursor to the next position
    pub fn append(&mut self, c: char) {
        let x = self.cursor.position;
        self.write(x, c);
        self.cursor.position = x + 1;
    }

    /// Append a string to the display
    /// This will not write if the position is out of bounds
    /// This will move the cursor to the next position
    pub fn append_str(&mut self, s: &str) {
        for c in s.chars() {
            self.append(c);
        }
    }

    /// Clear the display
    pub fn clear(&mut self) {
        for c in self.buffer.iter_mut() {
            *c = ' ';
        }
        self.cursor.position = 0;
    }

    /// Render the display to a string
    pub fn render(&self) -> String {
        let mut output = String::new();
        for (x, c) in self.buffer.iter().enumerate() {
            if self.cursor.position == x as u32 && self.cursor.visible {
                output.push_str("\x1b[7m");
            }
            output.push(*c);
            if self.cursor.position == x as u32 && self.cursor.visible {
                output.push_str("\x1b[0m");
            }
        }

        let santized = html_escape(&output);
        let transformed = apply_escape_codes(&santized);
        transformed
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
fn apply_escape_codes(input: &str) -> String {
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
            "0" => "font-weight:normal;text-decoration:none;background-color:#000",
            "1" => "font-weight:bold;",
            "4" => "text-decoration:underline;",
            "7" => "background-color: #fff;color:#000;",
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
