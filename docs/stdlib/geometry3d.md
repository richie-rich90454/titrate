# geometry3d

The `tt.math.geometry3d` module provides 3D geometric primitives for bounding volumes, intersection testing, ray casting, and frustum culling.

```titrate
import tt.math.geometry3d.AABB;
import tt.math.geometry3d.Ray;
import tt.math.geometry3d.Plane;
import tt.math.geometry3d.Triangle;
import tt.math.geometry3d.Sphere;
import tt.math.geometry3d.Frustum;
```

## AABB

Axis-aligned bounding box defined by minimum and maximum corners.

- `fn init(min: Vec3, max: Vec3)` — create from min/max corners
- `center(): Vec3` — center point of the box
- `size(): Vec3` — dimensions of the box
- `contains(point: Vec3): bool` — check if a point is inside
- `containsAABB(other: AABB): bool` — check if another AABB is fully inside
- `intersects(other: AABB): bool` — check for overlap
- `merge(other: AABB): AABB` — smallest AABB containing both
- `expand(point: Vec3): AABB` — expand to include a point
- `surfaceArea(): double` — total surface area
- `volume(): double` — volume of the box

```titrate
let box = new AABB(new Vec3(-1.0, -1.0, -1.0), new Vec3(1.0, 1.0, 1.0));
let c = box.center();          // Vec3(0, 0, 0)
let v = box.volume();          // 8.0
let inside = box.contains(new Vec3(0.5, 0.5, 0.5));  // true
```

## Ray

A 3D ray with origin and direction (automatically normalized). Supports intersection tests against AABBs, planes, spheres, and triangles using the Möller–Trumbore algorithm.

- `fn init(origin: Vec3, direction: Vec3)` — create ray (direction is normalized)
- `pointAt(t: double): Vec3` — point at parameter t along the ray
- `intersectsAABB(box: AABB): bool` — check if ray hits the AABB
- `intersectsAABBRange(box: AABB): double` — returns t at intersection, or -1.0
- `intersectsPlane(normal: Vec3, d: double): bool` — check ray-plane intersection
- `intersectsPlaneRange(normal: Vec3, d: double): double` — returns t at intersection
- `intersectsSphere(center: Vec3, radius: double): bool` — check ray-sphere intersection
- `intersectsSphereRange(center: Vec3, radius: double): double` — returns t at intersection
- `intersectsTriangle(v0: Vec3, v1: Vec3, v2: Vec3): bool` — check ray-triangle intersection
- `intersectsTriangleRange(v0: Vec3, v1: Vec3, v2: Vec3): double` — returns t at intersection

```titrate
let ray = new Ray(new Vec3(0.0, 0.0, -5.0), new Vec3(0.0, 0.0, 1.0));
let box = new AABB(new Vec3(-1.0, -1.0, -1.0), new Vec3(1.0, 1.0, 1.0));
let hit = ray.intersectsAABB(box);        // true
let t = ray.intersectsAABBRange(box);     // 4.0
let pt = ray.pointAt(t);                  // Vec3(0, 0, -1)
```

## Plane

A 3D plane defined by a unit normal and distance from origin (ax + by + cz + d = 0). The normal is automatically normalized on construction.

- `fn init(normal: Vec3, d: double)` — create plane (normal is normalized)
- `distanceToPoint(point: Vec3): double` — signed distance from point to plane
- `side(point: Vec3): int` — 1 (front), -1 (back), 0 (on plane)
- `contains(point: Vec3): bool` — check if point lies on the plane
- `projectPoint(point: Vec3): Vec3` — project point onto the plane
- `closestPoint(point: Vec3): Vec3` — same as projectPoint
- `reflectPoint(point: Vec3): Vec3` — reflect point across the plane
- `reflectDirection(dir: Vec3): Vec3` — reflect direction vector
- `intersectsRay(origin: Vec3, direction: Vec3): bool` — check ray-plane intersection
- `intersectsRayRange(origin: Vec3, direction: Vec3): double` — returns t at intersection
- `intersectsPlane(other: Plane): bool` — check if two planes intersect

**Factory functions:**

- `planeFromPoints(a: Vec3, b: Vec3, c: Vec3): Plane` — plane through three points
- `planeFromNormalPoint(normal: Vec3, point: Vec3): Plane` — plane with given normal through a point

```titrate
let ground = new Plane(new Vec3(0.0, 1.0, 0.0), 0.0);  // y = 0 plane
let dist = ground.distanceToPoint(new Vec3(0.0, 5.0, 0.0));  // 5.0
let reflected = ground.reflectPoint(new Vec3(1.0, 3.0, 0.0)); // Vec3(1, -3, 0)

let p = planeFromPoints(
    new Vec3(0.0, 0.0, 0.0),
    new Vec3(1.0, 0.0, 0.0),
    new Vec3(0.0, 0.0, 1.0)
);
```

## Triangle

A 3D triangle defined by three vertices. Supports area, normal, barycentric coordinates, and ray intersection.

