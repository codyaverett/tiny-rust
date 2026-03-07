# 24-tiny-sandbox

Chroot jail with privilege drop and execve.

## Technique

Uses the `no_std` + libc pattern from example 03. Forks a child process that
performs container-like isolation: `chroot()` to restrict filesystem view,
`chdir("/")` to avoid escaping the jail, `setgid()`/`setuid()` to drop
privileges, then `execve()` to run a command in the sandbox.

Critical ordering: chroot -> chdir -> setgid -> setuid -> execve
(setgid must come before setuid because after dropping to unprivileged uid
we lose the ability to change gid).

## New syscalls

- `execve` -- replace process image with new program
- `chroot` -- change root directory
- `chdir` -- change working directory
- `setuid` -- set user ID (drop privileges)
- `setgid` -- set group ID (drop privileges)

## Usage

```sh
cargo build --release

# Show usage (no root needed):
./target/release/tiny-sandbox

# With root (create a minimal jail):
sudo mkdir -p /tmp/jail/bin
sudo cp /bin/ls /tmp/jail/bin/
sudo ./target/release/tiny-sandbox /tmp/jail 1000 /bin/ls /
# [sandbox] chroot=/tmp/jail uid=1000 cmd=/bin/ls
# bin
# [sandbox] child exited with status 0
```

Note: `chroot` requires root privileges. Running unprivileged will report the error.
