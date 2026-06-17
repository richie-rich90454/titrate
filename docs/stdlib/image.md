# image

The `tt.image` module provides image processing utilities including pixel-level manipulation, convolution kernels, morphological operations, thresholding, and geometric transforms.

```titrate
import tt.image.Image;
import tt.image.Kernel;
import tt.image.Morphology;
import tt.image.Threshold;
import tt.image.Transform;
```

## Image

Grayscale, RGB, and RGBA image class with pixel access, geometric operations, and format conversion.

- `fn init(width: int, height: int, channels: int)` — create a blank image (channels: 1=grayscale, 3=RGB, 4=RGBA)
- `Image.fromGrayscale(width: int, height: int): Image` — create a grayscale image
- `Image.fromRGB(width: int, height: int): Image` — create an RGB image
- `Image.fromRGBA(width: int, height: int): Image` — create an RGBA image
- `getWidth(): int` — image width in pixels
- `getHeight(): int` — image height in pixels
- `getChannels(): int` — number of color channels
- `getPixel(x: int, y: int): int` — get pixel value at (x, y) as packed integer
- `setPixel(x: int, y: int, value: int): void` — set pixel value at (x, y)
- `getPixelFloat(x: int, y: int, channel: int): double` — get a single channel value as a float in [0.0, 1.0]
- `setPixelFloat(x: int, y: int, channel: int, value: double): void` — set a single channel value from a float in [0.0, 1.0]
- `crop(x: int, y: int, w: int, h: int): Image` — extract a rectangular sub-image
- `resize(newWidth: int, newHeight: int, method: string): Image` — resize using `"nearest"`, `"bilinear"`, or `"bicubic"` interpolation
- `flipHorizontal(): Image` — flip image left-to-right
- `flipVertical(): Image` — flip image top-to-bottom
- `rotate90(): Image` — rotate 90° clockwise
- `rotate180(): Image` — rotate 180°
- `rotate270(): Image` — rotate 270° clockwise
- `toGrayscale(): Image` — convert to grayscale (luminance weighting)
- `toRGB(): Image` — convert to RGB
- `toRGBA(): Image` — convert to RGBA
- `copy(): Image` — deep copy of the image

```titrate
let img = Image.fromRGB(640, 480);
img.setPixel(100, 50, 0xFF0000);  // red pixel

// Read pixel
let px = img.getPixel(100, 50);

// Crop a region
let region = img.crop(10, 10, 200, 200);

// Resize with bilinear interpolation
let small = img.resize(320, 240, "bilinear");

// Convert to grayscale
let gray = img.toGrayscale();
```

## Kernel

Convolution kernels for image filtering. Provides common kernels and supports custom kernel application.

- `gaussianBlur(size: int, sigma: double): NDArray<double>` — Gaussian blur kernel
- `sharpen(): NDArray<double>` — sharpening kernel (3×3)
- `edgeDetect(): NDArray<double>` — edge detection kernel (3×3)
- `sobelX(): NDArray<double>` — Sobel horizontal edge detector (3×3)
- `sobelY(): NDArray<double>` — Sobel vertical edge detector (3×3)
- `laplacian(): NDArray<double>` — Laplacian kernel (3×3)
- `emboss(): NDArray<double>` — emboss kernel (3×3)
- `boxBlur(size: int): NDArray<double>` — box (mean) blur kernel
- `unsharpMask(amount: double): NDArray<double>` — unsharp mask kernel
- `apply(image: Image, kernel: NDArray<double>): Image` — apply a convolution kernel to an image

```titrate
// Apply Gaussian blur
let blurKernel = Kernel.gaussianBlur(5, 1.5);
let blurred = Kernel.apply(img, blurKernel);

// Sharpen an image
let sharpKernel = Kernel.sharpen();
let sharpened = Kernel.apply(img, sharpKernel);

// Edge detection with Sobel
let sx = Kernel.sobelX();
let sy = Kernel.sobelY();
let edgesX = Kernel.apply(grayImg, sx);
let edgesY = Kernel.apply(grayImg, sy);

// Emboss effect
let embossKernel = Kernel.emboss();
let embossed = Kernel.apply(img, embossKernel);
```

## Morphology

Mathematical morphology operations for binary and grayscale images. Uses a structuring element for shape-based filtering.

