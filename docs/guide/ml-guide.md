# Machine Learning with Titrate

Titrate's `tt.ml` module provides tensor and `NDArray` primitives, scalar
automatic differentiation, linear layers, loss and metric functions,
SGD-family optimizers, data loaders, regularization helpers, and two
lightweight training helpers (`Sequential` and `Estimator`).

The APIs are deliberately small and explicit. Almost every function operates
on `ArrayList<double>` vectors or `ArrayList<ArrayList<double>>` matrices
(an `NDArray<double>` variant exists for batched data and tensors). Gradients
are computed by calling a `backward()` method or a gradient function directly;
there is no general-purpose tensor autograd engine. Every example below uses
names that exist in `lib/tt/ml/` and `lib/tt/math/ndarray/`.

## Tensors (`tt::ml::Tensor`)

The `Tensor.tr` module defines two tensor types. The older `Tensor` stores a
flat `ArrayList<double>` plus a shape; `NDTensor` is a thin wrapper around the
standard `NDArray<double>` used by the additive ML modules.

### The legacy `Tensor`

```titrate
import tt::ml::Tensor;
import tt::util::ArrayList;

// Construct from a shape; values start at 0.0
let shape: ArrayList<int> = new ArrayList<int>();
shape.add(2);
shape.add(3);
let t: Tensor = new Tensor(shape);          // 2 x 3, six zero elements

// Constructors from data
let v: ArrayList<double> = new ArrayList<double>();
v.add(1.0); v.add(2.0); v.add(3.0);
let t1: Tensor = Tensor.tensorFrom1D(v);    // shape [3]

let rows: ArrayList<ArrayList<double>> = new ArrayList<ArrayList<double>>();
// ... add two ArrayList<double> rows ...
let t2: Tensor = Tensor.tensorFrom2D(rows); // shape [2, 2]
```

`Tensor` exposes the public fields `data`, `shape`, `grad`, `requiresGrad`,
`operation`, and `parents`, and the instance methods `size()`,
`get(indices)`, `set(indices, val)`, `getFlat(i)`, `setFlat(i, val)`,
`backward()`, `zeroGrad()`, and `accumulateGrad(gradOutput)`.

Operations are top-level functions, not operators or methods:

```titrate
let sum: Tensor = Tensor.tensorAdd(a, b);        // element-wise
let diff: Tensor = Tensor.tensorSub(a, b);
let scaled: Tensor = Tensor.tensorMulScalar(a, 2.0);
let elem: Tensor = Tensor.tensorElementMul(a, b); // element-wise product
let mm: Tensor = Tensor.tensorMatmul(m1, m2);    // 2D matrix product
let tt: Tensor = Tensor.tensorTranspose(m1);     // 2D transpose
let total: double = Tensor.tensorSum(a);         // scalar
let avg: double = Tensor.tensorMean(a);          // scalar
```

`Tensor.backward()` is a simple one-step accumulation: it sets each zero slot
of this tensor's `grad` to `1.0`, then adds this tensor's values into the
`grad` of any parent in `parents` that has `requiresGrad = true`. The arithmetic
helpers above do not build a gradient graph. For gradient-based training, use
the explicit `backward()` methods on `Linear` and `Sequential`, or the scalar
autograd described in the next section.

### `NDTensor`

`NDTensor` is backed by `NDArray<double>` (rows, cols, ndim, shape) and is
created through the `ndTensor*` factories:

```titrate
import tt::ml::Tensor;

let shape: ArrayList<int> = new ArrayList<int>();
shape.add(2);
shape.add(2);
let t: NDTensor = Tensor.ndTensor(shape);        // zeros

// From values
let t1: NDTensor = Tensor.ndTensorFromArray(v);      // 1D
let t2: NDTensor = Tensor.ndTensorFromMatrix(rows);  // 2D

// Instance methods: size(), rows(), cols(), ndim(), shape(),
// getFlat(i), setFlat(i, val), get(i, j), set(i, j, val),
// fill(val), clone()
t.set(0, 1, 3.0);
let x: double = t.get(0, 1);

// Top-level operations
let added: NDTensor = Tensor.ndTensorAdd(t1, t2);
let sub: NDTensor = Tensor.ndTensorSub(t1, t2);
let scaled: NDTensor = Tensor.ndTensorScale(t1, 2.0);
let dot: double = Tensor.ndTensorDot(t1, t2);
let mm: NDTensor = Tensor.ndTensorMatmul(t2, Tensor.ndTensorTranspose(t2));
```

