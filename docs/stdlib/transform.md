# transform

The `tt.math.transform` module provides real-time 3D transformation primitives: 4×4 transformation matrices, unit quaternions for rotation, composed transforms (position + rotation + scale), and color types for rendering.

```titrate
import tt.math.transform.Mat4;
import tt.math.transform.Quaternion;
import tt.math.transform.Transform;
import tt.math.transform.Color;
```

## Mat4

A 4×4 transformation matrix using column-major storage (OpenGL convention). Supports matrix arithmetic, inversion, point/vector transformation, and a set of factory functions for building common transformations.

- `fn init(m00..m33: double)` — create from 16 components (column-major)
- `get(row: int, col: int): double` — element access (row-major indexing into column-major storage)
- `set(row: int, col: int, val: double): void` — set an element
- `multiply(other: Mat4): Mat4` — matrix multiplication (`this * other`)
- `multiplyVec4(v: Vec4): Vec4` — multiply by a four-component vector
- `transpose(): Mat4` — transpose
- `determinant(): double` — determinant
- `inverse(): Mat4` — inverse via the cofactor method
- `transformPoint(v: Vec3): Vec3` — transform a point (applies translation, performs perspective divide)
- `transformVector(v: Vec3): Vec3` — transform a direction vector (ignores translation)
- `transformDirection(v: Vec3): Vec3` — transform a direction and normalize the result
- `toString(): string` — readable representation

**Factory functions:**

- `mat4Identity(): Mat4` — identity matrix
- `mat4Zero(): Mat4` — zero matrix
- `mat4Translation(x: double, y: double, z: double): Mat4` — translation matrix
- `mat4Scaling(x: double, y: double, z: double): Mat4` — scaling matrix
- `mat4RotationX(angle: double): Mat4` — rotation about the X axis (radians)
- `mat4RotationY(angle: double): Mat4` — rotation about the Y axis (radians)
- `mat4RotationZ(angle: double): Mat4` — rotation about the Z axis (radians)
- `mat4RotationAxis(axis: Vec3, angle: double): Mat4` — rotation about an arbitrary axis (radians, axis is normalized)
- `mat4Perspective(fovY: double, aspect: double, near: double, far: double): Mat4` — perspective projection matrix
- `mat4Orthographic(left: double, right: double, bottom: double, top: double, near: double, far: double): Mat4` — orthographic projection matrix
- `mat4LookAt(eye: Vec3, center: Vec3, up: Vec3): Mat4` — view matrix looking from `eye` toward `center`

```titrate
// Build a view-projection matrix
let view: Mat4 = mat4LookAt(
    new Vec3(0.0, 0.0, 5.0),
    new Vec3(0.0, 0.0, 0.0),
    new Vec3(0.0, 1.0, 0.0)
);
let proj: Mat4 = mat4Perspective(MathTrig.radians(60.0), 16.0 / 9.0, 0.1, 100.0);
let viewProj: Mat4 = proj.multiply(view);

// Transform a point through the matrix
let worldPoint: Vec3 = new Vec3(1.0, 2.0, 3.0);
let clipPoint: Vec3 = viewProj.transformPoint(worldPoint);
```

## Quaternion

A unit-quaternion representation for 3D rotation. Supports the Hamilton product, conjugate/inverse, interpolation (nlerp and slerp), and conversion to/from matrices and Euler angles.

- `fn init(w: double, x: double, y: double, z: double)` — create from components
- `multiply(q: Quaternion): Quaternion` — Hamilton product (`this * q`)
- `conjugate(): Quaternion` — conjugate `(w, -x, -y, -z)`
- `inverse(): Quaternion` — inverse (conjugate divided by norm²)
- `norm(): double` — magnitude
- `normalize(): Quaternion` — normalize to a unit quaternion
- `dot(q: Quaternion): double` — dot product
- `rotateVector(v: Vec3): Vec3` — rotate a vector (`q * v * q⁻¹`)
- `toMat4(): Mat4` — convert to a 4×4 rotation matrix
- `toEuler(): Vec3` — convert to Euler angles (roll, pitch, yaw in radians)
- `lerp(q: Quaternion, t: double): Quaternion` — normalized linear interpolation
- `slerp(q: Quaternion, t: double): Quaternion` — spherical linear interpolation (shortest path)
- `angle(): double` — rotation angle in radians
- `axis(): Vec3` — rotation axis
- `equals(q: Quaternion): bool` — approximate equality
- `toString(): string` — readable representation

**Factory functions:**

- `quatIdentity(): Quaternion` — identity quaternion (no rotation)
- `quatFromAxisAngle(axis: Vec3, angle: double): Quaternion` — from axis-angle (radians, axis is normalized)
- `quatFromEuler(pitch: double, yaw: double, roll: double): Quaternion` — from Euler angles (radians)
- `quatFromMat4(m: Mat4): Quaternion` — from a 4×4 rotation matrix
- `quatFromToRotation(from: Vec3, to: Vec3): Quaternion` — shortest rotation taking vector `from` onto `to`
- `quatLookRotation(forward: Vec3, up: Vec3): Quaternion` — rotation that faces the given forward direction

