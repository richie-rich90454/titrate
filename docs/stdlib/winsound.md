# WinSound

The `tt.os.Winsound` module provides Windows sound playback with `Beep`, `MessageBeep`, and `PlaySound`, plus the `SND_*` play-flag constants and the `MB_*` MessageBeep type constants. It mirrors Python's `winsound` module. On non-Windows systems these become graceful no-ops that return `false` and print a notice; the constants remain defined so portable code can reference them without `#ifdef`s.

## Import

```titrate
import tt::os::Winsound;
```

## Constants

All constants are zero-argument functions returning the platform's numeric value (loaded from `os/winsound.json`).

### `SND_*` play flags

- `SND_ALIAS(): int`, `SND_APPLICATION(): int`, `SND_ALIAS_ID(): int`, `SND_ASYNC(): int`, `SND_FILENAME(): int`, `SND_LOOP(): int`, `SND_MEMORY(): int`, `SND_NODEFAULT(): int`, `SND_NOSTOP(): int`, `SND_NOWAIT(): int`, `SND_PURGE(): int`, `SND_RESOURCE(): int`

### `MB_*` MessageBeep types

- `MB_OK(): int`
- `MB_ICONASTERISK(): int`
- `MB_ICONEXCLAMATION(): int`
- `MB_ICONHAND(): int`
- `MB_ICONQUESTION(): int`

## Functions

### Beep

Generate a tone of `frequency` Hertz for `duration` milliseconds on the PC speaker. `frequency` must be in `37..32767 Hz`. Returns `true` on success and `false` on error or unsupported platform.

**Parameters:** `frequency: int`, `duration: int`
**Returns:** `bool`

```titrate
Beep(880, 250);  // 880 Hz for a quarter second
```

### MessageBeep

Play a Windows system sound identified by `type` (one of the `MB_*` constants). Returns `true` on success and `false` on error.

**Overloads:**
- `MessageBeep(type: int): bool`
- `MessageBeep(): bool` — plays the default OK sound

```titrate
MessageBeep(MB_ICONEXCLAMATION());
```

### PlaySound

Play a waveform-audio sound identified by `sound` (a registry alias when `SND_ALIAS` is set, or a file name when `SND_FILENAME` is set). `flags` is a bitwise OR of `SND_*` constants. Returns `true` on success and `false` on error.

**Overloads:**
- `PlaySound(sound: string, flags: int): bool`
- `PlaySound(sound: string): bool` — defaults to `SND_FILENAME | SND_NODEFAULT`

```titrate
PlaySound("alarm.wav");
PlaySound("SystemExclamation", SND_ALIAS());
```

### StopSound

Stop any currently playing asynchronous sound. Equivalent to `PlaySound(null, SND_PURGE)` on Windows.

**Returns:** `bool`