The underlying `NDArray` also provides factories such as `NDArray.zeros(shape)`,
`NDArray.ones(shape)`, `NDArray.filled(shape, value)`, and `fromData(shape, data)`,
plus indexing methods (`get1D`/`set1D`, `get2D`/`set2D`, `get3D`/`set3D`) and
`reshape`, `transpose`, and `flatten` methods. Reduction functions such as
`sum` and `mean` are top-level functions in the `ndarray` modules. See
[Scientific Computing](./scientific-computing) for the full NDArray reference.

## Automatic Differentiation (`tt::ml::Autograd`)

Autograd is reverse-mode differentiation over scalar `Scalar` nodes. A `Scalar`
has public fields `value`, `grad`, `visited`, `parents`, and `localGrads`. Only
addition and multiplication are wired into the graph.

```titrate
import tt::ml::Autograd;
import tt::lang::Double;

// f(x) = x^2 + 2x + 1 at x = 3  ->  f'(3) = 8
let x: Scalar = Autograd.scalar(3.0);
let two: Scalar = Autograd.scalar(2.0);
let one: Scalar = Autograd.scalar(1.0);

let x2: Scalar = Autograd.scalarMul(x, x);
let twoX: Scalar = Autograd.scalarMul(x, two);
let f: Scalar = Autograd.scalarAdd(Autograd.scalarAdd(x2, twoX), one);

Autograd.scalarBackward(f);

io::println("f(3) = " + Double.toString(f.value));   // 16.0
io::println("f'(3) = " + Double.toString(x.grad));   // 8.0
```

`scalarBackward(root)` sets `root.grad = 1.0` and accumulates gradients into
every ancestor. Gradients accumulate on shared nodes, as the `x^2` case shows:
both parents of the multiply are the same node, and its final gradient is `6`
from that branch plus `2` from `2x`.

## Linear Layers (`tt::ml::Linear`)

`Linear` is a fully connected layer with explicit forward and backward passes.
Weights and biases are deterministically zero-initialized.

```titrate
import tt::ml::Linear;
import tt::util::ArrayList;

let layer: Linear = new Linear(2, 1);
layer.setWeight(0, 0, 0.5);
layer.setWeight(0, 1, -0.25);
layer.setBias(0, 0.1);

let input: ArrayList<double> = new ArrayList<double>();
input.add(1.0); input.add(2.0);
let output: ArrayList<double> = layer.forward(input);   // [0.1]

let upstream: ArrayList<double> = new ArrayList<double>();
upstream.add(1.0);
let inputGrad: ArrayList<double> = layer.backward(upstream);

// Gradients are stored on the layer: weights get weightGrads, biases get biasGrads
layer.zeroGrad();
```

`backward(gradOutput)` accumulates `weightGrads` and `biasGrads` and returns
`dL/dInput`, so layers can be chained manually.

## Loss Functions (`tt::ml::Loss`)

All loss functions return a `double`; gradient helpers return
`ArrayList<double>`.

```titrate
import tt::ml::Loss;

// Vector losses: predicted and actual are ArrayList<double>
let mse: double = Loss.mseLoss(predicted, actual);
let mae: double = Loss.maeLoss(predicted, actual);
let ce: double = Loss.crossEntropyLoss(probs, oneHot);
let kl: double = Loss.klDivergence(p, q);
let huberVec: double = Loss.huberLossVec(predicted, actual, 1.0);

// Vector gradients
let mseGrad: ArrayList<double> = Loss.mseGradient(predicted, actual);
let ceGrad: ArrayList<double> = Loss.crossEntropyGradient(probs, oneHot);

// Scalar losses: one prediction / target pair
let bce: double = Loss.binaryCrossEntropyLoss(0.9, 1.0);
let hinge: double = Loss.hingeLoss(-1.0, 1.0);
let huber: double = Loss.huberLoss(5.0, 0.0, 1.0);

// Object form with a uniform compute/gradient interface
let mseObj: MSELoss = new MSELoss();
let value: double = mseObj.compute(predicted, actual);
let grad: ArrayList<double> = mseObj.gradient(predicted, actual);

let ceObj: CrossEntropyLoss = new CrossEntropyLoss();
let ceValue: double = ceObj.compute(probs, oneHot);
let ceGradient: ArrayList<double> = ceObj.gradient(probs, oneHot);
```

