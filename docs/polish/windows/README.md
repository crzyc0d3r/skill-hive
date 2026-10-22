Windows navigation frames are captured by the Windows tauri-driver job.

The job walks Library, Settings, Add skills (Marketplace, From a folder, From Git, Found on this computer), and back to the Library, in Light and then Dark. Before each frame it checks that `html` actually has or lacks the `dark` class. For about two seconds on every screen it samples the page element (`library`, `settings`, or `install`), the same element the Playwright test measures, and fails if that top moves by more than one pixel. Measuring the scroll container misses a banner that sits inside it. The PNGs and `tops.json` from the job are uploaded as the `windows-navigation` artifact.

The PNGs checked in here are a few frames from the Windows run of `b06b9da` so the layout is visible without downloading the artifact. That run still recorded the scroll container. The landmark sampler replaces that check.
