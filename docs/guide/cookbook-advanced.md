# Advanced Cookbook

The basic [Cookbook](cookbook.md) covers files, HTTP, caching, and other everyday tasks. This cookbook goes after the newer standard-library domains: numerical arrays, machine learning, graphics, audio, and networking. Each recipe is a complete program built from APIs that exist in `lib/tt`, followed by a short explanation of the design choices.

All examples use the canonical import form `tt::module::Name`. Dots work too (`tt.module.Name`) if you prefer them.

## Recipe 1: Numerical work with NDArray

**Problem:** Build two matrices, multiply them, and report shape-aware statistics without writing the loops yourself.

```titrate
import tt::math::ndarray::NDArray;
import tt::math::ndarray::NDArrayGen;
import tt::math::ndarray::NDArrayReduce;
import tt::math::ndarray::NDArrayLinalg;
import tt::util::ArrayList;
import tt::lang::Double;
import tt::lang::Integer;

fn shape(r: int, c: int): ArrayList<int> {
    let s: ArrayList<int> = new ArrayList<int>();
    s.add(r);
    s.add(c);
    return s;
}

public fn ndarrayDemo(): void {
    // Build a 2x3 and a 3x2 matrix from flat row-major data.
    let leftData: ArrayList<double> = new ArrayList<double>();
    leftData.add(1.0); leftData.add(2.0); leftData.add(3.0);
    leftData.add(4.0); leftData.add(5.0); leftData.add(6.0);

    let rightData: ArrayList<double> = new ArrayList<double>();
    rightData.add(7.0);  rightData.add(8.0);
    rightData.add(9.0);  rightData.add(10.0);
    rightData.add(11.0); rightData.add(12.0);

    let left: NDArray<double> = fromData(shape(2, 3), leftData);
    let right: NDArray<double> = fromData(shape(3, 2), rightData);

    let product: NDArray<double> = matmul(left, right);

    io::println("left shape: " + Integer.toString(left.rows()) +
                "x" + Integer.toString(left.cols()));
    io::println("product shape: " + Integer.toString(product.rows()) +
                "x" + Integer.toString(product.cols()));
    io::println("sum(left) = " + Double.toString(sum(left)));
    io::println("mean(left) = " + Double.toString(mean(left)));
    io::println("max(left) = " + Double.toString(max(left)));
    io::println("stddev(left) = " + Double.toString(stddev(left)));
    io::println("trace(product) = " + Double.toString(trace(product)));

    // A generated vector: 5 values evenly spaced from 0.0 to 1.0.
    let grid: NDArray<double> = linspace(0.0, 1.0, 5);
    io::println("grid[4] = " + Double.toString(grid.get1D(4)));
}

public fn main(): void {
    ndarrayDemo();
}
```

**How it works:** `fromData<T>(shape, data)` attaches a shape to a flat buffer; `get2D`/`set2D` use the computed strides for indexed access. `matmul` performs the 2x3 by 3x2 contraction and returns a new `NDArray<double>`. Reductions (`sum`, `mean`, `max`, `stddev`) come from `NDArrayReduce`, and `trace` from `NDArrayLinalg`.

**Useful neighbors:** `zeros(shape)`, `ones(shape)`, `filled(shape, value)`, `arange(start, stop, step)`, `logspace(start, stop, num)`, `reshape`, `transpose`, `flatten`, `slice(a, starts, ends)`, `concat`, `stack`, `sort`, `argsort`, and the element-wise helpers `ndAdd`, `ndSubtract`, `ndMultiply`, `ndDivide`, `ndSqrt`, `ndExp`, and `ndLog`. `reshape` and `flatten` share the original data buffer, so they are views rather than copies.

::: tip Choose the right reduction module
`NDArrayReduce` holds the generic reductions and statistics over `NDArray<T>`. `NDArrayStats` has a similar surface, and `NDArrayMath` holds the element-wise `nd*` functions. Import the one whose functions you call so names stay unambiguous.
:::

## Recipe 2: Train and inspect a small model

**Problem:** Assemble a two-layer network, train it for one step, and inspect a prediction. Also show the lower-level tensor and autograd APIs for cases where you need manual control.

