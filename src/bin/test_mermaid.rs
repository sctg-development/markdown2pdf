use escape_string::escape;
use headless_chrome::{Browser, Tab};
use unescape::unescape;
use std::{error::Error, fmt::Display, sync::Arc};

/// Error thrown if Mermaid is unable to compile the diagram
#[derive(Debug)]
pub struct CompileError;

impl Error for CompileError {
    fn description(&self) -> &str {
        "Error occured while compiling the diagram!"
    }
}

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompileError")
    }
}

/// The Mermaid struct holds the embedded Chromium instance that is used to render Mermaid
/// diagrams
#[derive(Clone)]
pub struct Mermaid {
    _browser: Browser,
    tab: Arc<Tab>,
}

impl Mermaid {
    /// Initializes Mermaid
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let browser = Browser::default()?;
        let mermaid_js = include_str!("../../doc_helper/mermaid/mermaid.min.js");
        let html_payload = include_str!("../../doc_helper/mermaid/index.html");

        let tab = browser.new_tab()?;
        tab.navigate_to(&format!("data:text/html;charset=utf-8,{}", html_payload))?;
        tab.evaluate(mermaid_js, false)?;

        Ok(Self {
            _browser: browser,
            tab,
        })
    }

    /// Renders a diagram
    ///
    /// # Example:
    /// ```
    /// let mermaid = Mermaid::new();
    /// let svg = mermaid.render(r#"graph TB\na-->b").expect("Unable to render!"#);
    /// ```
    pub fn render(&self, input: &str) -> Result<String, Box<dyn Error>> {
        let data = self
            .tab
            .evaluate(&format!("render('{}')", escape(input)), true)?;
        let string = data.value.unwrap_or_default().to_string();
        let slice = unescape(string.trim_matches('"')).unwrap_or_default();

        if slice == "null" {
            return Err(Box::new(CompileError));
        }

        Ok(slice.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_mermaid_instance_without_crashing() {
        let mermaid = Mermaid::new();
        assert!(mermaid.is_ok());
    }

    #[test]
    fn render_mermaid() {
        let mermaid = Mermaid::new().unwrap();
        let rendered = mermaid.render("graph TB\na-->b");
        assert!(rendered.is_ok());
        // TODO: Perform visual image comparison
        assert!(rendered.unwrap().starts_with("<svg"));
    }

    #[test]
    fn syntax_error() {
        let mermaid = Mermaid::new().unwrap();
        let rendered = mermaid.render("grph TB\na-->b");
        assert!(rendered.is_err());
    }
}

fn main() {
    let mermaid = Mermaid::new().unwrap();
    println!("{}", mermaid.render(r#"mindmap
  root((System Features))
    Signal Processing
      Real-time FFT
      Butterworth filters
      Differential subtraction
      Peak detection
    Hardware Control
      USB-HID interface
      I2C thermal sensors
      SPI DAC/DDS control
      Modbus TCP server
    Web Interface
      Real-time streaming
      OAuth2/JWT security
      Multi-language UI
      Interactive graphs
    Extensibility
      Python integration
      Plugin drivers
      Hot-reload config
      REST API"#).unwrap());
}