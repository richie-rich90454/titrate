# ml

The `tt.ml` module provides machine learning primitives including tensors with autograd, neural network layers, loss functions, optimizers, model composition, and data loading.

```titrate
import tt.ml.Tensor;
import tt.ml.Layer;
import tt.ml.Loss;
import tt.ml.Optimizer;
import tt.ml.Model;
import tt.ml.DataLoader;
```

## Tensor

N-dimensional tensor with automatic differentiation (autograd) support. Tracks computation graphs for backpropagation and supports gradient accumulation.

- `fn init(shape: ArrayList<int>)` — create zero-filled tensor
- `fn init(shape: ArrayList<int>, data: ArrayList<double>)` — create tensor from flat data
- `Tensor.zeros(shape: ArrayList<int>): Tensor` — zero-filled tensor
- `Tensor.ones(shape: ArrayList<int>): Tensor` — one-filled tensor
- `Tensor.randn(shape: ArrayList<int>, mean: double, std: double): Tensor` — random normal tensor
- `Tensor.uniform(shape: ArrayList<int>, lo: double, hi: double): Tensor` — random uniform tensor
- `Tensor.fromArray(data: ArrayList<double>): Tensor` — create 1D tensor from list
- `Tensor.fromMatrix(data: ArrayList<ArrayList<double>>): Tensor` — create 2D tensor from nested list

**Autograd:**
- `requiresGrad: bool` — whether this tensor tracks gradients
- `setRequiresGrad(flag: bool): void` — enable/disable gradient tracking
- `grad(): Tensor` — accumulated gradient tensor
- `zeroGrad(): void` — zero out accumulated gradients
- `backward(): void` — compute gradients via reverse-mode autograd

**Access:**
- `get(indices: ArrayList<int>): double` — multi-dimensional access
- `set(indices: ArrayList<int>, value: double): void` — multi-dimensional set
- `getFlat(i: int): double` — linear index access
- `shape(): ArrayList<int>` — tensor shape
- `ndim(): int` — number of dimensions
- `size(): int` — total number of elements
- `reshape(newShape: ArrayList<int>): Tensor` — reshape (shares data when possible)
- `flatten(): Tensor` — collapse to 1D

**Operations:**
- `add(other: Tensor): Tensor` — element-wise addition
- `sub(other: Tensor): Tensor` — element-wise subtraction
- `mul(other: Tensor): Tensor` — element-wise multiplication (Hadamard)
- `div(other: Tensor): Tensor` — element-wise division
- `matmul(other: Tensor): Tensor` — matrix multiplication
- `scale(s: double): Tensor` — scalar multiplication
- `transpose(): Tensor` — transpose (reverse axes)
- `transpose2D(): Tensor` — 2D transpose
- `sum(): Tensor` — sum of all elements (scalar tensor)
- `mean(): Tensor` — mean of all elements
- `max(): Tensor` — maximum element
- `min(): Tensor` — minimum element
- `argMax(): Tensor` — index of maximum element
- `norm(): Tensor` — L2 norm
- `log(): Tensor` — element-wise natural log
- `exp(): Tensor` — element-wise exponential
- `pow(exponent: double): Tensor` — element-wise power
- `clip(lo: double, hi: double): Tensor` — clip values
- `slice(starts: ArrayList<int>, ends: ArrayList<int>): Tensor` — sub-tensor
- `concat(other: Tensor, axis: int): Tensor` — concatenate along axis

**Operators:**
- `operator+(other: Tensor): Tensor` — addition
- `operator-(other: Tensor): Tensor` — subtraction
- `operator*(scalar: double): Tensor` — scalar multiply
- `operator[](indices: ArrayList<int>): double` — index access

```titrate
let shape = new ArrayList<int>();
shape.add(2); shape.add(3);
let x = Tensor.randn(shape, 0.0, 1.0);
x.setRequiresGrad(true);
let y = x.matmul(x.transpose2D());
let loss = y.mean();
loss.backward();
let gradient = x.grad();
```

## Layer

Neural network layer building blocks. Each layer implements a forward pass and tracks trainable parameters.

- `fn init()` — base layer constructor

**Dense (fully connected):**
- `Layer.dense(inFeatures: int, outFeatures: int): Layer` — fully connected layer
- `forward(input: Tensor): Tensor` — compute output: W·x + b
- `weights(): Tensor` — weight matrix parameter
- `bias(): Tensor` — bias vector parameter

