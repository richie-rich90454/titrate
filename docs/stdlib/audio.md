# audio

The `tt.audio` module provides audio processing utilities including buffer manipulation, WAV file I/O, pitch detection, and MFCC feature extraction.

```titrate
import tt.audio.AudioBuffer;
import tt.audio.WavReader;
import tt.audio.WavWriter;
import tt.audio.Pitch;
import tt.audio.Mfcc;
```

## AudioBuffer

Audio buffer with sample rate, channel, and duration management. Supports normalization, fading, mixing, splicing, and reversal.

- `fn init(sampleRate: int, channels: int)` — create an empty audio buffer
- `fn init(sampleRate: int, channels: int, samples: ArrayList<double>)` — create a buffer from sample data
- `getSampleRate(): int` — sample rate in Hz
- `getChannels(): int` — number of audio channels
- `getNumSamples(): int` — total number of samples
- `getDuration(): double` — duration in seconds
- `getSample(index: int): double` — get sample at index
- `setSample(index: int, value: double): void` — set sample at index
- `getChannel(channel: int): ArrayList<double>` — extract a single channel
- `setChannel(channel: int, data: ArrayList<double>): void` — replace a channel's samples
- `normalize(): void` — normalize samples to [-1.0, 1.0] range
- `normalizeTo(target: double): void` — normalize to a specific peak amplitude
- `fadeIn(duration: double): void` — apply linear fade-in over given seconds
- `fadeOut(duration: double): void` — apply linear fade-out over given seconds
- `fadeInSamples(numSamples: int): void` — apply fade-in over a specific number of samples
- `fadeOutSamples(numSamples: int): void` — apply fade-out over a specific number of samples
- `mix(other: AudioBuffer): AudioBuffer` — mix (add) two audio buffers
- `splice(startSample: int, endSample: int): AudioBuffer` — extract a sub-buffer
- `append(other: AudioBuffer): void` — concatenate another buffer to the end
- `reverse(): void` — reverse the audio buffer in-place
- `copy(): AudioBuffer` — deep copy of the buffer
- `toMono(): AudioBuffer` — convert to mono by averaging channels
- `resample(targetRate: int): AudioBuffer` — resample to a different sample rate

```titrate
let buf = new AudioBuffer(44100, 1);

// Load samples (e.g., from a generated tone)
for (let i = 0; i < 44100; i++) {
    let t = Double.parseDouble(Integer.toString(i)) / 44100.0;
    buf.setSample(i, MathTrig.sin(2.0 * Math.PI() * 440.0 * t));
}

io::println("Duration: " + Double.toString(buf.getDuration()));  // 1.0

// Normalize and apply fade
buf.normalize();
buf.fadeIn(0.05);
buf.fadeOut(0.1);

// Extract a segment
let segment = buf.splice(22050, 44100);

// Reverse for special effect
segment.reverse();
```

## WavReader

Read WAV audio files with support for PCM formats and multiple bit depths.

- `fn init()` — create a WavReader instance
- `read(path: string): AudioBuffer` — read a WAV file and return an AudioBuffer
- `getBitDepth(): int` — bit depth of the last read file (8, 16, 24, or 32)
- `getSampleRate(): int` — sample rate of the last read file
- `getChannels(): int` — number of channels of the last read file
- `getNumFrames(): int` — number of frames in the last read file
- `isPCM(): bool` — whether the last read file uses PCM format

```titrate
let reader = new WavReader();
let audio = reader.read("recording.wav");

io::println("Sample rate: " + Integer.toString(reader.getSampleRate()));
io::println("Channels: " + Integer.toString(reader.getChannels()));
io::println("Bit depth: " + Integer.toString(reader.getBitDepth()));
io::println("Duration: " + Double.toString(audio.getDuration()) + "s");
```

## WavWriter

Write WAV audio files with configurable PCM format and bit depth.

- `fn init()` — create a WavWriter instance
- `write(path: string, buffer: AudioBuffer, bitDepth: int): void` — write an AudioBuffer to a WAV file (bitDepth: 8, 16, 24, or 32)
- `write16(path: string, buffer: AudioBuffer): void` — write as 16-bit PCM (convenience method)
- `write24(path: string, buffer: AudioBuffer): void` — write as 24-bit PCM (convenience method)
- `write32(path: string, buffer: AudioBuffer): void` — write as 32-bit PCM (convenience method)

```titrate
let writer = new WavWriter();

// Write as 16-bit WAV (standard CD quality)
writer.write16("output.wav", audio);

// Write as 24-bit WAV (higher quality)
writer.write24("output_hq.wav", audio);

// Write with explicit bit depth
writer.write("custom.wav", audio, 32);
```

## Pitch

Pitch detection algorithms for fundamental frequency estimation from audio signals.

- `fn init(sampleRate: int)` — create a Pitch detector with the given sample rate
- `autocorrelation(signal: ArrayList<double>): double` — pitch detection via autocorrelation
- `yin(signal: ArrayList<double>, threshold: double): double` — pitch detection using the YIN algorithm (threshold typically 0.1–0.3)
- `harmonicProductSpectrum(signal: ArrayList<double>, maxHarmonics: int): double` — pitch detection via harmonic product spectrum
- `detect(signal: ArrayList<double>): double` — best-effort pitch detection using a combined approach
- `getFrequency(signal: ArrayList<double>): double` — alias for `detect`
- `isVoiced(signal: ArrayList<double>, threshold: double): bool` — determine if a signal segment contains voiced speech/audio

