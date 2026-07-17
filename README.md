# Vapor Examples

Small source examples shipped with the Vapor app so a new developer can inspect
the current workspace/project/content shape without needing the full Vapor-Root
checkout.

This workspace contains two groups of registered Vapor content projects.

Minimal shape examples:

- `basic_engine`
- `basic_game`
- `basic_packagepack`

Terminal runtime proof:

- `terminal_engine`
- `hello_world_game`
- `hello_world_packagepack`

The basic examples are deliberately minimal. The terminal proof is the
"hello world on steroids" example: a small engine binary loads a game library
through Vapor's installed packagepack composition and runs a terminal game loop.
It is example content, not first-party Loo-Cast product content.

For a new publishable workspace, use the Vapor Shell template command instead of
editing these files in place:

```text
source init basic-content /path/to/my-content --organization my-studio --name my-content
content validate
content deploy my-studio/my-content/my-content-packagepack --select
```
