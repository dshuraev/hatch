# hatch

A lightweight SSH-based command dispatcher and middleware.

## What it does

`hatch` lets you execute a pre-registered set of commands on your machine over SSH without exposing an interactive shell to the SSH client.

## How it works

`hatch` uses SSH's `ForceCommand` mechanism. In your `authorized_keys` file, point a dedicated key to `hatch` binary:

```txt
command="/usr/local/bin/hatch" ssh-ed25519 AAAA...
```

From that point on, any SSH connection using that key is routed through `hatch` instead of a client shell.

By default, `hatch` reads its config from `$XDG_CONFIG_HOME/hatch/hatch.yaml`. You can override that path with
`hatch --config ./path/to/config.yaml` or `hatch -c ./path/to/config.yaml`.

When a client runs something like `ssh user@host lock-screen`, SSH exposes `lock-screen` through the
`SSH_ORIGINAL_COMMAND` environment variable. `hatch` reads that value, validates the config, and matches the
original command exactly against a key under `commands`.

If a matching entry is found, `hatch` executes its `run` value via the platform shell. On Unix-like systems this is currently `/bin/sh -c`. The `run` value is treated as trusted machine-owner configuration, while the incoming SSH command is matched exactly against configured keys. For example, this config:

```yaml
commands:
  lock-screen:
    run: loginctl lock-session
```

allows `ssh user@host lock-screen` to dispatch `loginctl lock-session`.

`hatch check ./path/to/config.yaml` validates a config file without dispatching any command.

## Configuration

`hatch` is configured via a YAML file:

```yaml
commands:
  lock-screen:
    run: loginctl lock-session
  restart-app:
    run: systemctl restart app
```

## Platform support

`hatch` currently targets Unix-like environments with SSH `ForceCommand` support. Windows support is planned, but the current setup instructions and examples assume a Unix-like host.

## Why not use plain SSH?

Regular SSH key gives full shell access. `hatch` narrows that down to an exact-match command dispatcher backed by trusted local configuration, giving you a controlled, auditable set of actions instead of a general interactive login shell.