- `dilate(image: Image, kernelSize: int): Image` — dilation (expand bright regions)
- `erode(image: Image, kernelSize: int): Image` — erosion (shrink bright regions)
- `open(image: Image, kernelSize: int): Image` — opening (erosion then dilation, removes small bright spots)
- `close(image: Image, kernelSize: int): Image` — closing (dilation then erosion, fills small dark holes)
- `gradient(image: Image, kernelSize: int): Image` — morphological gradient (dilate − erode, edge outline)
- `topHat(image: Image, kernelSize: int): Image` — top-hat transform (image − open, extracts small bright features)
- `blackHat(image: Image, kernelSize: int): Image` — black-hat transform (close − image, extracts small dark features)
- `hitOrMiss(image: Image, foreground: NDArray<double>, background: NDArray<double>): Image` — hit-or-miss transform for pattern matching

```titrate
// Remove noise with opening
let cleaned = Morphology.open(binaryImg, 3);

// Fill gaps with closing
let filled = Morphology.close(binaryImg, 3);

// Edge detection via gradient
let edges = Morphology.gradient(binaryImg, 3);

// Extract small bright features
let smallBright = Morphology.topHat(grayImg, 5);

// Extract small dark features
let smallDark = Morphology.blackHat(grayImg, 5);
```

## Threshold

Thresholding and histogram-based image segmentation. Includes Otsu's method, adaptive thresholding, and histogram equalization.

- `binary(image: Image, threshold: double): Image` — binary threshold (pixels above threshold become white, below become black)
- `otsu(image: Image): Image` — Otsu's automatic threshold selection
- `otsuThreshold(image: Image): double` — compute the optimal Otsu threshold value
- `adaptive(image: Image, blockSize: int, constant: double): Image` — adaptive thresholding using local mean
- `adaptiveGaussian(image: Image, blockSize: int, constant: double): Image` — adaptive thresholding using Gaussian-weighted local mean
- `colorThreshold(image: Image, channel: int, minVal: double, maxVal: double): Image` — threshold on a specific color channel
- `histogramEqualization(image: Image): Image` — histogram equalization for contrast enhancement
- `clahe(image: Image, clipLimit: double, gridSize: int): Image` — contrast-limited adaptive histogram equalization

```titrate
// Simple binary threshold
let binary = Threshold.binary(grayImg, 128.0);

// Otsu's method (automatic threshold)
let otsuResult = Threshold.otsu(grayImg);
let threshVal = Threshold.otsuThreshold(grayImg);
io::println("Otsu threshold: " + Double.toString(threshVal));

// Adaptive thresholding for uneven lighting
let adaptive = Threshold.adaptive(grayImg, 11, 2.0);

// Color-based thresholding
let redMask = Threshold.colorThreshold(rgbImg, 0, 0.5, 1.0);

// Enhance contrast
let equalized = Threshold.histogramEqualization(grayImg);
let enhanced = Threshold.clahe(grayImg, 2.0, 8);
```

## Transform

Geometric transforms for image warping. Supports affine, perspective, and individual rotation/scaling/translation/shear operations.

- `affine(image: Image, matrix: NDArray<double>, interpolation: string): Image` — apply a 2×3 affine transform matrix (`"nearest"`, `"bilinear"`, `"bicubic"`)
- `perspective(image: Image, matrix: NDArray<double>, interpolation: string): Image` — apply a 3×3 perspective transform matrix
- `rotate(image: Image, angle: double, interpolation: string): Image` — rotate image by angle in degrees (counter-clockwise)
- `scale(image: Image, factorX: double, factorY: double, interpolation: string): Image` — scale image by factors
- `translate(image: Image, dx: int, dy: int): Image` — translate image by (dx, dy) pixels
- `shear(image: Image, shearX: double, shearY: double, interpolation: string): Image` — shear image along X and/or Y axis
- `flipHorizontal(image: Image): Image` — flip left-to-right
- `flipVertical(image: Image): Image` — flip top-to-bottom
- `makeRotationMatrix(angle: double): NDArray<double>` — construct a 2×3 rotation matrix
- `makeScaleMatrix(factorX: double, factorY: double): NDArray<double>` — construct a 2×3 scale matrix
- `makeTranslationMatrix(dx: int, dy: int): NDArray<double>` — construct a 2×3 translation matrix
- `makeShearMatrix(shearX: double, shearY: double): NDArray<double>` — construct a 2×3 shear matrix

```titrate
// Rotate 45 degrees with bilinear interpolation
let rotated = Transform.rotate(img, 45.0, "bilinear");

// Scale to 150%
let scaled = Transform.scale(img, 1.5, 1.5, "bicubic");

// Translate by (20, 10) pixels
let shifted = Transform.translate(img, 20, 10);

// Shear horizontally
let sheared = Transform.shear(img, 0.2, 0.0, "bilinear");

// Compose affine transforms manually
let rotMat = Transform.makeRotationMatrix(30.0);
let transMat = Transform.makeTranslationMatrix(50, 30);
let combined = Transform.affine(img, rotMat, "bilinear");
```