```titrate
import tt::util::ArrayList;
import tt::ml::Model;
import tt::ml::Layer;
import tt::ml::Optimizer;
import tt::ml::Tensor;
import tt::lang::Double;
import tt::lang::Integer;

fn vector(a: double, b: double): ArrayList<double> {
    let v: ArrayList<double> = new ArrayList<double>();
    v.add(a);
    v.add(b);
    return v;
}

fn single(a: double): ArrayList<double> {
    let v: ArrayList<double> = new ArrayList<double>();
    v.add(a);
    return v;
}

public fn trainSmallModel(): void {
    let model: SequentialModel = new SequentialModel();
    model.add(new DenseLayer(2, 4, "relu"));
    model.add(new DenseLayer(4, 1, "linear"));

    let input: ArrayList<double> = vector(0.5, 0.7);
    let target: ArrayList<double> = single(1.0);
    let optimizer: SGDOptimizer = new SGDOptimizer(0.01, 0.9, false);

    let before: ArrayList<double> = model.predict(input);
    let loss: double = model.train(input, target, optimizer);
    let after: ArrayList<double> = model.predict(input);

    io::println("loss = " + Double.toString(loss));
    io::println("before = " + Double.toString(before.get(0)));
    io::println("after  = " + Double.toString(after.get(0)));
}

public fn tensorDemo(): void {
    let aData: ArrayList<double> = new ArrayList<double>();
    aData.add(1.0); aData.add(2.0); aData.add(3.0); aData.add(4.0);
    let a: Tensor = tensorFrom1D(aData);

    let shape: ArrayList<int> = new ArrayList<int>();
    shape.add(2); shape.add(2);
    let m: Tensor = new Tensor(shape);
    var i: int = 0;
    while (i < 4) {
        m.setFlat(i, a.getFlat(i));
        i = i + 1;
    }
    let transposed: Tensor = tensorTranspose(m);
    let flattenedSum: double = tensorSum(transposed);
    io::println("sum of transposed = " + Double.toString(flattenedSum));
}

public fn main(): void {
    trainSmallModel();
    tensorDemo();
}
```

**How it works:** `SequentialModel.add` accepts a `DenseLayer`; the constructor takes `(inputSize, outputSize, act)` where `act` is `"relu"`, `"sigmoid"`, or `"tanh"`. `train` returns the scalar loss after one gradient step using the supplied optimizer. `predict` runs a forward pass without updating weights.

**Useful neighbors:** `addDropout(rate)`, `trainAdam(input, target, optimizer)` with `AdamOptimizer(lr)`, `evaluate(data, labels)`, the standalone losses in `tt::ml::Loss` (`mseLoss`, `crossEntropyLoss`, `huberLoss`, `klDivergence`), other optimizers in `tt::ml::Optimizer` (`AdamW`, `RMSProp`, `AdaGrad`) plus `stepLR`, `cosineLR`, and `warmupLR`, and batching through `tt::ml::DataLoader`:

```titrate
import tt::ml::DataLoader;
import tt::util::ArrayList;
import tt::lang::Integer;

public fn batchDemo(): void {
    let dataset: Dataset = new Dataset();
    let features: ArrayList<double> = new ArrayList<double>();
    features.add(1.0);
    let label: ArrayList<double> = new ArrayList<double>();
    label.add(0.0);

    dataset.add(features, label);
    let loader: DataLoader = new DataLoader(dataset, 8, true);
    io::println("batches = " + Integer.toString(loader.numBatches()));
    let firstBatch = loader.getBatch(0);
    loader.shuffle();
}
```

For automatic differentiation, `tt::ml::Autograd` provides scalar nodes:

```titrate
import tt::ml::Autograd;
import tt::lang::Double;

public fn autogradDemo(): void {
    let x = scalar(3.0);
    let w = scalar(2.0);
    let y = scalarMul(x, w);          // y = x * w
    scalarBackward(y);
    io::println("dy/dx = " + Double.toString(x.grad)); // 2.0
    io::println("dy/dw = " + Double.toString(w.grad)); // 3.0
}
```

## Recipe 3: Draw a pixel buffer and open a window

**Problem:** Create an in-memory canvas, paint a gradient, read a pixel back, then optionally display it in a window.

