//! The game in a Chrome page: screenshots in, mouse clicks out.

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use serde_json::json;

use crate::cdp::Cdp;
use crate::image::Image;

/// A browser showing the game.
pub struct Screen {
    /// The DevTools connection.
    cdp: Cdp,
    /// Screenshot pixels per page (CSS) pixel.
    scale: f64,
    /// Capture only this rectangle (screenshot pixels: x, y, width, height).
    clip: Option<[f32; 4]>,
}

impl Screen {
    /// Opens `url` with a `width` x `height` page; `visible` shows the window instead of
    /// running headless.
    pub fn open(url: &str, width: u32, height: u32, visible: bool) -> Result<Self> {
        let mut args = vec![
            format!("--window-size={width},{height}"),
            // Keep rendering while the window is in the background.
            "--disable-background-timer-throttling".into(),
            "--disable-renderer-backgrounding".into(),
            "--disable-backgrounding-occluded-windows".into(),
        ];
        if !visible {
            args.push("--headless=new".into());
        }
        if std::env::var_os("BOT_SOFTWARE_GL").is_some() {
            // Software WebGL for machines without a GPU (slow).
            args.extend(["--use-angle=swiftshader", "--enable-unsafe-swiftshader"].map(Into::into));
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
                .map(Into::into),
            );
        }
        let mut cdp = Cdp::launch(&args)?;
        // The page gets exactly width x height, whatever the window's toolbars take.
        cdp.call(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false }),
        )?;
        let mut screen = Self {
            cdp,
            scale: 1.0,
            clip: None,
        };
        screen.goto(url)?;
        let shot = screen.screenshot()?;
        let css_width = screen.eval("window.innerWidth", false)?;
        screen.scale = shot.width as f64 / css_width.as_f64().unwrap_or(shot.width as f64);
        Ok(screen)
    }

    /// Loads `url` and waits until the game's canvas is on the page.
    pub fn goto(&mut self, url: &str) -> Result<()> {
        self.cdp.call("Page.navigate", json!({ "url": url }))?;
        let ready = "!!document.querySelector('canvas') && document.readyState === 'complete'";
        for _ in 0..200 {
            if self.eval(ready, false)?.as_bool() == Some(true) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Err(anyhow!("the page at {url} did not load"))
    }

    /// Evaluates a JavaScript expression (awaiting it if it is a promise) and returns its
    /// value.
    fn eval(&mut self, expression: &str, promise: bool) -> Result<serde_json::Value> {
        let r = self.cdp.call(
            "Runtime.evaluate",
            json!({ "expression": expression, "returnByValue": true, "awaitPromise": promise }),
        )?;
        Ok(r["result"]["value"].clone())
    }

    /// What the page shows now.
    pub fn screenshot(&mut self) -> Result<Image> {
        Image::from_png(&self.png()?)
    }

    /// From now on, capture only `rect` (screenshot pixels: x, y, width, height); `None`
    /// for the whole page. Screenshots then start at the rectangle's corner.
    pub fn set_clip(&mut self, rect: Option<[f32; 4]>) {
        self.clip = rect;
    }

    /// What the page shows now (or the clip rectangle), as PNG bytes.
    pub fn png(&mut self) -> Result<Vec<u8>> {
        let mut params = json!({ "format": "png", "fromSurface": true, "optimizeForSpeed": true });
        if let Some([x, y, w, h]) = self.clip {
            let s = self.scale as f32;
            params["clip"] =
                json!({ "x": x / s, "y": y / s, "width": w / s, "height": h / s, "scale": 1 });
        }
        let r = self.cdp.call("Page.captureScreenshot", params)?;
        let data = r["data"].as_str().context("no screenshot data")?;
        Ok(base64::engine::general_purpose::STANDARD.decode(data)?)
    }

    /// Clicks at screenshot pixel `at`.
    pub fn click(&mut self, at: [f32; 2]) -> Result<()> {
        let (x, y) = (f64::from(at[0]) / self.scale, f64::from(at[1]) / self.scale);
        for (kind, buttons) in [("mouseMoved", 0), ("mousePressed", 1), ("mouseReleased", 0)] {
            self.cdp.call(
                "Input.dispatchMouseEvent",
                json!({ "type": kind, "x": x, "y": y, "button": "left", "buttons": buttons, "clickCount": 1 }),
            )?;
        }
        Ok(())
    }

    /// Frames per second the page renders, measured over one second.
    pub fn fps(&mut self) -> Result<f64> {
        let js = "new Promise(r => { let n = 0; const t0 = performance.now(); \
                  const f = () => { n++; if (performance.now() - t0 < 1000) requestAnimationFrame(f); else r(n); }; \
                  requestAnimationFrame(f); })";
        Ok(self.eval(js, true)?.as_f64().unwrap_or(0.0))
    }
}