`crossEntropyLoss` expects probability vectors (e.g. one-hot targets), not raw
logits plus a class index. `crossEntropyGradient` returns `(p - actual) / n`.

## Optimizers (`tt::ml::Optim` and `tt::ml::Optimizer`)

Optimizers update matrix weights and vector biases. They take weights and
gradients as arguments; they do not own a model.

### Simple SGD (`tt::ml::Optim`)

```titrate
import tt::ml::Optim;

let opt: SGD = new SGD(0.01);
opt.step(layer.weights, layer.weightGrads);   // matrix parameters
opt.stepVec(layer.biases, layer.biasGrads);   // vector parameters

// Equivalent free functions
Optim.sgdStepMat(layer.weights, layer.weightGrads, 0.01);
Optim.sgdStepVec(layer.biases, layer.biasGrads, 0.01);
```

### Advanced optimizers (`tt::ml::Optimizer`)

```titrate
import tt::ml::Optimizer;

let sgd: SGDOptimizer = new SGDOptimizer(0.1, 0.9, false); // lr, momentum, nesterov
let adam: AdamOptimizer = new AdamOptimizer(0.001);
let adamw: AdamW = new AdamW(0.001, 0.01);                  // lr, weight decay
let rmsprop: RMSProp = new RMSProp(0.001);
let adagrad: AdaGrad = new AdaGrad(0.01);

adam.step(weights, grads);
```

All of these expose `step(weights, grads)`. Clearing gradients is done on the
layer or model (`layer.zeroGrad()` / `model.zeroGrad()`), not on the optimizer.

### Learning-rate schedules

Schedulers are functions that return the learning rate for a given epoch; they
are not objects with a `step()` method:

```titrate
let lr1: double = Optimizer.stepLR(0.1, 30, 0.1, epoch);   // decay by gamma every stepSize epochs
let lr2: double = Optimizer.cosineLR(0.1, 0.001, 100, epoch);
let lr3: double = Optimizer.warmupLR(0.1, 10, epoch);
```

## Activation Functions (`tt::ml::Activation` and `tt::ml::Layer`)

`tt::ml::Activation` provides scalar functions and their derivatives, plus a
vector softmax:

```titrate
import tt::ml::Activation;

let a: double = Activation.relu(-2.0);        // 0.0
let b: double = Activation.sigmoid(0.0);      // 0.5
let c: double = Activation.tanh(0.0);         // 0.0
let da: double = Activation.reluDerivative(2.0);
let db: double = Activation.sigmoidDerivative(0.0);
let dc: double = Activation.tanhDerivative(0.0);

// softmax takes and returns ArrayList<double>
let probs: ArrayList<double> = Activation.softmax(scores);
let jvp: ArrayList<double> = Activation.softmaxDerivative(scores, upstream);
```

`tt::ml::Layer` also exposes `relu`, `sigmoid`, `tanh`, `gelu`,
`reluDerivative`, `sigmoidDerivative`, and `softmax` with the same signatures.

## Building Models

### `Sequential` (`tt::ml::Sequential`)

`Sequential` stacks `Linear` layers with a named activation per layer. Valid
activation strings are `"relu"`, `"sigmoid"`, `"tanh"`, `"softmax"`, and
`"linear"` (no activation).

```titrate
import tt::ml::Sequential;
import tt::ml::Linear;
import tt::ml::Optim;

let model: Sequential = new Sequential();
model.add(new Linear(2, 4), "relu");
model.add(new Linear(4, 2), "softmax");

let out: ArrayList<double> = model.forward(input);
let pred: ArrayList<double> = model.predict(input);  // same as forward here
let grad: ArrayList<double> = model.backward(upstream);
model.zeroGrad();

// One full training step: forward, loss, backward, optimizer update
let optimizer: SGD = new SGD(0.1);
let loss: double = model.trainStep(input, target, optimizer, "cross_entropy");
```