```titrate
// Rotate 90° about the Y axis
let q: Quaternion = quatFromAxisAngle(new Vec3(0.0, 1.0, 0.0), MathTrig.radians(90.0));
let v: Vec3 = q.rotateVector(new Vec3(1.0, 0.0, 0.0));  // ≈ (0, 0, -1)

// Smoothly interpolate between two orientations
let a: Quaternion = quatFromEuler(0.0, 0.0, 0.0);
let b: Quaternion = quatFromEuler(0.0, MathTrig.radians(90.0), 0.0);
let mid: Quaternion = a.slerp(b, 0.5);
```

## Transform

A composed transform combining position, rotation, and scale. Useful for scene graphs where child transforms are composed onto parent transforms.

- `fn init(position: Vec3, rotation: Quaternion, scale: Vec3)` — create from components
- `toMat4(): Mat4` — compose into a single 4×4 matrix (`T * R * S`)
- `translate(delta: Vec3): Transform` — return a new transform offset by `delta`
- `rotate(q: Quaternion): Transform` — return a new transform with `q` appended to the rotation
- `scaleBy(s: Vec3): Transform` — return a new transform with scale multiplied by `s`
- `compose(other: Transform): Transform` — compose this transform with `other` (parent * child)
- `inverse(): Transform` — inverse transform
- `lerp(other: Transform, t: double): Transform` — interpolate (position/scale lerp, rotation slerp)
- `transformPoint(point: Vec3): Vec3` — transform a point through this transform
- `transformDirection(dir: Vec3): Vec3` — transform a direction (rotation only)
- `equals(other: Transform): bool` — component-wise equality
- `toString(): string` — readable representation

**Factory functions:**

- `transformIdentity(): Transform` — identity transform (origin, no rotation, unit scale)
- `transformFromMat4(m: Mat4): Transform` — decompose a matrix into position, rotation, and scale

```titrate
// Parent and child transforms
let parent: Transform = new Transform(
    new Vec3(10.0, 0.0, 0.0),
    quatFromAxisAngle(new Vec3(0.0, 1.0, 0.0), MathTrig.radians(45.0)),
    new Vec3(1.0, 1.0, 1.0)
);
let child: Transform = new Transform(
    new Vec3(2.0, 0.0, 0.0),
    quatIdentity(),
    new Vec3(1.0, 1.0, 1.0)
);
let world: Transform = parent.compose(child);
let worldMatrix: Mat4 = world.toMat4();

// Transform a local point into world space
let worldPoint: Vec3 = parent.transformPoint(new Vec3(1.0, 0.0, 0.0));
```

## Color

An RGB color with components clamped to the `[0.0, 1.0]` range. Supports HSV conversion, hex parsing/formatting, luminance, and interpolation.

- `fn init(r: double, g: double, b: double)` — create (components clamped to 0–1)
- `lerp(other: Color, t: double): Color` — linear interpolation
- `toRGBA(a: double): RGBA` — convert to RGBA with the given alpha
- `toHSV(): Vec3` — convert to HSV (hue in degrees 0–360, saturation/value 0–1)
- `toHex(): string` — hex string (`#rrggbb`)
- `luminance(): double` — relative luminance (Rec. 709 coefficients)
- `brightness(): double` — gamma-corrected perceived brightness
- `equals(other: Color): bool` — approximate equality
- `toString(): string` — readable representation

## RGBA

An RGB color with an alpha channel, all clamped to `[0.0, 1.0]`.

- `fn init(r: double, g: double, b: double, a: double)` — create (components clamped to 0–1)
- `lerp(other: RGBA, t: double): RGBA` — linear interpolation
- `toRGB(): Color` — drop the alpha channel
- `toHSV(): Vec3` — convert to HSV
- `toHex(): string` — hex string (`#rrggbbaa`)
- `withAlpha(a: double): RGBA` — return a copy with the given alpha
- `premultiply(): RGBA` — return a premultiplied-alpha copy
- `luminance(): double` — relative luminance
- `equals(other: RGBA): bool` — approximate equality
- `toString(): string` — readable representation

**Factory functions:**

- `colorFromHSV(h: double, s: double, v: double): Color` — create from HSV (hue in degrees)
- `colorFromHex(hex: string): Color` — parse a `#rrggbb` hex string
- `rgbaFromHSV(h: double, s: double, v: double, a: double): RGBA` — create RGBA from HSV
- `rgbaFromHex(hex: string): RGBA` — parse a `#rrggbbaa` hex string
- `colorLerp(a: Color, b: Color, t: double): Color` — interpolate two colors
- `rgbaLerp(a: RGBA, b: RGBA, t: double): RGBA` — interpolate two RGBA colors
- `colorByName(name: string): Color` — look up a named CSS color (loaded from `data/color/named_colors.json`)

```titrate
// Create colors from different representations
let red: Color = new Color(1.0, 0.0, 0.0);
let fromHex: Color = colorFromHex("#00ff00");
let fromHSV: Color = colorFromHSV(240.0, 1.0, 1.0);  // blue

// Interpolate and convert
let blended: Color = red.lerp(fromHex, 0.5);
let hex: string = blended.toHex();           // "#808000"
let lum: double = red.luminance();           // 0.2126

// RGBA with alpha
let translucent: RGBA = red.toRGBA(0.5);
let premult: RGBA = translucent.premultiply();
let named: Color = colorByName("coral");
```
