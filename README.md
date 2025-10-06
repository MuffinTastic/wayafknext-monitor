# WayAFKNext Monitor
Monitor your Wayland desktop for user activity. Intended for use with [WayAFKNext](https://github.com/MuffinTastic/WayAFKNext).

Releases are built automatically for x86_64 and aarch64. See GitHub Actions.

### How it works

It opens a UNIX socket in the same directory as the executable. Communication is done with newline-terminated JSON. This is the API:

Client-to-server:
 - `{"Quit":null}\n`
 - `{"StartWatch":{"status_mins": number, "notifs_mins": number}}\n`
 - `{"StopWatch":null}\n`

Server-to-client:
 - `{"WatchEvent":{"StatusIdle": boolean}}\n`
 - `{"WatchEvent":{"NotifsIdle": boolean}}\n`
 - `{"WatchStarted":{"status_mins": number, "notifs_mins": number}}\n`
 - `{"WatchStopped":null}\n`