`trainStep` accepts `"cross_entropy"` or any other string for MSE. Its
`optimizer` parameter is the `SGD` type from `tt::ml::Optim`.

### `Estimator` (`tt::ml::Estimator`)

`Estimator` wraps a `Linear` layer plus SGD in a scikit-learn-style
`fit`/`predict` interface. Training is deterministic (no shuffling).

```titrate
import tt::ml::Estimator;

let est: Estimator = new Estimator(1, 1, 0.1, "mse");
let finalLoss: double = est.fit(inputs, targets, 200);
let prediction: ArrayList<double> = est.predict(sample);
```

The loss string is `"mse"` (default path) or `"cross_entropy"`.

### `SequentialModel` (`tt::ml::Model`)

`SequentialModel` stacks `DenseLayer` objects and optionally applies dropout:

```titrate
import tt::ml::Model;
import tt::ml::Layer;
import tt::ml::Optimizer;

let model: SequentialModel = new SequentialModel();
model.add(new DenseLayer(2, 4, "relu"));
model.add(new DenseLayer(4, 2, "linear"));
model.addDropout(0.2);

let output: ArrayList<double> = model.forward(input);
let prediction: ArrayList<double> = model.predict(input); // dropout disabled for prediction

let sgdOpt: SGDOptimizer = new SGDOptimizer(0.1, 0.0, false);
let loss1: double = model.train(input, target, sgdOpt);

let adamOpt: AdamOptimizer = new AdamOptimizer(0.001);
let loss2: double = model.trainAdam(input, target, adamOpt);

// Accuracy on ArrayList<Tensor> data and labels (compares argmax indices)
let accuracy: double = model.evaluate(dataTensors, labelTensors);
```

`DenseLayer(input, output, act)` uses Xavier initialization and applies `"relu"`,
`"sigmoid"`, or `"tanh"` inside `forward`.

### Other layers (`tt::ml::Layer`)

```titrate
import tt::ml::Layer;

let dropout: DropoutLayer = Layer.Dropout(0.5);       // factory function
let dropped: ArrayList<double> = dropout.forward(input);

let bn: BatchNormLayer = new BatchNormLayer(4);
let normalized: ArrayList<double> = bn.forward(input);

// Conv2D(numFilters, kernelSize, stride, padding); single input channel,
// input is a 2D ArrayList<ArrayList<double>>, output is one map per filter
let conv: Conv2D = new Conv2D(16, 3, 1, 1);
let maps: ArrayList<ArrayList<ArrayList<double>>> = conv.forward(image);

// RNN and LSTM process one timestep per forward call and return the new hidden state
let rnn: RNN = new RNN(64, 32);
let hidden: ArrayList<double> = rnn.forward(stepInput);
let lstm: LSTM = new LSTM(64, 32);
let hiddenLstm: ArrayList<double> = lstm.forward(stepInput);
```

There are no max-pooling, flattening, or tensor-level convenience helpers in
`tt.ml`; reshape and similar operations belong to `NDArray` (see section 1).

## Data Loading (`tt::ml::DataLoader`)

### ArrayList-backed dataset and loader

```titrate
import tt::ml::DataLoader;

let ds: Dataset = new Dataset();
ds.add(features, labels);                       // both ArrayList<double>

let loader: DataLoader = new DataLoader(ds, 32, true);  // dataset, batch size, shuffle
let nBatches: int = loader.numBatches();
```

`getBatch(i)` returns a two-element list: index `0` is the batch's feature
rows, index `1` the label rows:

```titrate
let batch: ArrayList<ArrayList<ArrayList<double>>> = loader.getBatch(0);
let features: ArrayList<ArrayList<double>> = batch.get(0);
let labels: ArrayList<ArrayList<double>> = batch.get(1);

loader.shuffle();  // reshuffle indices
loader.reset();    // restore index order (and reshuffle if enabled)

// Train / validation / test split, returned in that order
let splits: ArrayList<Dataset> = DataLoader.trainValTestSplit(ds, 0.6, 0.2);
let train: Dataset = splits.get(0);
let val: Dataset = splits.get(1);
let test: Dataset = splits.get(2);
```

