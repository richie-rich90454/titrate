# 3D Graphics and Game Development with Titrate

Titrate's geometry and math modules provide the building blocks for 3D graphics programming — from vector math and matrix transforms to collision detection and computational geometry. This guide covers the fundamentals and walks through a simple ray tracer.

## 3D Math Fundamentals

All 3D graphics start with linear algebra. Titrate's `tt.math` module provides vectors, matrices, and quaternions optimized for graphics workloads.

### Vectors

```titrate
import tt::math::vec::Vec3;
import tt::math::vec::Vec4;

// Create vectors
let a = new Vec3(1.0, 2.0, 3.0);
let b = new Vec3(4.0, 5.0, 6.0);

// Basic operations
let sum = a + b;              // (5, 7, 9)
let diff = a - b;             // (-3, -3, -3)
let scaled = a * 2.0;         // (2, 4, 6)
let negated = -a;             // (-1, -2, -3)

// Dot and cross products
let dot = a.dot(b);           // 32.0
let cross = a.cross(b);       // (-3, 6, -3)

// Length and normalization
let len = a.magnitude();      // √14
let norm = a.normalized();    // unit vector in same direction
```

### Matrices

```titrate
import tt::math::transform::Mat4;

// Identity matrix
let identity = Mat4.eye();

// Translation matrix
let translate = Mat4.translation(1.0, 2.0, 3.0);

// Rotation matrix (angle in radians, axis)
let rotY = Mat4.rotationY(1.5708);  // 90° around Y
let rotAxis = Mat4.rotationAxis(new Vec3(0.0, 1.0, 0.0), 1.5708);

// Scale matrix
let scale = Mat4.scaling(2.0, 2.0, 2.0);

// Matrix multiplication (composition of transforms)
let model = translate.mulMat4(rotY).mulMat4(scale);

// Transform a point
let point = new Vec4(1.0, 0.0, 0.0, 1.0);
let transformed = model.mulVec4(point);
```

### Quaternions

Quaternions avoid gimbal lock and are more efficient for interpolating rotations:

```titrate
import tt::math::transform::Quaternion;

// Create from axis-angle
let q1 = Quaternion.fromAxisAngle(new Vec3(0.0, 1.0, 0.0), 1.5708);  // 90° around Y

// Create from Euler angles (radians)
let q2 = Quaternion.fromEuler(0.0, 1.5708, 0.0);  // yaw = 90°

// Compose rotations
let combined = q1.mul(q2);

// Spherical linear interpolation (SLERP)
let t = 0.5;
let interpolated = q1.slerp(q2, t);

// Convert to rotation matrix
let rotMatrix = q1.toMat4();

// Rotate a vector
let v = new Vec3(1.0, 0.0, 0.0);
let rotated = q1.rotate(v);
```

## Geometry Primitives

The `tt::math::geometry3d` module provides geometric primitives used in collision detection, ray casting, and spatial queries.

```titrate
import tt::math::geometry3d::AABB;
import tt::math::geometry3d::Ray;
import tt::math::geometry3d::Plane;
import tt::math::geometry3d::Triangle;
import tt::math::geometry3d::Sphere;
import tt::math::geometry3d::Frustum;
```

### AABB (Axis-Aligned Bounding Box)

```titrate
// Create from min/max corners
let box = new AABB(new Vec3(-1.0, -1.0, -1.0),
                   new Vec3(1.0, 1.0, 1.0));

// Test containment
let inside = box.contains(new Vec3(0.0, 0.0, 0.0));   // true
let outside = box.contains(new Vec3(5.0, 0.0, 0.0));  // false

// Test intersection with another AABB
let other = new AABB(new Vec3(0.5, 0.5, 0.5),
                     new Vec3(2.0, 2.0, 2.0));
let intersects = box.intersectsAABB(other);  // true

// Get center and extents
let center = box.center();
let extents = box.extents();
```

### Ray