```titrate
import tt::graphics::Canvas;
import tt::graphics::Color;
import tt::graphics::Window;
import tt::lang::Result;
import tt::lang::Integer;

public fn paintGradient(): void {
    let canvas: Canvas = new Canvas(64, 64);
    if (!canvas.isValid()) {
        io::println("canvas allocation failed");
        return;
    }

    canvas.clear(Color.rgb(16, 18, 24));

    var x: int = 0;
    while (x < canvas.getWidth()) {
        var y: int = 0;
        while (y < canvas.getHeight()) {
            let c: int = Color.rgba(x * 4, y * 4, 128, 255);
            canvas.setPixel(x, y, c);
            y = y + 1;
        }
        x = x + 1;
    }

    let center: int = canvas.getPixel(32, 32);
    io::println("center pixel = " + Integer.toString(center));
    io::println("red channel = " + Integer.toString(Color.red(center)));
    canvas.destroy();
}

public fn openWindow(): void {
    if (!windowingAvailable()) {
        io::println("windowing is not available on this build");
        return;
    }

    let win: Window = new Window("Titrate", 640, 480);
    let created: Result<bool, string> = win.create();
    if (created.isErr()) {
        io::println("create failed: " + created.unwrapErr());
        return;
    }

    while (win.isOpen()) {
        let code: int = win.pollInput();
        if (code == 27) {             // Escape closes the window
            win.close();
        }
    }
}

public fn main(): void {
    paintGradient();
    openWindow();
}
```

**How it works:** `Canvas` is a pure pixel buffer: `setPixel`, `getPixel`, `clear`, and `destroy` are all you need for off-screen work, and no display server is required. `Color.rgb`/`rgba` pack channels into an `int`; `red`, `green`, `blue`, and `alpha` unpack them. `Window` is the optional display layer: `create()` returns a `Result` because a window may fail to open, and input is polled with `pollInput()` after pushing synthetic codes with `pushInput(code)` in tests.

::: warning Headless environments
`windowingAvailable()` is the cheap way to avoid calling `create()` on a build or machine without a display. Canvas work does not have this restriction.
:::

## Recipe 4: Synthesize, process, and save audio

**Problem:** Generate a sine tone, apply fades, estimate its pitch, write a WAV file, and play it if an output device exists.

```titrate
import tt::audio::AudioBuffer;
import tt::audio::Synth;
import tt::audio::AudioDevice;
import tt::audio::WavWriter;
import tt::audio::Pitch;
import tt::util::ArrayList;
import tt::lang::Double;
import tt::lang::Integer;

fn toneBuffer(): AudioBuffer {
    let synth: Synth = new Synth(44100, 0.2);
    let samples = synth.sine(440.0, 44100);

    let buffer: AudioBuffer = new AudioBuffer(44100, 1);
    var i: int = 0;
    while (i < samples.size()) {
        let frame: ArrayList<double> = new ArrayList<double>();
        frame.add(samples.get(i));
        buffer.addFrame(frame);
        i = i + 1;
    }
    return buffer;
}

public fn audioDemo(): void {
    let buffer: AudioBuffer = toneBuffer();
    buffer.fadeIn(0.01);
    buffer.fadeOut(0.05);

    io::println("frames = " + Integer.toString(buffer.numFrames()));
    io::println("duration = " + Double.toString(buffer.duration()));
    io::println("pitch = " + Double.toString(detectAutocorrelation(buffer)));

    writeWav("tone.wav", buffer, 16);
    io::println("wrote tone.wav");

    if (outputAvailable()) {
        let device: AudioDevice = new AudioDevice(44100.0, 1);
        let frames: ArrayList<double> = new ArrayList<double>();
        var i: int = 0;
        while (i < buffer.numFrames()) {
            frames.add(buffer.getSample(0, i));
            i = i + 1;
        }
        let played = device.play(frames);
        if (played.isErr()) {
            io::println("playback failed: " + played.unwrapErr());
        }
    }
}

public fn main(): void {
    audioDemo();
}
```