There is no iterator protocol on `DataLoader`: use `numBatches()` and
`getBatch(i)` in an indexed loop.

### NDArray-backed loader

For `NDArray<double>` data, use `NDArrayDataset` and `NDArrayDataLoader`. Rows
are samples; features and labels are `NDArray<double>` values.

```titrate
import tt::ml::DataLoader;
import tt::math::ndarray::NDArray;

let ds2: NDArrayDataset = new NDArrayDataset(featuresND, labelsND);
let loader2: NDArrayDataLoader = new NDArrayDataLoader(ds2, 64, false);
let b: ArrayList<NDArray<double>> = loader2.getBatch(0); // b[0] features, b[1] labels
loader2.shuffleOrder();
```

## Metrics (`tt::ml::Metrics`)

```titrate
import tt::ml::Metrics;

let bestClass: int = Metrics.argmaxList(scores);          // index of largest value
let acc: double = Metrics.accuracy(predictedLabels, actualLabels); // ArrayList<int>
let accScores: double = Metrics.accuracyFromScores(scoreRows, actualLabels);
let p: double = Metrics.precision(predictedLabels, actualLabels, 1);
let r: double = Metrics.recall(predictedLabels, actualLabels, 1);
let f1: double = Metrics.f1(predictedLabels, actualLabels, 1);
let cm: ArrayList<ArrayList<int>> = Metrics.confusionMatrix(predictedLabels, actualLabels, 2);
```

`confusionMatrix` is indexed `[actualClass][predictedClass]`. Use
`accuracyFromScores` when you have per-sample score vectors; it calls
`argmaxList` on each row for you.

## Regularization (`tt::ml::Regularization`)

Penalties and matching gradients are available for matrices (the shape used
by `Linear.weights`) and vectors (the shape used by `Linear.biases`):

```titrate
import tt::ml::Regularization;

let l1: double = Regularization.l1Penalty(layer.weights, 0.01);
let l2: double = Regularization.l2Penalty(layer.weights, 0.01);
let l1g: ArrayList<ArrayList<double>> = Regularization.l1Gradient(layer.weights, 0.01);
let l2g: ArrayList<ArrayList<double>> = Regularization.l2Gradient(layer.weights, 0.01);

// Vector variants: l1PenaltyVec, l1GradientVec, l2PenaltyVec, l2GradientVec
let biasPenalty: double = Regularization.l2PenaltyVec(layer.biases, 0.01);
```

Gradients come back in the same shape as the supplied parameters, so they can
be added into `weightGrads` / `biasGrads` before an optimizer step.

## End-to-End Example: Learning a Small Classifier

This example trains a two-layer `Sequential` model on a tiny dataset using
`Dataset`, `DataLoader`, `SGD`, and the cross-entropy loss helper.