**Convolutional:**
- `Layer.conv2d(inChannels: int, outChannels: int, kernelSize: int): Layer` — 2D convolution
- `forward(input: Tensor): Tensor` — convolve input with kernel

**Recurrent:**
- `Layer.rnn(inputSize: int, hiddenSize: int): Layer` — simple RNN layer
- `Layer.lstm(inputSize: int, hiddenSize: int): Layer` — LSTM layer
- `forward(input: Tensor, hiddenState: Tensor): Tensor` — single step forward
- `hiddenState(): Tensor` — current hidden state

**Normalization:**
- `Layer.batchNorm(numFeatures: int): Layer` — batch normalization
- `Layer.layerNorm(numFeatures: int): Layer` — layer normalization
- `forward(input: Tensor): Tensor` — normalize input

**Regularization:**
- `Layer.dropout(rate: double): Layer` — dropout layer
- `forward(input: Tensor): Tensor` — apply dropout (no-op in eval mode)

**Activation functions (stateless layers):**
- `Layer.relu(): Layer` — ReLU activation
- `Layer.sigmoid(): Layer` — sigmoid activation
- `Layer.tanh(): Layer` — tanh activation
- `Layer.softmax(): Layer` — softmax activation
- `Layer.gelu(): Layer` — GELU activation
- `forward(input: Tensor): Tensor` — apply activation

**Parameter management:**
- `parameters(): ArrayList<Tensor>` — all trainable parameters
- `train(): void` — set training mode
- `eval(): void` — set evaluation mode (disables dropout, uses running stats for batchnorm)

```titrate
let dense = Layer.dense(128, 64);
let relu = Layer.relu();
let input = Tensor.randn(shape, 0.0, 1.0);
let output = relu.forward(dense.forward(input));
```

## Loss

Loss function modules for training neural networks. Each computes a scalar loss tensor from predictions and targets.

- `fn init()` — base loss constructor

**Regression losses:**
- `Loss.mse(): Loss` — mean squared error: mean((pred - target)²)
- `Loss.mae(): Loss` — mean absolute error: mean(|pred - target|)
- `Loss.huber(delta: double): Loss` — Huber loss: quadratic near 0, linear beyond delta

**Classification losses:**
- `Loss.crossEntropy(): Loss` — cross-entropy with softmax: -Σ target · log(softmax(pred))
- `Loss.binaryCrossEntropy(): Loss` — binary cross-entropy: -[target·log(pred) + (1-target)·log(1-pred)]
- `Loss.hinge(): Loss` — hinge loss (SVM-style): max(0, 1 - target · pred)

**Distance-based losses:**
- `Loss.klDivergence(): Loss` — KL-divergence: Σ target · log(target / pred)
- `Loss.cosineSimilarity(): Loss` — cosine similarity loss: 1 - cos(pred, target)

**Computation:**
- `compute(predictions: Tensor, targets: Tensor): Tensor` — compute scalar loss value

```titrate
let lossFn = Loss.crossEntropy();
let loss = lossFn.compute(predictions, targets);
loss.backward();
```

## Optimizer

Parameter optimizers with learning rate scheduling support.

- `fn init(parameters: ArrayList<Tensor>, lr: double)` — base optimizer constructor

**Optimizers:**
- `Optimizer.sgd(parameters: ArrayList<Tensor>, lr: double, momentum: double, nesterov: bool): Optimizer` — SGD with optional momentum and Nesterov acceleration
- `Optimizer.adam(parameters: ArrayList<Tensor>, lr: double, beta1: double, beta2: double, eps: double): Optimizer` — Adam optimizer
- `Optimizer.adamW(parameters: ArrayList<Tensor>, lr: double, weightDecay: double): Optimizer` — Adam with decoupled weight decay
- `Optimizer.rmsprop(parameters: ArrayList<Tensor>, lr: double, alpha: double, eps: double): Optimizer` — RMSProp optimizer
- `Optimizer.adagrad(parameters: ArrayList<Tensor>, lr: double, eps: double): Optimizer` — AdaGrad optimizer

**Step and state:**
- `step(): void` — perform one optimization step (update parameters)
- `zeroGrad(): void` — zero all parameter gradients
- `learningRate(): double` — current learning rate
- `setLearningRate(lr: double): void` — manually set learning rate