```titrate
// Create a ray with origin and direction
let ray = new Ray(new Vec3(0.0, 0.0, 5.0),    // origin
                  new Vec3(0.0, 0.0, -1.0));   // direction (normalized)

// Intersect with sphere
let sphere = new Sphere(new Vec3(0.0, 0.0, 0.0), 1.0);  // center, radius
let hit = ray.intersectSphere(sphere);
if (hit.isOk()) {
    let t = hit.unwrap();  // distance along ray to hit point
    let hitPoint = ray.pointAt(t);
    io::println("Hit at t=" + Double.toString(t));
}

// Intersect with plane
let plane = new Plane(new Vec3(0.0, 1.0, 0.0), 0.0);  // normal, distance
let planeHit = ray.intersectPlane(plane);

// Intersect with triangle
let tri = new Triangle(new Vec3(-1.0, 0.0, 0.0),
                       new Vec3(1.0, 0.0, 0.0),
                       new Vec3(0.0, 1.0, 0.0));
let triHit = ray.intersectTriangle(tri);
```

### Plane

```titrate
let plane = new Plane(new Vec3(0.0, 1.0, 0.0), 0.0);  // Y=0 ground plane

// Classify a point relative to the plane
let dist = plane.distanceTo(new Vec3(0.0, 5.0, 0.0));  // 5.0 (above)
let onPlane = plane.distanceTo(new Vec3(3.0, 0.0, -2.0));  // 0.0

// Project a point onto the plane
let projected = plane.project(new Vec3(2.0, 7.0, 3.0));  // (2, 0, 3)
```

### Sphere

```titrate
let sphere = new Sphere(new Vec3(0.0, 0.0, 0.0), 2.0);

// Containment test
let inside = sphere.contains(new Vec3(1.0, 0.0, 0.0));  // true

// Sphere-sphere intersection
let other = new Sphere(new Vec3(3.0, 0.0, 0.0), 2.0);
let overlap = sphere.intersectsSphere(other);  // true (touching)
```

### Frustum

```titrate
// Create from view-projection matrix
let viewProj = viewMatrix.mulMat4(projMatrix);
let frustum = new Frustum(viewProj);

// Test if a point is inside the frustum
let visible = frustum.containsPoint(new Vec3(0.0, 0.0, -10.0));

// Test if an AABB is inside the frustum
let inView = frustum.containsAABB(box);

// Test if a sphere is inside the frustum
let sphereVisible = frustum.containsSphere(sphere);
```

## Computational Geometry

The `tt.geom` module provides algorithms for convex hulls, Delaunay triangulation, and spatial indexing.

```titrate
import tt::geom::ConvexHull;
import tt::geom::Delaunay;
import tt::geom::SpatialIndex;
```

### Convex Hull

```titrate
let hull = new ConvexHull();

// Add 3D points
hull.addPoint(new Vec3(0.0, 0.0, 0.0));
hull.addPoint(new Vec3(1.0, 0.0, 0.0));
hull.addPoint(new Vec3(0.0, 1.0, 0.0));
hull.addPoint(new Vec3(0.0, 0.0, 1.0));
hull.addPoint(new Vec3(0.5, 0.5, 0.5));  // interior point

// Compute the hull
hull.compute();

let vertices = hull.vertices();
let faces = hull.faces();
io::println("Hull has " + Integer.toString(vertices.size()) + " vertices");
io::println("Hull has " + Integer.toString(faces.size()) + " faces");
```

### Delaunay Triangulation

```titrate
let delaunay = new Delaunay();

// Add 2D points
delaunay.addPoint(0.0, 0.0);
delaunay.addPoint(1.0, 0.0);
delaunay.addPoint(0.0, 1.0);
delaunay.addPoint(1.0, 1.0);
delaunay.addPoint(0.5, 0.5);

// Compute triangulation
delaunay.compute();

let triangles = delaunay.triangles();
for (i in 0..triangles.size()) {
    let tri = triangles.get(i);
    io::println("Triangle: (" + Integer.toString(tri.a) + ", " +
                Integer.toString(tri.b) + ", " + Integer.toString(tri.c) + ")");
}
```

### Spatial Index

```titrate
// Create a spatial index for fast nearest-neighbor and range queries
let index = new SpatialIndex();

// Insert objects with positions
index.insert(new Vec3(1.0, 2.0, 3.0), "object_a");
index.insert(new Vec3(4.0, 5.0, 6.0), "object_b");
index.insert(new Vec3(7.0, 8.0, 9.0), "object_c");

// Nearest neighbor query
let nearest = index.nearest(new Vec3(1.5, 2.5, 3.5));
io::println("Nearest: " + nearest.value);

// Range query (all objects within radius)
let nearby = index.queryRadius(new Vec3(1.5, 2.5, 3.5), 5.0);
io::println("Nearby count: " + Integer.toString(nearby.size()));
```

