# geom

The `tt.geom` module provides computational geometry algorithms including convex hulls, Delaunay triangulation, polygon operations, spline curves, and spatial indexing structures.

```titrate
import tt.geom.ConvexHull;
import tt.geom.Delaunay;
import tt.geom.Polygon;
import tt.geom.Spline;
import tt.geom.SpatialIndex;
```

## ConvexHull

Convex hull computation for 2D and 3D point sets using Graham scan and QuickHull algorithms.

- `fn init()` — create an empty convex hull builder

**2D hull (Graham scan):**
- `compute2D(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<double>>` — compute 2D convex hull via Graham scan, returns hull vertices in counter-clockwise order
- `grahamScan(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<double>>` — alias for `compute2D`

**2D/3D hull (QuickHull):**
- `quickHull2D(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<double>>` — 2D convex hull via QuickHull
- `quickHull3D(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<double>>` — 3D convex hull via QuickHull, returns triangular faces

**Hull properties:**
- `area2D(hull: ArrayList<ArrayList<double>>): double` — area of 2D convex hull (shoelace formula)
- `perimeter2D(hull: ArrayList<ArrayList<double>>): double` — perimeter of 2D convex hull
- `volume3D(hull: ArrayList<ArrayList<double>>): double` — volume of 3D convex hull
- `surfaceArea3D(hull: ArrayList<ArrayList<double>>): double` — surface area of 3D convex hull
- `centroid2D(hull: ArrayList<ArrayList<double>>): ArrayList<double>` — centroid of 2D hull
- `isConvex2D(polygon: ArrayList<ArrayList<double>>): bool` — test if a polygon is convex

```titrate
let hull = new ConvexHull();
let points = new ArrayList<ArrayList<double>>();
let p1: ArrayList<double> = new ArrayList<double>(); p1.add(0.0); p1.add(0.0);
let p2: ArrayList<double> = new ArrayList<double>(); p2.add(4.0); p2.add(0.0);
let p3: ArrayList<double> = new ArrayList<double>(); p3.add(2.0); p3.add(3.0);
let p4: ArrayList<double> = new ArrayList<double>(); p4.add(1.0); p4.add(1.0);
points.add(p1); points.add(p2); points.add(p3); points.add(p4);

let result = hull.compute2D(points);
let a = ConvexHull.area2D(result);
let p = ConvexHull.perimeter2D(result);
io::println("Hull area: " + Double.toString(a));
io::println("Hull perimeter: " + Double.toString(p));
```

## Delaunay

Delaunay triangulation and Voronoi diagram computation using the Bowyer-Watson algorithm.

- `fn init()` — create an empty Delaunay triangulator

**Triangulation:**
- `triangulate(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<int>>` — compute Delaunay triangulation via Bowyer-Watson, returns list of triangles (each triangle is 3 vertex indices)
- `insertPoint(point: ArrayList<double>): void` — incrementally insert a point into existing triangulation

**Voronoi diagram:**
- `voronoi(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<double>>` — compute Voronoi diagram as the dual of the Delaunay triangulation, returns Voronoi vertices
- `voronoiEdges(points: ArrayList<ArrayList<double>>): ArrayList<ArrayList<int>>` — Voronoi edges as pairs of vertex indices

**Circumcircles:**
- `circumcircle(triangle: ArrayList<ArrayList<double>>): (double, double, double)` — circumcircle center (cx, cy) and radius of a triangle
- `inCircumcircle(point: ArrayList<double>, triangle: ArrayList<ArrayList<double>>): bool` — test if a point lies inside a triangle's circumcircle

**Properties:**
- `numTriangles(): int` — number of triangles in current triangulation
- `numVertices(): int` — number of vertices
- `getTriangles(): ArrayList<ArrayList<int>>` — current triangle list

```titrate
let del = new Delaunay();
let points = new ArrayList<ArrayList<double>>();
let a: ArrayList<double> = new ArrayList<double>(); a.add(0.0); a.add(0.0);
let b: ArrayList<double> = new ArrayList<double>(); b.add(1.0); b.add(0.0);
let c: ArrayList<double> = new ArrayList<double>(); c.add(0.5); c.add(1.0);
let d: ArrayList<double> = new ArrayList<double>(); d.add(0.3); d.add(0.4);
points.add(a); points.add(b); points.add(c); points.add(d);

let triangles = del.triangulate(points);
let voronoiVerts = del.voronoi(points);
let (cx, cy, r) = del.circumcircle(triangles.get(0));
```