```titrate
import tt::util::ArrayList;
import tt::lang::Double;
import tt::ml::Linear;
import tt::ml::Sequential;
import tt::ml::Optim;
import tt::ml::DataLoader;
import tt::ml::Metrics;

fn pair(a: double, b: double): ArrayList<double> {
    let v: ArrayList<double> = new ArrayList<double>();
    v.add(a);
    v.add(b);
    return v;
}

fn oneHot(i: int): ArrayList<double> {
    let v: ArrayList<double> = new ArrayList<double>();
    v.add(i == 0 ? 1.0 : 0.0);
    v.add(i == 1 ? 1.0 : 0.0);
    return v;
}

public fn main(): void {
    let ds: Dataset = new Dataset();
    ds.add(pair(0.0, 0.0), oneHot(0));
    ds.add(pair(0.0, 1.0), oneHot(1));
    ds.add(pair(1.0, 0.0), oneHot(1));
    ds.add(pair(1.0, 1.0), oneHot(0));

    let loader: DataLoader = new DataLoader(ds, 2, false);
    let model: Sequential = new Sequential();
    model.add(new Linear(2, 4), "relu");
    model.add(new Linear(4, 2), "softmax");
    let optimizer: SGD = new SGD(0.1);

    var epoch: int = 0;
    while (epoch < 200) {
        var b: int = 0;
        while (b < loader.numBatches()) {
            let batch: ArrayList<ArrayList<ArrayList<double>>> = loader.getBatch(b);
            let features: ArrayList<ArrayList<double>> = batch.get(0);
            let targets: ArrayList<ArrayList<double>> = batch.get(1);
            var s: int = 0;
            while (s < features.size()) {
                model.trainStep(features.get(s), targets.get(s), optimizer, "cross_entropy");
                s = s + 1;
            }
            b = b + 1;
        }
        epoch = epoch + 1;
    }

    let scoreRows: ArrayList<ArrayList<double>> = new ArrayList<ArrayList<double>>();
    let actual: ArrayList<int> = new ArrayList<int>();
    var i: int = 0;
    while (i < ds.size()) {
        let sample: ArrayList<ArrayList<double>> = ds.get(i);
        scoreRows.add(model.predict(sample.get(0)));
        actual.add(Metrics.argmaxList(sample.get(1)));
        i = i + 1;
    }
    io::println("Accuracy: " + Double.toString(Metrics.accuracyFromScores(scoreRows, actual)));
}
```

::: tip Debugging training
If your model is not learning, check these common issues:
1. **Learning rate too high** — loss oscillates or diverges. Try reducing by 10×.
2. **Learning rate too low** — loss barely decreases. Try increasing by 3×.
3. **Vanishing gradients** — prefer ReLU over sigmoid/tanh in hidden layers.
4. **Overfitting** — add `DropoutLayer`, apply L1/L2 penalties, or add data.
:::

## Not Implemented (Do Not Use)

These names appeared in older material but do not exist in `lib/tt/ml/` or
`lib/tt/math/ndarray/`. Calling them will fail to compile or run:

- `Tensor.fromArray`, `Tensor.zeros`, `Tensor.ones`, `Tensor.rand`,
  `Tensor.randn`, `Tensor.scalar` — use the `tensor*`/`ndTensor*` factories or
  `new Tensor(shape)`.
- Tensor methods `reshape`, `transpose`, `sum`, `mean`, `max`, `matmul`, `mul`,
  `argmax`, `eq`, `item`, `grad()`, `setRequiresGrad`, `shape()` — gradients
  are a public `grad` field and `requiresGrad` is set directly; operations are
  top-level functions. Reshape/transpose live on `NDArray`.
- Operator syntax on tensors (`a + b`, `a * 2.0`) — use `Tensor.tensorAdd`,
  `Tensor.tensorMulScalar`, etc.
- `Layer.Dense` and `Layer.Conv2D(in, out, kernel)` — the classes are
  `DenseLayer(input, output, act)` and `Conv2D(numFilters, kernelSize, stride,
  padding)` (single input channel).
- `maxPool2D()` / `flatten()` on tensors — use `NDArray.reshape`/`flatten`.
- `LSTM.forward` returning `(output, hidden)` — it returns one hidden-state
  vector for one timestep.
- `Model.fit` — use `Sequential.trainStep`, `Estimator.fit`, or
  `SequentialModel.train` / `trainAdam`.
- `Optimizer.SGD(...)`, `Optimizer.Adam(...)`, `Optimizer.AdamW(...)` factories,
  `model.parameters()`, `optimizer.zeroGrad()`, and scheduler objects with a
  `step()` method — instantiate `SGDOptimizer`/`AdamOptimizer`/`AdamW` directly,
  clear gradients on the layer/model, and call the `stepLR`/`cosineLR`/`warmupLR`
  functions.
- `DataLoader.split`, `DataLoader.loadMNIST`, `for (batch in loader)`, and
  `batch.inputs` / `batch.labels` — use `trainValTestSplit`, indexed
  `getBatch(i)`, and `batch.get(0)` / `batch.get(1)`. The `DataLoader`
  constructor takes `(Dataset, int, bool)`, not raw feature/label arrays.

## What's Next?

- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [Standard Library](./stdlib) — full module reference
- [Error Handling](./error-handling) — robust error handling with `Result`
