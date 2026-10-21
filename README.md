# Vapor Examples

Small source examples shipped with the Vapor app for inspecting the current
workspace, content, and packagepack shapes.

The terminal proof is a deliberately small salvage roguelike: spin an
eight-sector wheel, take salvage or damage, and extract before the hull fails.
A tiny Game Mod adds a ninth bonus sector. Wheel rules and data are separate
Crates; the Engine owns terminal input and rendering.

The playable `hello-world-packagepack` is a non-default composition. Its
executable is a Packagepack-owned composition entrypoint: it installs the
selected Game and Mod into the selected Engine before the Engine begins its
loop. Its Workshop metadata is private and exists for testing Vapor's content
lifecycle. It is example content, not first-party Loo-Cast product content.