**How it works:** `Synth` produces raw `ArrayList<double>` sample streams (`sine`, `square`, `saw`, `noise`, `playNote`, `sequence`). `AudioBuffer` is the multichannel container: `addFrame` appends one frame, and `getSample(channel, frame)` reads a specific channel. `normalize`, `fadeIn`, and `fadeOut` modify the buffer in place; `mix`, `splice`, and `reversed` return a new `AudioBuffer`. `writeWav(path, buffer, bitsPerSample)` writes a standard 16-bit file; `writeF32(path, buffer)` writes 32-bit float.

**Useful neighbors:** `tt::audio::WavReader.readWav`, `tt::audio::Wave.readAudioBuffer`, `tt::audio::Aifc`, `tt::audio::Sunau`, `tt::audio::Mfcc.computeMFCC`, `tt::audio::Pitch.detectPitchYIN`, and `tt::audio::ChunkReader` for RIFF-style chunk parsing.

## Recipe 5: HTTP requests and a raw TCP exchange

**Problem:** Fetch a JSON payload with `HttpClient`, then send a line over a raw TCP socket and read the reply.

```titrate
import tt::net::HttpClient;
import tt::net::TcpClient;
import tt::lang::Integer;

public fn fetchUrl(url: string): void {
    let client: HttpClient = new HttpClient();
    client.setTimeout(5000);
    client.setUserAgent("titrate-cookbook/1.0");
    client.setFollowRedirects(true);

    let response = client.get(url);
    io::println("status = " + Integer.toString(response.getStatusCode()));
    io::println("body = " + response.getBody());
}

public fn tcpExchange(host: string, port: int, message: string): void {
    let client: TcpClient = new TcpClient();
    if (!client.connect(host, port)) {
        io::println("connect failed");
        return;
    }

    client.setTimeout(3000);
    let sent: int = client.send(message);
    io::println("sent = " + Integer.toString(sent));

    let reply: string = client.receive(4096);
    io::println("reply = " + reply);
    client.close();
}

public fn main(): void {
    fetchUrl("https://example.com/");
    tcpExchange("localhost", 8080, "PING\n");
}
```

**How it works:** `HttpClient.get` returns an `HttpResponse` with `getStatusCode`, `getHeaders`, and `getBody`; `post`, `put`, `patch`, `delete`, and `head` cover the other verbs, and `setHeader` adds custom headers. `TcpClient` is a thin wrapper over VM built-ins: `connect`, `send`, `receive`, `setTimeout`, and `close`. `isConnected` reports whether the handle is still live.

**Useful neighbors:** `tt::net::Socket` for lower-level control (`bind`, `listen`, `accept`, `sendAll`, `recvAll`, `setKeepAlive`), `tt::net::TcpServer` for accepting connections, `tt::net::UdpSocket` for datagrams, `tt::net::Dns` for name resolution, `tt::net::Uri` / `UrlBuilder` for URL construction, and `tt::net::WebSocket` for upgraded connections.

::: warning Network calls are fallible and slow
Set a timeout before every remote operation and check the result. For `HttpClient`, the response object is always returned; decide what a non-2xx status means in your own code. For raw sockets, handle an empty `receive` string as a closed peer.
:::

## Verification notes

Every function and class used in this cookbook was read from the corresponding source file under `lib/tt`, and the native-backed calls were checked against `trc/src/bytecode/vm/natives/lookup.rs`.

Two things are deliberately **not** used here because the source does not provide them:

- The older `docs/guide/ml-guide.md` shows `Tensor.fromArray`, `Tensor.zeros`, `Tensor.rand`, `Tensor.randn`, `Tensor.scalar`, `setRequiresGrad`, `grad()`, `matmul()` as a method, and `Layer.Dense(...)`. The actual `lib/tt/ml/Tensor.tr` exposes `new Tensor(shape)`, `get`/`set`, `getFlat`/`setFlat`, `backward`, a public `grad` field, and top-level `tensorFrom1D`, `tensorFrom2D`, `tensorAdd`, `tensorMatmul`, `tensorTranspose`, `tensorSum`, and `tensorMean`; `Layer.tr` exposes `new DenseLayer(input, output, act)`. Use the source-backed forms shown in this cookbook.
- `Math.sqrt` and `Math.sin` do not exist on the `Math` module. Power, exponential, logarithm, and root functions live on `MathAdvanced`; trigonometric functions live on `MathTrig`.

No other API was referenced without being verified against source.