## Transform Pipeline

The standard 3D transform pipeline composes model, view, and projection matrices to move geometry from object space to screen space.

### Model / View / Projection

```titrate
import tt::math::transform::Mat4;
import tt::math::Vec3;

// Model matrix: object space → world space
let model = Mat4.translation(0.0, 0.0, -5.0)
    .mulMat4(Mat4.rotationY(0.7854))   // 45° rotation
    .mulMat4(Mat4.scaling(1.0, 1.0, 1.0));

// View matrix: world space → camera space
let eye = new Vec3(0.0, 2.0, 10.0);
let target = new Vec3(0.0, 0.0, 0.0);
let up = new Vec3(0.0, 1.0, 0.0);
let view = Mat4.lookAt(eye, target, up);

// Projection matrix: camera space → clip space
let fov = 1.0472;  // 60° in radians
let aspect = 16.0 / 9.0;
let near = 0.1;
let far = 100.0;
let proj = Mat4.perspective(fov, aspect, near, far);

// Full MVP matrix
let mvp = proj.mulMat4(view).mulMat4(model);

// Transform a vertex
let vertex = new Vec4(1.0, 1.0, 1.0, 1.0);
let clipPos = mvp.mulVec4(vertex);

// Perspective divide → NDC
let w = clipPos.w;
let ndc = new Vec3(clipPos.x / w, clipPos.y / w, clipPos.z / w);
```

### Affine Transforms

```titrate
// Compose a chain of transforms
let transform = Mat4.identity();

// Translate, then rotate, then scale (applied right-to-left)
transform = transform.mulMat4(Mat4.translation(3.0, 0.0, 0.0));
transform = transform.mulMat4(Mat4.rotationZ(1.5708));
transform = transform.mulMat4(Mat4.scaling(2.0, 2.0, 2.0));

// Inverse transform (for ray picking, etc.)
let invTransform = transform.inverse();
```

## Collision Detection

Collision detection typically uses a two-phase approach: a broad phase (fast, approximate) followed by a narrow phase (precise, expensive).

### Broad Phase with SpatialIndex

```titrate
import tt::geom::SpatialIndex;
import tt::math::geometry3d::AABB;

let index = new SpatialIndex();

// Register all objects with their AABBs
let box1 = new AABB(new Vec3(-1.0, -1.0, -1.0), new Vec3(1.0, 1.0, 1.0));
let box2 = new AABB(new Vec3(0.5, 0.5, 0.5), new Vec3(2.5, 2.5, 2.5));
let box3 = new AABB(new Vec3(10.0, 10.0, 10.0), new Vec3(12.0, 12.0, 12.0));

index.insert(box1.center(), "obj1");
index.insert(box2.center(), "obj2");
index.insert(box3.center(), "obj3");

// Find potential collision pairs (broad phase)
let pairs = index.queryRadius(box1.center(), 5.0);
```

### Narrow Phase with Geometry Primitives

```titrate
// Precise intersection tests
let sphere1 = new Sphere(new Vec3(0.0, 0.0, 0.0), 1.0);
let sphere2 = new Sphere(new Vec3(1.5, 0.0, 0.0), 1.0);

// Sphere-sphere
let colliding = sphere1.intersectsSphere(sphere2);  // true

// AABB-AABB
let aabbHit = box1.intersectsAABB(box2);  // true

// Ray-AABB
let ray = new Ray(new Vec3(-5.0, 0.0, 0.0), new Vec3(1.0, 0.0, 0.0));
let rayHit = ray.intersectAABB(box1);
if (rayHit.isOk()) {
    io::println("Ray hits box at t=" + Double.toString(rayHit.unwrap()));
}

// Triangle-ray (for mesh collision)
let tri = new Triangle(new Vec3(0.0, 1.0, 0.0),
                       new Vec3(-1.0, -1.0, 0.0),
                       new Vec3(1.0, -1.0, 0.0));
let triHit = ray.intersectTriangle(tri);
```

## End-to-End Example: Simple Ray Tracer

This example renders a scene with spheres and a ground plane using basic ray tracing.