## Polygon

Simple polygon operations including area, centroid, convexity, triangulation, point-in-polygon test, and offset.

- `fn init(vertices: ArrayList<ArrayList<double>>)` — create polygon from ordered vertices (2D)

**Geometric properties:**
- `area(): double` — area via shoelace formula (positive for CCW, negative for CW)
- `signedArea(): double` — signed area
- `centroid(): ArrayList<double>` — centroid (cx, cy)
- `perimeter(): double` — perimeter length

**Tests:**
- `isConvex(): bool` — test if polygon is convex
- `containsPoint(point: ArrayList<double>): bool` — point-in-polygon test (ray casting)
- `isSimple(): bool` — test if polygon is simple (no self-intersections)
- `orientation(): int` — +1 for CCW, -1 for CW, 0 for degenerate

**Decomposition:**
- `triangulate(): ArrayList<ArrayList<int>>` — ear-clipping triangulation, returns triangle vertex indices
- `convexPartition(): ArrayList<Polygon>` — partition into convex sub-polygons

**Transformation:**
- `offset(distance: double): Polygon` — offset (inflate/deflate) polygon by distance (Minkowski sum with circle)
- `reverse(): Polygon` — reverse vertex order (flip orientation)
- `boundingBox(): (double, double, double, double)` — axis-aligned bounding box (minX, minY, maxX, maxY)

**Access:**
- `vertices(): ArrayList<ArrayList<double>>` — vertex list
- `numVertices(): int` — vertex count
- `getVertex(i: int): ArrayList<double>` — vertex by index

```titrate
let verts = new ArrayList<ArrayList<double>>();
let v1: ArrayList<double> = new ArrayList<double>(); v1.add(0.0); v1.add(0.0);
let v2: ArrayList<double> = new ArrayList<double>(); v2.add(4.0); v2.add(0.0);
let v3: ArrayList<double> = new ArrayList<double>(); v3.add(4.0); v3.add(3.0);
let v4: ArrayList<double> = new ArrayList<double>(); v4.add(0.0); v4.add(3.0);
verts.add(v1); verts.add(v2); verts.add(v3); verts.add(v4);

let poly = new Polygon(verts);
io::println("Area: " + Double.toString(poly.area()));         // 12.0
io::println("Convex: " + Boolean.toString(poly.isConvex()));  // true

let testPt: ArrayList<double> = new ArrayList<double>(); testPt.add(2.0); testPt.add(1.5);
io::println("Contains: " + Boolean.toString(poly.containsPoint(testPt)));  // true

let inflated = poly.offset(1.0);
let triangles = poly.triangulate();
```

## Spline

Parametric curve classes: Bézier curves (quadratic, cubic, general), B-splines, and NURBS with evaluation, derivatives, and arc length computation.

- `fn init()` — base spline constructor

**Bézier curves:**
- `Spline.bezierQuad(p0: ArrayList<double>, p1: ArrayList<double>, p2: ArrayList<double>): Spline` — quadratic Bézier curve
- `Spline.bezierCubic(p0: ArrayList<double>, p1: ArrayList<double>, p2: ArrayList<double>, p3: ArrayList<double>): Spline` — cubic Bézier curve
- `Spline.bezier(controlPoints: ArrayList<ArrayList<double>>): Spline` — general Bézier curve of arbitrary degree (de Casteljau algorithm)

**B-splines:**
- `Spline.bspline(controlPoints: ArrayList<ArrayList<double>>, degree: int): Spline` — B-spline with uniform knot vector
- `Spline.bsplineWithKnots(controlPoints: ArrayList<ArrayList<double>>, degree: int, knots: ArrayList<double>): Spline` — B-spline with custom knot vector

**NURBS:**
- `Spline.nurbs(controlPoints: ArrayList<ArrayList<double>>, weights: ArrayList<double>, degree: int, knots: ArrayList<double>): Spline` — Non-Uniform Rational B-Spline