```titrate
let detector = new Pitch(44100);

// Read a segment of audio
let segment = audio.splice(0, 4096);
let samples = segment.getChannel(0);

// YIN algorithm (recommended for monophonic pitch)
let freq = detector.yin(samples, 0.15);
io::println("Detected pitch: " + Double.toString(freq) + " Hz");

// Autocorrelation method
let freqAC = detector.autocorrelation(samples);

// Harmonic product spectrum
let freqHPS = detector.harmonicProductSpectrum(samples, 5);

// Check if segment is voiced
if (detector.isVoiced(samples, 0.3)) {
    io::println("Voiced segment detected");
}
```

## Mfcc

Mel-Frequency Cepstral Coefficients computation for speech and audio feature extraction. Implements the full MFCC pipeline: pre-emphasis, framing, windowing, FFT, mel filterbank, DCT, and delta features.

- `fn init(sampleRate: int, numCoefficients: int)` — create an MFCC extractor (typically 13 coefficients)
- `fn init(sampleRate: int, numCoefficients: int, numMelFilters: int, fftSize: int)` — create with custom mel filter count and FFT size
- `compute(signal: ArrayList<double>): ArrayList<ArrayList<double>>` — compute MFCCs for the entire signal, returns per-frame coefficient vectors
- `preEmphasis(signal: ArrayList<double>, coefficient: double): ArrayList<double>` — apply pre-emphasis filter (typically coefficient = 0.97)
- `frameSignal(signal: ArrayList<double>, frameSize: int, hopSize: int): ArrayList<ArrayList<double>>` — split signal into overlapping frames
- `applyWindow(frame: ArrayList<double>, windowType: string): ArrayList<double>` — apply a window function to a frame
- `computeMelFilterbank(numFilters: int, fftSize: int, sampleRate: int): ArrayList<ArrayList<double>>` — generate mel-spaced filterbank matrix
- `dct2(signal: ArrayList<double>): ArrayList<double>` — type-II DCT for cepstral computation
- `delta(features: ArrayList<ArrayList<double>>, order: int): ArrayList<ArrayList<double>>` — compute delta (order=1) or delta-delta (order=2) features

```titrate
let mfcc = new Mfcc(44100, 13);

// Compute MFCCs from audio
let audio = reader.read("speech.wav");
let samples = audio.getChannel(0);
let features = mfcc.compute(samples);

io::println("Frames: " + Integer.toString(features.size()));
io::println("Coefficients per frame: " + Integer.toString(features.get(0).size()));

// Compute delta and delta-delta features
let delta1 = mfcc.delta(features, 1);
let delta2 = mfcc.delta(features, 2);

// Manual pipeline: pre-emphasis → frame → window
let emphasized = mfcc.preEmphasis(samples, 0.97);
let frames = mfcc.frameSignal(emphasized, 1024, 512);
let windowed = mfcc.applyWindow(frames.get(0), "hamming");
```

## Sun AU format and audio sniffing (Phase 1-2 parity)

### Sun AU reader/writer

The `AuReader` and `AuWriter` classes provide I/O for the Sun/NeXT AU audio container format (`.au`/`.snd`). This format is the historical Unix audio interchange format.

- `AuReader.read(path: string): AudioBuffer` — read a Sun AU file into an `AudioBuffer`
- `AuReader.getSampleRate(): int` — sample rate of the last read file
- `AuReader.getChannels(): int` — channel count of the last read file
- `AuWriter.write(path: string, buffer: AudioBuffer): void` — write an `AudioBuffer` to a Sun AU file
- `AuWriter.write(path: string, buffer: AudioBuffer, encoding: string): void` — write with a specific encoding (`"mulaw"`, `"pcm8"`, `"pcm16"`, `"pcm24"`, `"pcm32"`, `"float"`)

```titrate
import tt.audio.AuReader;
import tt.audio.AuWriter;

let au = new AuReader().read("chime.au");
io::println(Integer.toString(au.getSampleRate()));  // e.g. 8000 (common for µ-law)

new AuWriter().write("copy.au", au, "mulaw");
```

### sndhdr `what()` sniffer

The `Sndhdr` module mirrors Python's `sndhdr` — it inspects a file's magic bytes and reports the audio format. It returns the detected type and, where available, the sample rate, channels, and framing.

- `Sndhdr.what(file: string): SndInfo` — detect audio format from a path; returns a `SndInfo` with `format`, `rate`, `channels`, `frames`
- `Sndhdr.what(file: string, h: Variant): SndInfo` — sniff from already-read header bytes

**Recognized formats:**

| Format name | Magic signature |
|------------|-----------------|
| `wav` | RIFF…WAVE |
| `au` | `.snd` (Sun/NeXT AU) |
| `aiff` | FORM…AIFF |
| `aifc` | FORM…AIFC |
| `flac` | `fLaC` |
| `ogg` | `OggS` |

```titrate
import tt.audio.Sndhdr;

let info: SndInfo = Sndhdr.what("track.au");
io::println(info.format);          // "au"
io::println(Integer.toString(info.rate));       // e.g. 8000
io::println(Integer.toString(info.channels));   // e.g. 1
```
