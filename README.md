# Timers module

Official rayslash module for timers, reminders, reboot, shutdown, logout, and lock actions. Every action is typed, scheduled by the launcher, and requires explicit activation. No shell is used.

Build the release component with `./scripts/build-release.sh`. The script formats regenerated bindings with the pinned Rust 1.92.0 toolchain and verifies that generation leaves `src/bindings.rs` unchanged.

Examples:

```text
timer 10min take a break
timer take a break 10min
remind me to take a break in 10 minutes
remind me in 10min to take a break
remind in 10min to take a break
shutdown in 10min
```