**Learning rate schedulers:**
- `stepScheduler(stepSize: int, gamma: double): void` — decay LR by gamma every stepSize epochs
- `cosineScheduler(tMax: int, etaMin: double): void` — cosine annealing schedule
- `warmupScheduler(warmupSteps: int, targetLr: double): void` — linear warmup to target LR
- `stepEpoch(): void` — advance scheduler by one epoch (call after each epoch)

```titrate
let params = model.parameters();
let opt = Optimizer.adam(params, 0.001, 0.9, 0.999, 1e-8);
opt.warmupScheduler(100, 0.001);
opt.cosineScheduler(1000, 1e-6);

// Training step
opt.zeroGrad();
let loss = lossFn.compute(predictions, targets);
loss.backward();
opt.step();
opt.stepEpoch();
```

## Model

Neural network model composition with sequential layer stacking, forward/backward passes, parameter management, and serialization.

- `fn init()` — create an empty sequential model

**Layer management:**
- `add(layer: Layer): void` — append a layer to the model
- `layers(): ArrayList<Layer>` — all layers in order
- `parameters(): ArrayList<Tensor>` — all trainable parameters from all layers

**Forward / backward:**
- `forward(input: Tensor): Tensor` — run input through all layers sequentially
- `backward(gradient: Tensor): void` — backpropagate gradient through all layers in reverse

**Training / evaluation:**
- `train(): void` — set all layers to training mode
- `eval(): void` — set all layers to evaluation mode

**Serialization:**
- `save(path: string): void` — save model parameters to file
- `load(path: string): void` — load model parameters from file

**Training loop helper:**
- `fit(dataLoader: DataLoader, lossFn: Loss, optimizer: Optimizer, epochs: int): ArrayList<double>` — run full training loop, returns loss per epoch
- `evaluate(dataLoader: DataLoader, lossFn: Loss): double` — evaluate average loss on a dataset

```titrate
let model = new Model();
model.add(Layer.dense(784, 128));
model.add(Layer.relu());
model.add(Layer.dropout(0.2));
model.add(Layer.dense(128, 10));
model.add(Layer.softmax());

let lossFn = Loss.crossEntropy();
let opt = Optimizer.adam(model.parameters(), 0.001, 0.9, 0.999, 1e-8);

let losses = model.fit(trainLoader, lossFn, opt, 10);
model.save("model_checkpoint.bin");
```

## DataLoader

Dataset abstraction with batching, shuffling, and train/validation/test splitting.

- `fn init(features: ArrayList<Tensor>, labels: ArrayList<Tensor>)` — create from feature/label lists

**Batching:**
- `batchSize: int` — current batch size
- `setBatchSize(size: int): void` — set batch size
- `batch(index: int): (Tensor, Tensor)` — get batch by index (features, labels)
- `numBatches(): int` — total number of batches

**Shuffling:**
- `shuffle(): void` — shuffle data in-place
- `setShuffle(flag: bool): void` — enable/disable automatic shuffling per epoch

**Splitting:**
- `split(trainRatio: double, valRatio: double): ArrayList<DataLoader>` — split into train/val/test loaders
- `splitKFold(k: int): ArrayList<DataLoader>` — split into k folds for cross-validation

**Data augmentation pipeline:**
- `augment(transform: fn(Tensor): Tensor): void` — add a transform to the augmentation pipeline
- `augmentRandomFlip(axis: int): void` — random flip augmentation
- `augmentRandomNoise(std: double): void` — add Gaussian noise augmentation
- `augmentRandomCrop(size: ArrayList<int>): void` — random crop augmentation

**Iteration:**
- `size(): int` — total number of samples
- `features(): ArrayList<Tensor>` — all feature tensors
- `labels(): ArrayList<Tensor>` — all label tensors

```titrate
let features = new ArrayList<Tensor>();
let labels = new ArrayList<Tensor>();
// ... populate features and labels ...
let loader = new DataLoader(features, labels);
loader.setBatchSize(32);
loader.setShuffle(true);
loader.augmentRandomNoise(0.01);

let splits = loader.split(0.8, 0.1);
let trainLoader = splits.get(0);
let valLoader = splits.get(1);
let testLoader = splits.get(2);

for (let i = 0; i < trainLoader.numBatches(); i++) {
    let (batchX, batchY) = trainLoader.batch(i);
    // train step...
}
```
