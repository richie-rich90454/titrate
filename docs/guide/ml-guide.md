# Machine Learning with Titrate

Titrate's `tt.ml` module provides a complete toolkit for building, training, and evaluating machine learning models — from tensor operations with automatic differentiation to neural network layers, optimizers, and training loops. This guide covers the fundamentals and walks through a complete image classification example.

## Tensors and Autograd

The `Tensor` class is the foundation of the ML module. Tensors are multi-dimensional arrays that can track gradients for automatic differentiation.

### Creating Tensors

```titrate
import tt::ml::Tensor;

// From a flat array with shape
let t1 = Tensor.fromArray([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3));

// Zeros and ones
let zeros = Tensor.zeros((3, 4));
let ones = Tensor.ones((2, 2));

// Random tensor (uniform [0, 1))
let rand = Tensor.rand((3, 3));

// Random normal (mean=0, std=1)
let randn = Tensor.randn((3, 3));

// Scalar tensor
let scalar = Tensor.scalar(3.14);
```

### Tensor Operations

```titrate
let a = Tensor.fromArray([1.0, 2.0, 3.0], (3,));
let b = Tensor.fromArray([4.0, 5.0, 6.0], (3,));

// Element-wise arithmetic
let sum = a + b;          // [5.0, 7.0, 9.0]
let diff = a - b;         // [-3.0, -3.0, -3.0]
let prod = a * b;         // [4.0, 10.0, 18.0] (element-wise)
let scaled = a * 2.0;     // [2.0, 4.0, 6.0]

// Matrix multiplication
let m1 = Tensor.fromArray([1.0, 2.0, 3.0, 4.0], (2, 2));
let m2 = Tensor.fromArray([5.0, 6.0, 7.0, 8.0], (2, 2));
let mm = m1.matmul(m2);

// Reductions
let s = a.sum();          // 6.0
let m = a.mean();         // 2.0
let mx = a.max();         // 3.0

// Reshape
let flat = Tensor.fromArray([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (6,));
let matrix = flat.reshape((2, 3));

// Transpose
let transposed = matrix.transpose();
```

### Automatic Differentiation (Autograd)

Tensors can track operations for gradient computation. Set `requiresGrad` to `true` on leaf tensors, then call `backward()` to compute gradients.

```titrate
// Create tensors with gradient tracking
let x = Tensor.fromArray([2.0, 3.0], (2,));
x.setRequiresGrad(true);

let w = Tensor.fromArray([0.5, 1.5], (2,));
w.setRequiresGrad(true);

// Forward pass: y = sum(w * x)
let y = w.mul(x).sum();

// Backward pass: compute dy/dw and dy/dx
y.backward();

// Read gradients
let gradW = w.grad();  // [2.0, 3.0] (dy/dw = x)
let gradX = x.grad();  // [0.5, 1.5] (dy/dx = w)

io::println("dy/dw = " + gradW.toString());
io::println("dy/dx = " + gradX.toString());
```

### Gradient Computation for a Simple Function

```titrate
// Compute gradient of f(x) = x² + 2x + 1 at x = 3
let x = Tensor.scalar(3.0);
x.setRequiresGrad(true);

let x2 = x * x;       // x²
let twoX = x * 2.0;   // 2x
let f = x2 + twoX + Tensor.scalar(1.0);

f.backward();

// f'(x) = 2x + 2 = 2(3) + 2 = 8
let gradient = x.grad();
io::println("f'(3) = " + gradient.toString());  // 8.0
```

## Building Neural Networks

The `tt.ml` module provides composable layers for building neural networks.

### Dense (Fully Connected) Layer

```titrate
import tt::ml::Layer;

// Create a dense layer: 784 inputs → 128 outputs
let dense1 = Layer.Dense(784, 128);

// Forward pass
let input = Tensor.randn((32, 784));  // batch of 32 samples
let output = dense1.forward(input);    // shape (32, 128)
```

### Conv2D Layer

```titrate
// Conv2D: 1 input channel, 32 output channels, 3×3 kernel
let conv = Layer.Conv2D(1, 32, 3);

// Input: batch of 16 images, 1 channel, 28×28
let images = Tensor.randn((16, 1, 28, 28));
let features = conv.forward(images);  // shape (16, 32, 28, 28)
```

### RNN and LSTM

```titrate
// LSTM layer: input size 64, hidden size 128
let lstm = Layer.LSTM(64, 128);

// Process a sequence: batch=8, seq_len=20, features=64
let sequence = Tensor.randn((8, 20, 64));
let (output, hidden) = lstm.forward(sequence);
// output shape: (8, 20, 128)
```

### Activation Functions

```titrate
import tt::ml::Activation;

let x = Tensor.fromArray([-2.0, -1.0, 0.0, 1.0, 2.0], (5,));

let relu = Activation.relu(x);          // [0.0, 0.0, 0.0, 1.0, 2.0]
let sigmoid = Activation.sigmoid(x);    // [0.119, 0.269, 0.5, 0.731, 0.881]
let tanh = Activation.tanh(x);          // [-0.964, -0.762, 0.0, 0.762, 0.964]
let softmax = Activation.softmax(x);    // normalized to sum=1
```

