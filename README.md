# WayAFKNext Monitor
Monitor your Wayland desktop for user activity. Intended for use with WayAFKNext.

Releases are built automatically for x86_64 and aarch64. See GitHub Actions.

### How it works

It opens a UNIX socket in the same directory as the executable. Communication is done with newline-terminated JSON. This is the API:

Client-to-server:
 - `{"Quit":null}\n`
 - `{"StartWatch":[S,N]}\n`
 - `{"StopWatch":null}\n`

Server-to-client:
 - `{"WatchEvent":{"StatusIdle":<bool>}}\n`
 - `{"WatchEvent":{"NotifsIdle":<bool>}}\n`
 - `{"WatchStarted":[S,N]}\n`
 - `{"WatchStopped":null}\n`

... where
 - `S` = status idle timeout in minutes (integer)
 - `N` = notifications idle timeout in minutes (integer)