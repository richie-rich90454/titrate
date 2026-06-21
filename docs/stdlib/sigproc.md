# sigproc

The `tt.sigproc` module provides signal processing utilities including FFT, filtering, windowing, wavelets, convolution, and spectral analysis.

```titrate
import tt.sigproc.FFT2;
import tt.sigproc.Filter;
import tt.sigproc.Window;
import tt.sigproc.Wavelet;
import tt.sigproc.Convolution;
import tt.sigproc.Spectrogram;
```

## FFT2

Fast Fourier Transform for 1D and 2D signals. Supports forward/inverse FFT, real-valued FFT, FFT shift, frequency bin computation, and windowed FFT.

- `fn init()` — create an FFT2 instance
- `fft(data: ArrayList<double>): ArrayList<Complex>` — compute 1D forward FFT
- `ifft(data: ArrayList<Complex>): ArrayList<double>` — compute 1D inverse FFT
- `fft2(data: NDArray<double>): NDArray<Complex>` — compute 2D forward FFT
- `ifft2(data: NDArray<Complex>): NDArray<double>` — compute 2D inverse FFT
- `rfft(data: ArrayList<double>): ArrayList<Complex>` — real-valued FFT (exploits real input symmetry)
- `irfft(data: ArrayList<Complex>): ArrayList<double>` — inverse real-valued FFT
- `fftShift(data: ArrayList<Complex>): ArrayList<Complex>` — shift zero-frequency component to center
- `ifftShift(data: ArrayList<Complex>): ArrayList<Complex>` — inverse FFT shift
- `fftFreq(n: int, sampleRate: double): ArrayList<double>` — compute FFT frequency bins
- `fftFreq2(rows: int, cols: int, sampleRateX: double, sampleRateY: double): ArrayList<ArrayList<double>>` — compute 2D FFT frequency bins
- `windowedFft(data: ArrayList<double>, window: ArrayList<double>): ArrayList<Complex>` — apply window then compute FFT

```titrate
let fft = new FFT2();

// 1D FFT on a simple signal
let signal = new ArrayList<double>();
for (let i = 0; i < 8; i++) {
    signal.add(MathTrig.sin(2.0 * Math.PI() * i / 8.0));
}
let spectrum = fft.fft(signal);

// Get frequency bins
let freqs = fft.fftFreq(8, 44100.0);

// Inverse FFT to recover the signal
let recovered = fft.ifft(spectrum);
```

## Filter

Digital filter design including Butterworth, Chebyshev, FIR, and IIR filters. Supports low-pass, high-pass, band-pass, and band-stop configurations.

- `fn init()` — create a Filter instance
- `butterworth(order: int, cutoffFreq: double, sampleRate: double, type: string): Filter` — design a Butterworth filter (`"lowpass"`, `"highpass"`, `"bandpass"`, `"bandstop"`)
- `butterworthBand(order: int, lowCutoff: double, highCutoff: double, sampleRate: double, type: string): Filter` — design a Butterworth band filter
- `chebyshev1(order: int, ripple: double, cutoffFreq: double, sampleRate: double, type: string): Filter` — design a Chebyshev Type I filter (ripple in passband)
- `chebyshev2(order: int, stopbandAtten: double, cutoffFreq: double, sampleRate: double, type: string): Filter` — design a Chebyshev Type II filter (ripple in stopband)
- `firWindow(numTaps: int, cutoffFreq: double, sampleRate: double, windowType: string): Filter` — design an FIR filter using the window method
- `firBandpass(numTaps: int, lowCutoff: double, highCutoff: double, sampleRate: double, windowType: string): Filter` — design an FIR band-pass filter
- `iir(order: int, cutoffFreq: double, sampleRate: double, type: string): Filter` — design a general IIR filter
- `apply(signal: ArrayList<double>): ArrayList<double>` — apply the filter to a signal
- `getNumerator(): ArrayList<double>` — get the filter numerator coefficients (B)
- `getDenominator(): ArrayList<double>` — get the filter denominator coefficients (A)

```titrate
let filt = Filter.butterworth(4, 1000.0, 44100.0, "lowpass");
let filtered = filt.apply(rawSignal);

let cheby = Filter.chebyshev1(6, 0.5, 2000.0, 44100.0, "highpass");
let highPassed = cheby.apply(rawSignal);

let firFilt = Filter.firWindow(65, 500.0, 44100.0, "hamming");
let firResult = firFilt.apply(rawSignal);
```

## Window

Window functions for spectral analysis and filter design. Reduces spectral leakage in FFT-based computations.

- `hamming(n: int): ArrayList<double>` — Hamming window
- `hanning(n: int): ArrayList<double>` — Hanning (Hann) window
- `blackman(n: int): ArrayList<double>` — Blackman window
- `kaiser(n: int, beta: double): ArrayList<double>` — Kaiser window with adjustable sidelobe attenuation
- `bartlett(n: int): ArrayList<double>` — Bartlett (triangular) window
- `flatTop(n: int): ArrayList<double>` — Flat-Top window (accurate amplitude measurements)
- `gaussian(n: int, sigma: double): ArrayList<double>` — Gaussian window
- `dolphChebyshev(n: int, attenuation: double): ArrayList<double>` — Dolph-Chebyshev window with equiripple sidelobes

```titrate
let n = 1024;
let win = Window.hamming(n);

// Apply window before FFT
let windowed = new ArrayList<double>();
for (let i = 0; i < signal.size(); i++) {
    windowed.add(signal.get(i) * win.get(i));
}

// Kaiser window with custom beta for sidelobe control
let kaiserWin = Window.kaiser(512, 8.0);
```