### Composing Layers into a Model

```titrate
import tt::ml::Model;

public class SimpleMLP extends Model {
    public Layer.Dense fc1;
    public Layer.Dense fc2;
    public Layer.Dense fc3;

    public fn init() {
        this.fc1 = Layer.Dense(784, 256);
        this.fc2 = Layer.Dense(256, 64);
        this.fc3 = Layer.Dense(64, 10);
    }

    public fn forward(x: Tensor): Tensor {
        let h1 = Activation.relu(this.fc1.forward(x));
        let h2 = Activation.relu(this.fc2.forward(h1));
        let out = this.fc3.forward(h2);
        return out;
    }
}
```

## Loss Functions

The `tt.ml` module provides standard loss functions for regression and classification.

```titrate
import tt::ml::Loss;
```

### MSE (Mean Squared Error)

```titrate
// Regression loss
let predicted = Tensor.fromArray([2.5, 0.0, 2.1, 7.8], (4,));
let target = Tensor.fromArray([3.0, -0.5, 2.0, 8.0], (4,));

let mseLoss = Loss.mse(predicted, target);
io::println("MSE: " + mseLoss.toString());
```

### CrossEntropy

```titrate
// Classification loss (expects raw logits, not softmax output)
let logits = Tensor.fromArray([2.0, 1.0, 0.1, 3.0, 0.5], (1, 5));
let targetClass = Tensor.fromArray([3], (1,));  // class index 3

let ceLoss = Loss.crossEntropy(logits, targetClass);
io::println("CrossEntropy: " + ceLoss.toString());
```

### BinaryCrossEntropy

```titrate
// Binary classification
let predicted = Tensor.fromArray([0.9, 0.2, 0.8, 0.1], (4,));
let target = Tensor.fromArray([1.0, 0.0, 1.0, 0.0], (4,));

let bceLoss = Loss.binaryCrossEntropy(predicted, target);
```

### Huber Loss

```titrate
// Robust regression loss (less sensitive to outliers than MSE)
let huberLoss = Loss.huber(predicted, target, 1.0);  // delta=1.0
```

## Optimizers

Optimizers update model parameters using computed gradients.

```titrate
import tt::ml::Optimizer;
```

### SGD (Stochastic Gradient Descent)

```titrate
let model = new SimpleMLP();
let optimizer = Optimizer.SGD(model.parameters(), 0.01);  // learning rate = 0.01

// Training step
let output = model.forward(input);
let loss = Loss.crossEntropy(output, target);
loss.backward();
optimizer.step();
optimizer.zeroGrad();
```

### Adam

```titrate
let optimizer = Optimizer.Adam(model.parameters(), 0.001);  // lr = 0.001

// Adam with custom betas
let customAdam = Optimizer.Adam(model.parameters(), 0.001, 0.9, 0.999);
```

### AdamW

```titrate
// Adam with decoupled weight decay
let optimizer = Optimizer.AdamW(model.parameters(), 0.001, 0.01);  // lr, weight_decay
```

### Learning Rate Schedulers

```titrate
// Step decay: reduce LR by factor 0.1 every 30 epochs
let scheduler = Optimizer.StepLR(optimizer, 30, 0.1);

// Cosine annealing
let cosineScheduler = Optimizer.CosineAnnealingLR(optimizer, 100);  // T_max=100

// In training loop
for (epoch in 0..100) {
    // ... training ...
    scheduler.step();
}
```

## Training Loop

The `Model.fit` method provides a high-level training API, or you can write a custom loop for full control.

### Using Model.fit

```titrate
let model = new SimpleMLP();
let optimizer = Optimizer.Adam(model.parameters(), 0.001);

// Train for 10 epochs
model.fit(trainData, trainLabels, optimizer, Loss.crossEntropy, 10);
```

### Custom Training Loop

```titrate
import tt::ml::DataLoader;

public fn train(model: SimpleMLP, trainLoader: DataLoader, valLoader: DataLoader,
                optimizer: Optimizer, epochs: int): void {
    for (epoch in 0..epochs) {
        var totalLoss: double = 0.0;
        var correct: int = 0;
        var total: int = 0;

        // Training
        for (batch in trainLoader) {
            let inputs = batch.inputs;
            let labels = batch.labels;

            optimizer.zeroGrad();

            let output = model.forward(inputs);
            let loss = Loss.crossEntropy(output, labels);
            loss.backward();
            optimizer.step();

            totalLoss = totalLoss + loss.item();

            // Count correct predictions
            let preds = output.argmax(1);
            correct = correct + preds.eq(labels).sum() as int;
            total = total + labels.shape().0;
        }

        let avgLoss = totalLoss / Double.parseDouble(Integer.toString(trainLoader.batchCount()));
        let accuracy = Double.parseDouble(Integer.toString(correct)) /
                       Double.parseDouble(Integer.toString(total));

        io::println("Epoch " + Integer.toString(epoch + 1) +
                    " — Loss: " + Double.toString(avgLoss) +
                    " — Accuracy: " + Double.toString(accuracy));
    }
}
```

