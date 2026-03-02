# hatch

A lightweight SSH-based command dispatcher and middleware.

## What it does

`hatch` lets you execute a pre-registered set of commands on your machine over SSH without ever exposing the shell.

## How it works

`hatch` uses SSH's `ForceCommand` mechanism. In your `authorized_keys` file, point a dedicated key to `hatch` binary:

```txt
command="/usr/local/bin/hatch" ssh-ed25519 AAAA...
```

From that point on, any SSH connection using that key is routed through `hatch` instead of a client shell.
When a client runs something like `ssh user@host lock-screen`, `hatch` checks that the command is registered
and executes a binary.

## Configuration

`hatch` is configured via a YAML file:

```yaml
commands:
  lock-screen:
    run: loginctl lock-session
  restart-app:
    run: systemctl restart app
```

## Why not use plain SSH?

Regular SSH key gives full shell access. `hatch` enforces minimal privilege policy
while providing a clean API. It allows you to define a controlled, auditable set of
actions - nothing more.