## Wavelet

Wavelet transforms for multi-resolution signal analysis. Supports Haar, Daubechies, continuous and discrete wavelet transforms.

- `fn init(waveletType: string)` — create a Wavelet instance (`"haar"`, `"db4"`, `"db8"`)
- `cwt(signal: ArrayList<double>, scales: ArrayList<double>, sampleRate: double): NDArray<double>` — continuous wavelet transform
- `dwt(signal: ArrayList<double>): (ArrayList<double>, ArrayList<double>)` — single-level discrete wavelet transform, returns (approximation, detail) coefficients
- `idwt(approx: ArrayList<double>, detail: ArrayList<double>): ArrayList<double>` — single-level inverse DWT
- `multiLevelDwt(signal: ArrayList<double>, levels: int): ArrayList<ArrayList<double>>` — multi-level DWT (wavelet decomposition)
- `multiLevelIdwt(coefficients: ArrayList<ArrayList<double>>): ArrayList<double>` — multi-level inverse DWT (wavelet reconstruction)
- `getWaveletName(): string` — get the name of the current wavelet
- `getDecompositionLength(): int` — get the decomposition filter length

```titrate
let wv = new Wavelet("db4");

// Single-level DWT
let (approx, detail) = wv.dwt(signal);

// Multi-level decomposition
let coeffs = wv.multiLevelDwt(signal, 5);

// Reconstruction
let reconstructed = wv.multiLevelIdwt(coeffs);

// Continuous wavelet transform
let scales = new ArrayList<double>();
for (let i = 1; i <= 128; i++) {
    scales.add(Double.parseDouble(Integer.toString(i)));
}
let cwtResult = wv.cwt(signal, scales, 44100.0);
```

## Convolution

Convolution and correlation operations for signal processing. Supports 1D/2D convolution, FFT-based fast convolution, and overlap-add/save methods.

- `convolve1D(signal: ArrayList<double>, kernel: ArrayList<double>): ArrayList<double>` — 1D convolution
- `convolve2D(image: NDArray<double>, kernel: NDArray<double>): NDArray<double>` — 2D convolution
- `correlate1D(signal: ArrayList<double>, kernel: ArrayList<double>): ArrayList<double>` — 1D cross-correlation
- `correlate2D(image: NDArray<double>, kernel: NDArray<double>): NDArray<double>` — 2D cross-correlation
- `deconvolve(signal: ArrayList<double>, kernel: ArrayList<double>): ArrayList<double>` — 1D deconvolution
- `fftConvolve(signal: ArrayList<double>, kernel: ArrayList<double>): ArrayList<double>` — FFT-based fast 1D convolution
- `fftConvolve2D(image: NDArray<double>, kernel: NDArray<double>): NDArray<double>` — FFT-based fast 2D convolution
- `overlapAdd(signal: ArrayList<double>, kernel: ArrayList<double>, blockSize: int): ArrayList<double>` — overlap-add convolution
- `overlapSave(signal: ArrayList<double>, kernel: ArrayList<double>, blockSize: int): ArrayList<double>` — overlap-save convolution

```titrate
let kernel = new ArrayList<double>();
kernel.add(0.25); kernel.add(0.5); kernel.add(0.25);

// Direct convolution
let result = Convolution.convolve1D(signal, kernel);

// FFT-based fast convolution (preferred for long signals)
let fastResult = Convolution.fftConvolve(signal, kernel);

// 2D convolution for image processing
let imgKernel = NDArray.fromData(shape, kernelData);
let blurred = Convolution.convolve2D(image, imgKernel);
```

## Spectrogram

Spectral analysis tools including STFT spectrogram, power spectral density, mel spectrogram, chromagram, and constant-Q transform.

- `fn init()` — create a Spectrogram instance
- `stft(signal: ArrayList<double>, windowSize: int, hopSize: int, windowType: string): NDArray<Complex>` — short-time Fourier transform
- `spectrogram(signal: ArrayList<double>, windowSize: int, hopSize: int, windowType: string): NDArray<double>` — magnitude spectrogram (|STFT|²)
- `psd(signal: ArrayList<double>, windowSize: int, hopSize: int, sampleRate: double): ArrayList<double>` — power spectral density (Welch's method)
- `melSpectrogram(signal: ArrayList<double>, sampleRate: double, numMelBins: int, windowSize: int, hopSize: int): NDArray<double>` — mel-scale spectrogram
- `chromagram(signal: ArrayList<double>, sampleRate: double, windowSize: int, hopSize: int): NDArray<double>` — chroma feature representation (pitch class energy)
- `constantQ(signal: ArrayList<double>, sampleRate: double, fMin: double, numBins: int, binsPerOctave: int): NDArray<double>` — constant-Q transform

```titrate
let sg = new Spectrogram();

// Compute spectrogram
let spec = sg.spectrogram(audioSignal, 1024, 512, "hanning");

// Power spectral density
let power = sg.psd(audioSignal, 2048, 1024, 44100.0);

// Mel spectrogram for machine learning
let melSpec = sg.melSpectrogram(audioSignal, 44100.0, 128, 1024, 512);

// Chromagram for music analysis
let chroma = sg.chromagram(musicSignal, 44100.0, 4096, 2048);

// Constant-Q transform
let cqt = sg.constantQ(audioSignal, 44100.0, 32.7, 84, 12);
```
