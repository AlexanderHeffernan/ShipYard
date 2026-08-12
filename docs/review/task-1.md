# Shipping completion review

The review captures below were taken from the running ShipYard Tauri app on the virtual desktop.

![Experimental completion animation settings](task-1-experimental-settings.png)

![All 11 completion animation variants](task-1-celebration-variants.jpg)

To exercise the feature, open **Shipyard Settings → Experimental**, choose an option from
**Completion animation**, and select **Preview**. The 11 choices are:

1. Quiet handoff (default; non-full-screen)
2. Sail away
3. Lighthouse beam
4. Confetti burst
5. Constellation route
6. Tidal rings
7. Dock stamp
8. Sunrise
9. Paper fleet
10. Firework sky
11. Signal path

The preview deliberately removes the center receipt so the selected animation is unobstructed;
Escape, the corner close button, or the automatic timeout returns to Settings. Preview never
changes project state. The automatic celebration is shown only after a shipping run exits
successfully; failed, cancelled, blocked, and ordinary script runs stay quiet. Every celebration
fades in and out, and the quiet handoff also fades the review surface away before returning it.
