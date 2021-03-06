//! # Alcro
//!
//! Alcro is a library to create desktop apps using rust and modern web technologies.
//! It uses the existing chrome installation for the UI.
//!
//! # Example
//!
//! ```
//! #![windows_subsystem = "windows"]
//! use alcro::{UIBuilder, Content};
//! let ui = UIBuilder::new().content(Content::Html("<html><body>Close Me!</body></html>")).run();
//! assert_eq!(ui.eval("document.body.innerText").unwrap(), "Close Me!");
//! ui.wait_finish();
//! ```

mod chrome;
use chrome::{bind, bounds, eval, load, set_bounds, Chrome};
pub use chrome::{Bounds, JSObject, JSResult, WindowState};
mod locate;
use locate::locate_chrome;
use std::sync::Arc;

const DEFAULT_CHROME_ARGS: &[&str] = &[
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-infobars",
    "--disable-extensions",
    "--disable-features=site-per-process",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--disable-translate",
    "--disable-windows10-custom-titlebar",
    "--metrics-recording-only",
    "--no-first-run",
    "--no-default-browser-check",
    "--safebrowsing-disable-auto-update",
    "--enable-automation",
    "--password-store=basic",
    "--use-mock-keychain",
];

/// The browser window
pub struct UI {
    chrome: Arc<Chrome>,
    _tmpdir: Option<tempdir::TempDir>,
}

impl UI {
    fn new(url: &str, dir: Option<&std::path::Path>, width: i32, height: i32, custom_args: &[&str]) -> UI {
        let _tmpdir:Option<tempdir::TempDir>;
        let dir = match dir {
            Some(dir)=>{
                _tmpdir=None;
                dir
            },
            None=>{
                _tmpdir = Some(tempdir::TempDir::new("alcro").expect("Cannot create temp directory"));
                _tmpdir.as_ref().unwrap().path()
            }
        };       

        let mut args: Vec<String> = Vec::from(DEFAULT_CHROME_ARGS)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        args.push(format!("--app={}", url));
        args.push(format!("--user-data-dir={}", dir.to_str().unwrap()));
        args.push(format!("--window-size={},{}", width, height));
        for arg in custom_args {
            args.push(arg.to_string())
        }
        args.push("--remote-debugging-port=0".to_string());

        let chrome = Chrome::new_with_args(&locate_chrome(), args);
        UI { chrome, _tmpdir }
    }

    /// Returns true if the browser is killed or closed
    pub fn done(&self) -> bool {
        return self.chrome.done();
    }

    /// Wait for the user to close the browser window or for it to be killed
    pub fn wait_finish(&self) {
        self.chrome.wait_finish();
    }

    /// Close the browser window
    pub fn close(&self) {
        self.chrome.kill()
    }

    /// Load a url in the browser. It returns Err if it fails.
    pub fn load(&self, url: &str) -> JSResult {
        return load(self.chrome.clone(), url);
    }

    /// Bind a rust function so that JS code can use it. It returns Err if it fails.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the function
    /// * `f` - The function. It should take a list of `JSObject` as argument and return a `JSResult`
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// use serde_json::to_value;
    ///
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run();
    /// ui.bind("add", |args| {
    ///     let mut sum = 0;
    ///     for arg in args {
    ///         if arg.is_i64() {
    ///             sum += arg.as_i64().unwrap();
    ///         } else {
    ///             return Err(to_value("Not a number").unwrap());
    ///         }
    ///     }
    ///     Ok(to_value(sum).unwrap())
    /// }).unwrap();
    /// assert_eq!(ui.eval("(async () => await add(1,2,3))();").unwrap(), 6);
    /// assert!(ui.eval("(async () => await add(1,2,'hi'))();").is_err());
    /// ```
    pub fn bind<F>(&self, name: &str, f: F) -> JSResult
    where
        F: Fn(&[JSObject]) -> JSResult + Sync + Send + 'static,
    {
        bind(self.chrome.clone(), name, Arc::new(f))
    }

    /// Evaluates js code and returns the result.
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run();
    /// assert_eq!(ui.eval("1+1").unwrap(), 2);
    /// assert_eq!(ui.eval("'Hello'+' World'").unwrap(), "Hello World");
    /// assert!(ui.eval("xfgch").is_err());
    /// ```

    pub fn eval(&self, js: &str) -> JSResult {
        eval(self.chrome.clone(), js)
    }

    /// It changes the size, position or state of the browser window specified by the `Bounds` struct. It returns Err if it fails.
    ///
    /// To change the window state alone use `WindowState::to_bounds()`
    pub fn set_bounds(&self, b: Bounds) -> JSResult {
        set_bounds(self.chrome.clone(), b)
    }

    /// It gets the size, position and state of the browser window. It returns Err if it fails.
    pub fn bounds(&self) -> Result<Bounds, JSObject> {
        bounds(self.chrome.clone())
    }
}

/// Closes the browser window
impl Drop for UI {
    fn drop(&mut self) {
        self.close();
        self.wait_finish();
    }
}

/// Specifies the type of content shown by the browser
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Content<'a> {
    /// The URL
    Url(&'a str),
    /// HTML text
    Html(&'a str),
}

/// Builder for constructing a UI instance.
pub struct UIBuilder<'a> {
    content: Content<'a>,
    dir: Option<&'a std::path::Path>,
    width: i32,
    height: i32,
    custom_args: &'a [&'a str],
}

impl<'a> UIBuilder<'a> {
    /// Default UI
    pub fn new() -> Self {
        UIBuilder {
            content: Content::Html(""),
            dir: None,
            width: 800,
            height: 600,
            custom_args: &[],
        }
    }

    /// Return the UI instance
    pub fn run(&self) -> UI {
        let html: String;
        let url = match self.content {
            Content::Url(u) => u,
            Content::Html(h) => {
                html = format!("data:text/html,{}", h);
                &html
            }
        };
        UI::new(url, self.dir, self.width, self.height, self.custom_args)
    }

    /// Set the content (url or html text)
    pub fn content(&mut self, content: Content<'a>) -> &mut Self {
        self.content = content;
        self
    }

    /// Set the user data directory. By default it is a temporary directory.
    pub fn user_data_dir(&mut self, dir: &'a std::path::Path) -> &mut Self {
        self.dir = Some(dir);
        self
    }

    /// Set the window size
    pub fn size(&mut self, width: i32, height: i32) -> &mut Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Add custom arguments to spawn chrome with
    pub fn custom_args(&mut self, custom_args: &'a [&'a str]) -> &mut Self {
        self.custom_args = custom_args;
        self
    }
}
