#!/usr/bin/env python3
"""Regression tests for `enable -d flyline` crash fixes.

flyline installs itself as bash's input stream.  Unloading the builtin with
`enable -d flyline` used to pop the *wrong* input stream whenever the unload
ran from inside a nested input source (eval, source, command substitution),
and bash would then resume reading from freed memory or from callbacks in a
library it was about to dlclose(), segfaulting the shell.

These tests drive an interactive bash through a PTY and verify the shell
survives every unload scenario:

  1. manual `enable -d flyline`
  2. `eval "enable -d flyline"`
  3. `source` of a script containing `enable -d flyline`
  4. command substitution containing `enable -d flyline`
  5. `eval "enable -d flyline && enable -f ... flyline"` (unload + reload)
  6. unload from inside a `runBashCommand` key binding (flyline code is on the
     call stack when bash dlcloses the library; teardown is deferred to the
     next input read)

Usage:
    python3 tests/unload_regression.py [path/to/libflyline.so]

Requires: a POSIX system with a PTY and an interactive bash build that can
load the flyline builtin.
"""

import os
import fcntl
import pty
import re
import select
import struct
import sys
import termios
import time

PROMPT = "bash-5.3# "


def read_until(fd, needle, timeout=20.0):
    data = b""
    deadline = time.time() + timeout
    while time.time() < deadline:
        readable, _, _ = select.select([fd], [], [], 0.2)
        if readable:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            data += chunk
            if needle.encode() in data:
                return data
    return data


def send(fd, text, wait=0.3):
    os.write(fd, text.encode())
    time.sleep(wait)


def is_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def wait_prompt(fd):
    return read_until(fd, PROMPT)


def strip_ansi(text):
    return re.sub(
        r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07|\x1b[()][0-9A-Z]|\x1b[=>]",
        "",
        text,
    )


def wait_for_markers(fd, markers, timeout=25.0):
    """Read until every marker appears in the (ANSI-stripped) output."""
    data = b""
    deadline = time.time() + timeout
    while time.time() < deadline:
        readable, _, _ = select.select([fd], [], [], 0.2)
        if readable:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            data += chunk
        text = strip_ansi(data.decode(errors="replace"))
        if all(marker in text for marker in markers):
            return data
    return data


def drain(fd, seconds=2.0):
    end = time.time() + seconds
    while time.time() < end:
        readable, _, _ = select.select([fd], [], [], 0.2)
        if readable:
            try:
                os.read(fd, 65536)
            except OSError:
                break


def scenario(fd, pid, name, command, check_lines):
    send(fd, command + "\n")
    out = wait_for_markers(fd, check_lines)
    alive = is_alive(pid)
    text = strip_ansi(out.decode(errors="replace"))
    missing = [line for line in check_lines if line not in text]
    ok = alive and not missing
    print(f"[{'OK' if ok else 'FAIL'}] {name} (alive={alive})")
    for line in missing:
        print(f"    missing marker: {line!r}")
    return alive


def main():
    flyline_so = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "target/debug/libflyline.so",
    )

    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("bash", ["bash", "--noprofile", "--norc", "-i"])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

    wait_prompt(fd)
    alive = is_alive(pid)
    if not alive:
        print("[FAIL] could not start interactive bash")
        return 1

    def load():
        send(fd, f"enable -f {flyline_so} flyline\n")
        drain(fd, 2.0)
        return is_alive(pid)

    if not load():
        print("[FAIL] flyline load crashed bash")
        return 1
    print("[OK] flyline loaded")

    results = []

    results.append(
        scenario(
            fd,
            pid,
            "manual enable -d flyline",
            "enable -d flyline; enable -p | grep -q flyline || echo MANUAL_UNLOADED",
            ["MANUAL_UNLOADED"],
        )
    )

    load()
    results.append(
        scenario(
            fd,
            pid,
            "eval enable -d flyline",
            'eval "enable -d flyline"; enable -p | grep -q flyline || echo EVAL_UNLOADED',
            ["EVAL_UNLOADED"],
        )
    )

    load()
    results.append(
        scenario(
            fd,
            pid,
            "source enable -d flyline",
            "source <(echo 'enable -d flyline'); "
            "enable -p | grep -q flyline || echo SOURCE_UNLOADED",
            ["SOURCE_UNLOADED"],
        )
    )

    load()
    results.append(
        scenario(
            fd,
            pid,
            "command substitution enable -d flyline",
            'x=$(enable -d flyline; echo subst-done); echo "$x"; '
            "enable -p | grep -q flyline || echo SUBST_UNLOADED",
            ["subst-done", "SUBST_UNLOADED"],
        )
    )

    load()
    results.append(
        scenario(
            fd,
            pid,
            "nested eval -> source -> enable -d flyline",
            'eval "source <(echo \'enable -d flyline\')"; '
            "enable -p | grep -q flyline || echo NESTED_UNLOADED",
            ["NESTED_UNLOADED"],
        )
    )

    load()
    results.append(
        scenario(
            fd,
            pid,
            "eval unload + immediate reload",
            f'eval "enable -d flyline && enable -f {flyline_so} flyline"; '
            "enable -p | grep -q flyline && echo RELOADED",
            ["RELOADED"],
        )
    )

    results.append(
        scenario(
            fd,
            pid,
            "manual enable -d after reload",
            "enable -d flyline; enable -p | grep -q flyline || echo FINAL_UNLOADED",
            ["FINAL_UNLOADED"],
        )
    )

    # Key binding: unload runs while flyline code is on the call stack.
    if not load():
        print("[FAIL] flyline reload crashed bash before key binding test")
        return 1
    send(fd, 'flyline key bind Ctrl+g \'always=runBashCommand("enable -d flyline")\'\n')
    drain(fd, 1.5)
    os.write(fd, b"\x07")  # Ctrl+g triggers the binding
    time.sleep(1.0)
    os.write(fd, b"\r")  # next read finishes the deferred teardown
    send(fd, "enable -p | grep -q flyline || echo KEYBIND_UNLOADED\n")
    out = wait_for_markers(fd, ["KEYBIND_UNLOADED"])
    alive = is_alive(pid)
    ok = alive and "KEYBIND_UNLOADED" in strip_ansi(out.decode(errors="replace"))
    print(f"[{'OK' if ok else 'FAIL'}] runBashCommand key binding unload (alive={alive})")
    results.append(alive and ok)

    # Reload after deferred unload and confirm flyline works again.
    ok = load()
    if ok:
        send(fd, "enable -p | grep -q flyline && echo RELOADED_AFTER_KEYBIND\n")
        out = wait_for_markers(fd, ["RELOADED_AFTER_KEYBIND"])
        ok = "RELOADED_AFTER_KEYBIND" in strip_ansi(out.decode(errors="replace"))
    print(f"[{'OK' if ok else 'FAIL'}] reload after key binding unload")
    results.append(ok)

    try:
        os.write(fd, b"exit\n")
        os.close(fd)
    except OSError:
        pass

    print()
    print("ALL PASS" if all(results) else "FAILURES PRESENT")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