- `fn init(v0: Vec3, v1: Vec3, v2: Vec3)` — create triangle from three vertices
- `area(): double` — area of the triangle
- `normal(): Vec3` — unit normal vector
- `centroid(): Vec3` — centroid (average of vertices)
- `barycentric(point: Vec3): Vec3` — barycentric coordinates (u, v, w)
- `contains(point: Vec3): bool` — check if point lies inside the triangle
- `intersectsRay(origin: Vec3, direction: Vec3): bool` — check ray-triangle intersection
- `intersectsRayRange(origin: Vec3, direction: Vec3): double` — returns t at intersection

```titrate
let tri = new Triangle(
    new Vec3(0.0, 0.0, 0.0),
    new Vec3(1.0, 0.0, 0.0),
    new Vec3(0.0, 1.0, 0.0)
);
let a = tri.area();   // 0.5
let n = tri.normal(); // Vec3(0, 0, 1)
```

## Sphere

A 3D sphere defined by center and radius.

- `fn init(center: Vec3, radius: double)` — create sphere
- `contains(point: Vec3): bool` — check if point is inside
- `containsSphere(other: Sphere): bool` — check if another sphere is fully inside
- `intersects(other: Sphere): bool` — check sphere-sphere intersection
- `intersectsAABB(min: Vec3, max: Vec3): bool` — check sphere-AABB intersection
- `merge(other: Sphere): Sphere` — bounding sphere containing both
- `volume(): double` — volume (4/3 π r³)
- `surfaceArea(): double` — surface area (4 π r²)

```titrate
let s = new Sphere(new Vec3(0.0, 0.0, 0.0), 2.0);
let v = s.volume();        // ≈ 33.51
let inside = s.contains(new Vec3(1.0, 0.0, 0.0));  // true
```

## Frustum

A 6-plane view frustum for frustum culling in 3D rendering.

- `fn init(left: Plane, right: Plane, top: Plane, bottom: Plane, near: Plane, far: Plane)` — create from 6 planes
- `containsPoint(point: Vec3): bool` — check if point is inside
- `containsAABB(box: AABB): bool` — check if AABB is fully inside
- `intersectsAABB(box: AABB): bool` — check if AABB intersects frustum
- `intersectsSphere(center: Vec3, radius: double): bool` — check sphere-frustum intersection

**Factory function:**

- `frustumFromMat4(m: Mat4): Frustum` — extract frustum planes from a view-projection matrix

```titrate
let frustum = frustumFromMat4(viewProjectionMatrix);
let visible = frustum.intersectsAABB(objectBounds);
let pointInside = frustum.containsPoint(new Vec3(0.0, 0.0, 0.0));
```

## Helper Functions

- `aabbCorners(box: AABB): ArrayList<Vec3>` — compute the 8 corner vertices of an AABB

## Deepened AABB

- `AABB.expand(point: ArrayList<double>): void` — expand to include point
- `AABB.intersects(other: AABB): bool` — intersection test
- `AABB.contains(point: ArrayList<double>): bool` — containment test
- `AABB.closestPoint(point: ArrayList<double>): ArrayList<double>` — closest point on AABB
- `AABB.surfaceArea(): double` — surface area
- `AABB.volume(): double` — volume

## Deepened Ray

- `Ray.at(t: double): ArrayList<double>` — point at parameter t
- `Ray.intersectAABB(aabb: AABB): double` — ray-AABB intersection (t or -1)
- `Ray.intersectSphere(center: ArrayList<double>, radius: double): double` — ray-sphere intersection
- `Ray.intersectPlane(plane: Plane): double` — ray-plane intersection
- `Ray.intersectTriangle(v0: ArrayList<double>, v1: ArrayList<double>, v2: ArrayList<double>): double` — Möller-Trumbore

## Deepened Plane

- `Plane.distanceToPoint(point: ArrayList<double>): double` — signed distance
- `Plane.projectPoint(point: ArrayList<double>): ArrayList<double>` — project onto plane
- `Plane.flip(): Plane` — flip normal
- `Plane.transform(matrix: ArrayList<ArrayList<double>>): Plane` — transform by matrix

## Deepened Triangle

- `Triangle.normal(): ArrayList<double>` — face normal
- `Triangle.area(): double` — triangle area
- `Triangle.centroid(): ArrayList<double>` — centroid
- `Triangle.barycentric(point: ArrayList<double>): ArrayList<double>` — barycentric coordinates
- `Triangle.containsPoint(point: ArrayList<double>): bool` — point-in-triangle test

## Deepened Sphere

- `Sphere.contains(point: ArrayList<double>): bool` — containment test
- `Sphere.intersectsSphere(other: Sphere): bool` — sphere-sphere intersection
- `Sphere.intersectsAABB(aabb: AABB): bool` — sphere-AABB intersection
- `Sphere.closestPoint(point: ArrayList<double>): ArrayList<double>` — closest surface point

## Deepened Frustum

- `Frustum.fromMatrix(m: ArrayList<ArrayList<double>>): Frustum` — extract from view-projection matrix
- `Frustum.containsPoint(point: ArrayList<double>): bool` — point-in-frustum test
- `Frustum.intersectsAABB(aabb: AABB): bool` — frustum-AABB test
- `Frustum.intersectsSphere(sphere: Sphere): bool` — frustum-sphere test