```titrate
import tt::math::Vec3;
import tt::math::transform::Mat4;
import tt::geometry3d::Ray;
import tt::geometry3d::Sphere;
import tt::geometry3d::Plane;
import tt::math::MathAdvanced;

public class HitRecord {
    public double t;
    public Vec3 point;
    public Vec3 normal;
    public int materialId;

    public fn init(tt: double, p: Vec3, n: Vec3, matId: int) {
        this.t = tt;
        this.point = p;
        this.normal = n;
        this.materialId = matId;
    }
}

public class SceneObject {
    public Sphere sphere;
    public int materialId;

    public fn init(s: Sphere, matId: int) {
        this.sphere = s;
        this.materialId = matId;
    }
}

public fn traceRay(ray: Ray, objects: ArrayList<SceneObject>, ground: Plane): HitRecord {
    let closest: HitRecord = null;
    let minT = 1e30;

    // Test ground plane
    let groundHit = ray.intersectPlane(ground);
    if (groundHit.isOk()) {
        let t = groundHit.unwrap();
        if (t > 0.001 && t < minT) {
            minT = t;
            let point = ray.pointAt(t);
            closest = new HitRecord(t, point, new Vec3(0.0, 1.0, 0.0), 0);
        }
    }

    // Test each sphere
    for (obj in objects) {
        let hit = ray.intersectSphere(obj.sphere);
        if (hit.isOk()) {
            let t = hit.unwrap();
            if (t > 0.001 && t < minT) {
                minT = t;
                let point = ray.pointAt(t);
                let normal = (point - obj.sphere.center).normalized();
                closest = new HitRecord(t, point, normal, obj.materialId);
            }
        }
    }

    return closest;
}

public fn shade(hit: HitRecord, lightDir: Vec3): double {
    if (hit == null) { return 0.0; }

    // Lambertian diffuse shading
    let nDotL = hit.normal.dot(lightDir);
    let diffuse = Math.max(0.0, nDotL);

    // Ambient
    let ambient = 0.1;

    // Material-specific base color
    let base: double;
    if (hit.materialId == 0) {
        // Checkerboard ground
        let fx = Math.floor(hit.point.x);
        let fz = Math.floor(hit.point.z);
        if ((fx as int + fz as int) % 2 == 0) {
            base = 0.9;
        } else {
            base = 0.3;
        }
    } else if (hit.materialId == 1) {
        base = 0.8;  // red sphere
    } else {
        base = 0.6;  // blue sphere
    }

    return base * (ambient + 0.9 * diffuse);
}

public fn main(): void {
    let width = 80;
    let height = 40;
    let aspect = Double.parseDouble(Integer.toString(width)) /
                 Double.parseDouble(Integer.toString(height));

    // Scene setup
    let objects = new ArrayList<SceneObject>();
    objects.add(new SceneObject(new Sphere(new Vec3(0.0, 1.0, -5.0), 1.0), 1));
    objects.add(new SceneObject(new Sphere(new Vec3(2.5, 0.5, -4.0), 0.5), 2));
    let ground = new Plane(new Vec3(0.0, 1.0, 0.0), 0.0);

    // Camera
    let origin = new Vec3(0.0, 2.0, 2.0);
    let lightDir = new Vec3(1.0, 1.0, 1.0).normalized();

    // Render
    let chars = " .:-=+*#%@";
    for (y in 0..height) {
        let row = "";
        for (x in 0..width) {
            let u = (Double.parseDouble(Integer.toString(x)) /
                     Double.parseDouble(Integer.toString(width))) * 2.0 - 1.0;
            let v = 1.0 - (Double.parseDouble(Integer.toString(y)) /
                           Double.parseDouble(Integer.toString(height))) * 2.0;

            let dir = new Vec3(u * aspect, v, -1.0).normalized();
            let ray = new Ray(origin, dir);

            let hit = traceRay(ray, objects, ground);
            let brightness = shade(hit, lightDir);

            let charIdx = Math.min(9, Math.floor(brightness * 10.0) as int);
            let charIdxClamped = Math.max(0, charIdx);
            row = row + String.charAt(chars, charIdxClamped);
        }
        io::println(row);
    }
}
```

::: tip Start simple, then optimize
This ray tracer uses brute-force intersection testing against every object. For scenes with thousands of objects, use `SpatialIndex` or a bounding volume hierarchy (BVH) to accelerate ray queries from O(n) to O(log n).
:::

## What's Next?

- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [Physics Simulation](./physics-guide) — particle dynamics and force fields
- [Standard Library](./stdlib) — full module reference
