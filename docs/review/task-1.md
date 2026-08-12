# Shipping completion review

The review captures below were taken from the running ShipYard Tauri app on the virtual desktop.

![Experimental completion animation settings](task-1-experimental-settings.png)

![Completion animation choices](task-1-celebration-variants.jpg)

To exercise the feature, open **Shipyard Settings → Experimental**, choose an option from
**Completion animation**, choose **Fast**, **Normal**, or **Slow** for a full-screen option,
and select **Preview**. The 12 choices are:

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
12. Shipyard sunset

The preview deliberately removes the success copy for full-screen options so the selected
animation is unobstructed; Quiet handoff previews its centered receipt in an isolated scene.
Escape, the corner close button, or the automatic timeout returns to Settings. Preview never
changes project state. Speed applies only to the 11 full-screen options; Quiet handoff keeps a
short, fixed timing. The automatic celebration is shown only after a shipping run exits
successfully; failed, cancelled, blocked, and ordinary script runs stay quiet. Every celebration
fades in and out. Quiet handoff first fades the review surface, then reveals a centered cargo-ship
receipt with the shipped item and destination. ShipYard-managed temporary resolution and pull
request worktrees stay out of Local Work while shipping is in progress.
