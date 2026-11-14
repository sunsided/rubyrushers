//! The game in a Chromium tab: screenshots in, mouse clicks and key presses out.

use std::ffi::OsStr;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use headless_chrome::browser::tab::point::Point;
use headless_chrome::protocol::cdp::Emulation;
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions, Tab};

use crate::image::Image;

/// A browser showing the game.
pub struct Screen {
    /// Keeps the browser process alive.
    _browser: Browser,
    /// The tab with the game.
    tab: Arc<Tab>,
    /// Screenshot pixels per page (CSS) pixel.
    scale: f64,
}

impl Screen {
    /// Opens `url` in a `width` x `height` window; `visible` shows the window instead of
    /// running headless.
    pub fn open(url: &str, width: u32, height: u32, visible: bool) -> Result<Self> {
        let software = std::env::var_os("BOT_SOFTWARE_GL").is_some();
        let mut args: Vec<&OsStr> = vec![
            // Keep rendering while the window is in the background.
            OsStr::new("--disable-background-timer-throttling"),
            OsStr::new("--disable-renderer-backgrounding"),
        ];
        if software {
            // Software WebGL for machines without a GPU (slow).
            args.extend(["--use-angle=swiftshader", "--enable-unsafe-swiftshader"].map(OsStr::new));
        } else {
            // Headless Chrome's default GL path renders a couple of frames a second;
            // ANGLE on Vulkan runs the game at full speed.
            args.extend(
                [
                    "--use-gl=angle",
                    "--use-angle=vulkan",
                    "--enable-features=Vulkan",
                    "--ignore-gpu-blocklist",
                ]
                .map(OsStr::new),
            );
        }
        let options = LaunchOptions::default_builder()
            .headless(!visible)
            .enable_gpu(!software)
            .window_size(Some((width, height)))
            .args(args)
            .idle_browser_timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| anyhow!("{e}"))?;
        let browser = Browser::new(options).context("start Chromium")?;
        let tab = browser.new_tab()?;
        // The page gets exactly width x height, whatever the window's toolbars take.
        tab.call_method(Emulation::SetDeviceMetricsOverride {
            width,
            height,
            device_scale_factor: 1.0,
            mobile: false,
            scale: None,
            screen_width: None,
            screen_height: None,
            position_x: None,
            position_y: None,
            dont_set_visible_size: None,
            screen_orientation: None,
            viewport: None,
            display_feature: None,
            device_posture: None,
        })?;
        let mut screen = Self {
            _browser: browser,
            tab,
            scale: 1.0,
        };
        screen.goto(url)?;
        let shot = screen.screenshot()?;
        let css_width = screen
            .tab
            .evaluate("window.innerWidth", false)?
            .value
            .and_then(|v| v.as_f64())
            .unwrap_or(shot.width as f64);
        screen.scale = shot.width as f64 / css_width;
        Ok(screen)
    }

    /// Loads `url` in the same tab and waits until the game's canvas is on the page.
    pub fn goto(&self, url: &str) -> Result<()> {
        self.tab.navigate_to(url)?;
        // The game never "finishes loading" in the browser's sense: poll for the canvas.
        for _ in 0..200 {
            let ready = self
                .tab
                .evaluate(
                    "!!document.querySelector('canvas') && document.readyState === 'complete'",
                    false,
                )
                .ok()
                .and_then(|r| r.value)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if ready {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Err(anyhow!("the page at {url} did not load"))
    }

    /// What the page shows now.
    pub fn screenshot(&self) -> Result<Image> {
        Image::from_png(&self.png()?)
    }

    /// What the page shows now, as PNG bytes.
    pub fn png(&self) -> Result<Vec<u8>> {
        self.tab
            .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
    }

    /// Clicks at screenshot pixel `at`.
    pub fn click(&self, at: [f32; 2]) -> Result<()> {
        self.tab.click_point(Point {
            x: f64::from(at[0]) / self.scale,
            y: f64::from(at[1]) / self.scale,
        })?;
        Ok(())
    }

    /// Frames per second the page renders, measured over one second.
    pub fn fps(&self) -> Result<f64> {
        let js = "new Promise(r => { let n = 0; const t0 = performance.now(); \
                  const f = () => { n++; if (performance.now() - t0 < 1000) requestAnimationFrame(f); else r(n); }; \
                  requestAnimationFrame(f); })";
        Ok(self
            .tab
            .evaluate(js, true)?
            .value
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0))
    }

    /// Presses and releases a key, e.g. `"1"` or `"F2"`.
    pub fn key(&self, key: &str) -> Result<()> {
        self.tab.press_key(key)?;
        Ok(())
    }
}