### DataLoader and Batch Generation

```titrate
// Create a DataLoader with batch size and shuffling
let trainLoader = new DataLoader(trainX, trainY, 32, true);   // batch=32, shuffle=true
let valLoader = new DataLoader(valX, valY, 64, false);        // batch=64, no shuffle

// Train/validation split
let (trainX, valX, trainY, valY) = DataLoader.split(data, labels, 0.8);
// 80% train, 20% validation
```

## End-to-End Example: Image Classification with a Simple CNN

This example builds a convolutional neural network for classifying 28×28 grayscale images (e.g., MNIST digits) into 10 categories.

```titrate
import tt::ml::Tensor;
import tt::ml::Model;
import tt::ml::Layer;
import tt::ml::Activation;
import tt::ml::Loss;
import tt::ml::Optimizer;
import tt::ml::DataLoader;

public class DigitCNN extends Model {
    public Layer.Conv2D conv1;
    public Layer.Conv2D conv2;
    public Layer.Dense fc1;
    public Layer.Dense fc2;

    public fn init() {
        // 1 input channel → 16 output channels, 3×3 kernel
        this.conv1 = Layer.Conv2D(1, 16, 3);
        // 16 → 32 channels, 3×3 kernel
        this.conv2 = Layer.Conv2D(16, 32, 3);
        // After two conv + pool: 32 * 5 * 5 = 800
        this.fc1 = Layer.Dense(800, 128);
        this.fc2 = Layer.Dense(128, 10);
    }

    public fn forward(x: Tensor): Tensor {
        // Conv block 1: conv → relu → max pool (28→26→13)
        let h = Activation.relu(this.conv1.forward(x));
        h = h.maxPool2D(2);

        // Conv block 2: conv → relu → max pool (13→11→5)
        h = Activation.relu(this.conv2.forward(h));
        h = h.maxPool2D(2);

        // Flatten: (batch, 32, 5, 5) → (batch, 800)
        h = h.flatten();

        // Fully connected
        h = Activation.relu(this.fc1.forward(h));
        h = this.fc2.forward(h);

        return h;
    }
}

public fn trainModel(): void {
    // Load and preprocess data
    let (trainX, trainY) = DataLoader.loadMNIST("train");
    let (testX, testY) = DataLoader.loadMNIST("test");

    // Normalize pixel values to [0, 1]
    trainX = trainX / 255.0;
    testX = testX / 255.0;

    // Create data loaders
    let trainLoader = new DataLoader(trainX, trainY, 64, true);
    let testLoader = new DataLoader(testX, testY, 256, false);

    // Initialize model and optimizer
    let model = new DigitCNN();
    let optimizer = Optimizer.Adam(model.parameters(), 0.001);
    let scheduler = Optimizer.StepLR(optimizer, 5, 0.5);  // halve LR every 5 epochs

    // Training loop
    let epochs = 10;
    for (epoch in 0..epochs) {
        var totalLoss: double = 0.0;
        var correct: int = 0;
        var total: int = 0;

        for (batch in trainLoader) {
            optimizer.zeroGrad();

            let output = model.forward(batch.inputs);
            let loss = Loss.crossEntropy(output, batch.labels);
            loss.backward();
            optimizer.step();

            totalLoss = totalLoss + loss.item();
            let preds = output.argmax(1);
            correct = correct + preds.eq(batch.labels).sum() as int;
            total = total + batch.labels.shape().0;
        }

        scheduler.step();

        let trainAcc = Double.parseDouble(Integer.toString(correct)) /
                       Double.parseDouble(Integer.toString(total));
        io::println("Epoch " + Integer.toString(epoch + 1) +
                    " — Loss: " + Double.toString(totalLoss) +
                    " — Train Acc: " + Double.toString(trainAcc));
    }

    // Evaluate on test set
    var testCorrect: int = 0;
    var testTotal: int = 0;
    for (batch in testLoader) {
        let output = model.forward(batch.inputs);
        let preds = output.argmax(1);
        testCorrect = testCorrect + preds.eq(batch.labels).sum() as int;
        testTotal = testTotal + batch.labels.shape().0;
    }

    let testAcc = Double.parseDouble(Integer.toString(testCorrect)) /
                  Double.parseDouble(Integer.toString(testTotal));
    io::println("Test Accuracy: " + Double.toString(testAcc));
}

public fn main(): void {
    trainModel();
}
```

::: tip Debugging training
If your model isn't learning, check these common issues:
1. **Learning rate too high** — loss oscillates or diverges. Try reducing by 10×.
2. **Learning rate too low** — loss barely decreases. Try increasing by 3×.
3. **Gradients vanishing** — switch from sigmoid/tanh to ReLU.
4. **Overfitting** — add dropout, reduce model size, or increase training data.
:::

## What's Next?

- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [Standard Library](./stdlib) — full module reference
- [Error Handling](./error-handling) — robust error handling with `Result`
