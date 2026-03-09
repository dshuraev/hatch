# `hatch` (in active development)

A secure SSH command gateway for controlled remote execution.

## Overview

`hatch` turns SSH into a deterministic command API.

Instead of granting a remote user a full interactive shell, `hatch` allows you to expose a strictly defined, declarative set of commands that can be executed over SSH. Every incoming request is matched exactly against trusted configuration and dispatched locally.

This makes SSH suitable for secure automation endpoints, headless devices, and single-purpose control interfaces.

## What Problem It Solves

Granting SSH access normally means granting shell access. Even when restricted, managing per-key `command=` directives in `authorized_keys` quickly becomes hard to maintain and audit.

`hatch` provides:

* A centralized declarative configuration
* Deterministic exact-match dispatching
* No interactive shell exposure via SSH
* Auditable command surface
* Simple validation before deployment

It replaces ad-hoc SSH wrappers and fragile shell dispatch scripts with a minimal, explicit control layer.

## How It Works

`hatch` uses SSH's `ForceCommand` mechanism.

In your `authorized_keys` file, attach a key to the `hatch` binary:

```txt
command="/usr/local/bin/hatch" ssh-ed25519 AAAA...
```

When a client runs:

```bash
ssh user@host lock-screen
```

SSH exposes `lock-screen` via the `SSH_ORIGINAL_COMMAND` environment variable.

`hatch`:

1. Reads `SSH_ORIGINAL_COMMAND`
2. Loads and validates configuration
3. Performs an exact string match against configured commands
4. Executes the trusted `run` directive locally

The incoming SSH command is treated as untrusted input. The configuration file is trusted owner-controlled data.

On Unix-like systems, commands are currently executed using `/bin/sh -c`.
When the child process exits normally, `hatch` exits with that same status code. If the process has no exit code (for example, terminated by signal), `hatch` exits with failure.

## Logging

Operational logs are emitted only to internal sinks, never to SSH client output.

Configure sink with environment variables:

```txt
HATCH_LOG_SINK=journald   # journald | file | off
HATCH_LOG_FILE=/var/log/hatch/hatch.log   # required when HATCH_LOG_SINK=file
```

Each request gets a random `dispatch_id` shared by startup, config, and dispatch events.
`hatch` logs startup immediately and includes the config path (or `<default>` hint followed by the resolved path event).

When config loading/parsing fails in dispatch mode, the SSH client receives only:

```txt
internal error
```

Detailed diagnostics are written only to the internal log sink.

## Configuration

Configuration is defined in YAML:

```yaml
commands:
  lock-screen:
    run: loginctl lock-session

  restart-app:
    run: systemctl restart app
    timeout: 30
    cwd: /opt/app
    env:
      APP_ENV: production
      LOG_LEVEL: warn
```

Only commands defined under `commands` are executable. Any unmatched request is rejected.
Per-command execution controls are optional:

* `timeout`: positive integer seconds. When exceeded, `hatch` hard-kills the child process.
* `cwd`: absolute path only.
* `env`: mapping of string keys and string values, overlaid on top of the inherited process environment.

By default, configuration is loaded from:

```txt
$XDG_CONFIG_HOME/hatch/hatch.yaml
```

Override with:

```bash
hatch --config ./path/to/config.yaml
hatch -c ./path/to/config.yaml
```

Validate configuration without executing anything:

```bash
hatch check ./path/to/config.yaml
```

## Use Cases

### Headless Linux Devices

Expose a minimal remote control surface without granting shell access.

Examples:

* Reboot device
* Restart service
* Rotate logs
* Trigger maintenance task

### Homelab Automation

Turn SSH into a lightweight command API:

```bash
ssh home restart-firewall
ssh home snapshot-db
ssh home backup-media
```

### Embedded / Edge Systems

Provide deterministic remote control entrypoints for appliances or IoT devices where interactive shells are undesirable.

### Secure CI Triggers

Allow CI systems to invoke specific operations without giving general host access.

## Why Not Just Use `authorized_keys` with `command=`?

OpenSSH already supports per-key command restrictions.

```bash
command="/usr/bin/loginctl lock-session"
```

However:

* Managing many keys duplicates configuration
* There is no centralized command map
* No built-in validation step
* Harder to audit and evolve

`hatch` centralizes policy and execution logic in one controlled location.

## Security Model

* SSH client input is treated as untrusted
* Only exact command matches are allowed
* Command execution is driven by trusted local configuration
* No interactive shell is exposed

This creates a narrow, explicit execution surface instead of a general-purpose login environment.

## Platform Support

Currently targets Unix-like systems with SSH `ForceCommand` support.

Windows support is planned.

## Design Philosophy

`hatch` is intentionally minimal.

It is not:

* A configuration management system
* A distributed orchestrator
* A job scheduler
* A general RPC framework

It is a deterministic SSH command gateway.

Small, explicit, composable.

## Build And Verify

Install the pinned toolchain and repo utilities with:

```bash
mise install
```

Local development and CI use the same `task` targets:

```bash
task ci
```

This fast gate verifies:

* formatting
* clippy with warnings denied
* unit and integration tests
* `cargo-deny`
* locked release build

For the full heavyweight suite, including Miri, fuzzing, coverage, unsafe usage reporting, and size analysis:

```bash
task check
```

## Release Process

Prepare a release locally with:

```bash
task release
```

That command runs the fast gate, checks dependency advisories, and builds an auditable release binary with `cargo-auditable`.

Tagged releases are published by GitHub Actions when a tag matching `v*` is pushed. The release workflow:

* rebuilds the project in release mode with `--locked`
* produces an auditable `hatch` binary
* creates a Linux tarball with `README.md` and `LICENSE`
* publishes SHA-256 checksums

`cargo-bloat` is available as an informational report:

```bash
task bloat
```

Use it before shipping when binary size matters, but it is intentionally not part of the blocking CI gate.

## Contributing

Contributor workflow, coding expectations, and release guidelines are documented in `CONTRIBUTING.md`.