**Evaluation:**
- `evaluate(t: double): ArrayList<double>` — point on curve at parameter t ∈ [0, 1]
- `derivative(t: double): ArrayList<double>` — first derivative at parameter t
- `secondDerivative(t: double): ArrayList<double>` — second derivative at parameter t
- `tangent(t: double): ArrayList<double>` — unit tangent vector at parameter t
- `normal(t: double): ArrayList<double>` — unit normal vector at parameter t (2D only)
- `curvature(t: double): double` — curvature at parameter t

**Arc length:**
- `length(): double` — approximate arc length (adaptive Gaussian quadrature)
- `lengthRange(tStart: double, tEnd: double): double` — arc length over parameter range
- `parameterAtLength(s: double): double` — inverse: find parameter t for given arc length s

**Control points:**
- `controlPoints(): ArrayList<ArrayList<double>>` — control point list
- `setControlPoint(index: int, point: ArrayList<double>): void` — modify a control point
- `degree(): int` — curve degree

```titrate
let p0: ArrayList<double> = new ArrayList<double>(); p0.add(0.0); p0.add(0.0);
let p1: ArrayList<double> = new ArrayList<double>(); p1.add(1.0); p1.add(2.0);
let p2: ArrayList<double> = new ArrayList<double>(); p2.add(3.0); p2.add(2.0);
let p3: ArrayList<double> = new ArrayList<double>(); p3.add(4.0); p3.add(0.0);

let curve = Spline.bezierCubic(p0, p1, p2, p3);
let mid = curve.evaluate(0.5);
let tang = curve.tangent(0.5);
let len = curve.arcLength();
let deriv = curve.derivative(0.5);
io::println("Midpoint: (" + Double.toString(mid.get(0)) + ", " + Double.toString(mid.get(1)) + ")");
io::println("Arc length: " + Double.toString(len));
```

## SpatialIndex

Spatial indexing data structures for efficient geometric queries: KD-Tree, R-Tree, and bounding volume hierarchy (BVH).

- `fn init()` — base spatial index constructor

**KD-Tree:**
- `SpatialIndex.kdtree(points: ArrayList<ArrayList<double>>): SpatialIndex` — build a KD-Tree from points
- `nearestNeighbor(query: ArrayList<double>): ArrayList<double>` — find closest point
- `nearestNeighborIndex(query: ArrayList<double>): int` — index of closest point
- `kNearestNeighbors(query: ArrayList<double>, k: int): ArrayList<ArrayList<double>>` — k closest points
- `rangeQuery(min: ArrayList<double>, max: ArrayList<double>): ArrayList<ArrayList<double>>` — all points within axis-aligned bounding box
- `radiusQuery(center: ArrayList<double>, radius: double): ArrayList<ArrayList<double>>` — all points within radius

**R-Tree:**
- `SpatialIndex.rtree(): SpatialIndex` — create an empty R-Tree
- `insert(min: ArrayList<double>, max: ArrayList<double>, id: int): void` — insert a bounding rectangle with identifier
- `search(min: ArrayList<double>, max: ArrayList<double>): ArrayList<int>` — find all entries intersecting the query rectangle
- `remove(min: ArrayList<double>, max: ArrayList<double>, id: int): bool` — remove an entry

**Bounding Volume Hierarchy (BVH):**
- `SpatialIndex.bvh(): SpatialIndex` — create an empty BVH
- `insertAABB(min: ArrayList<double>, max: ArrayList<double>, id: int): void` — insert an axis-aligned bounding box
- `intersectRay(origin: ArrayList<double>, direction: ArrayList<double>): ArrayList<int>` — find all AABBs intersected by a ray
- `intersectSegment(start: ArrayList<double>, end: ArrayList<double>): ArrayList<int>` — find all AABBs intersected by a line segment
- `build(): void` — optimize the BVH structure (call after all insertions)

**Common:**
- `size(): int` — number of indexed elements
- `depth(): int` — tree depth

```titrate
let points = new ArrayList<ArrayList<double>>();
for (let i = 0; i < 1000; i++) {
    let pt: ArrayList<double> = new ArrayList<double>();
    pt.add(Math.random() * 100.0);
    pt.add(Math.random() * 100.0);
    points.add(pt);
}

let tree = SpatialIndex.kdtree(points);
let query: ArrayList<double> = new ArrayList<double>(); query.add(50.0); query.add(50.0);
let nearest = tree.nearestNeighbor(query);
let neighbors = tree.kNearestNeighbors(query, 5);
let nearby = tree.radiusQuery(query, 10.0);
```
