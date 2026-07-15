# special

The `tt.math.special` module provides special mathematical functions including Bessel functions, orthogonal polynomials, hypergeometric functions, elliptic integrals, and more.

```titrate
import tt.math.special.Special;
```

## Bessel Functions

- `Special.besselJ(n: int, x: double): double` — Bessel function of the first kind
- `Special.besselY(n: int, x: double): double` — Bessel function of the second kind
- `Special.besselI(n: int, x: double): double` — modified Bessel function of the first kind
- `Special.besselK(n: int, x: double): double` — modified Bessel function of the second kind
- `Special.besselI0(x: double): double` — I₀(x)
- `Special.besselI1(x: double): double` — I₁(x)
- `Special.besselK0(x: double): double` — K₀(x)
- `Special.besselK1(x: double): double` — K₁(x)
- `Special.sphericalBesselJ(n: int, x: double): double` — spherical Bessel jₙ(x)
- `Special.sphericalBesselY(n: int, x: double): double` — spherical Bessel yₙ(x)

## Orthogonal Polynomials

- `Special.legendreP(n: int, x: double): double` — Legendre polynomial Pₙ(x)
- `Special.hermiteH(n: int, x: double): double` — physicist's Hermite polynomial
- `Special.hermiteHe(n: int, x: double): double` — probabilist's Hermite polynomial
- `Special.laguerreL(n: int, x: double): double` — Laguerre polynomial
- `Special.chebyshevT(n: int, x: double): double` — Chebyshev polynomial of the first kind
- `Special.chebyshevU(n: int, x: double): double` — Chebyshev polynomial of the second kind
- `Special.legendreRoots(n: int): ArrayList<double>` — roots of Legendre polynomial
- `Special.gaussLegendreWeights(n: int): ArrayList<double>` — Gauss-Legendre quadrature weights

## Hypergeometric Functions

- `Special.hypergeometric0F1(a: double, x: double): double` — ₀F₁
- `Special.hypergeometric1F1(a: double, b: double, x: double): double` — ₁F₁ (Kummer's)
- `Special.hypergeometric2F1(a: double, b: double, c: double, x: double): double` — ₂F₁ (Gauss)

## Elliptic Integrals

- `Special.ellipticK(m: double): double` — complete elliptic integral of the first kind
- `Special.ellipticE(m: double): double` — complete elliptic integral of the second kind
- `Special.ellipticPi(n: double, m: double): double` — complete elliptic integral of the third kind
- `Special.ellipticF(phi: double, m: double): double` — incomplete elliptic integral of the first kind
- `Special.ellipticEIncomplete(phi: double, m: double): double` — incomplete elliptic integral of the second kind
- `Special.ellipticPiIncomplete(n: double, phi: double, m: double): double` — incomplete elliptic integral of the third kind
- `Special.carlsonRF(x: double, y: double, z: double): double` — Carlson symmetric form RF
- `Special.carlsonRJ(x: double, y: double, z: double, p: double): double` — Carlson symmetric form RJ
- `Special.carlsonRC(x: double, y: double): double` — Carlson symmetric form RC

## Gamma/Beta/Zeta

- `Special.gamma(x: double): double` — gamma function
- `Special.logGamma(x: double): double` — log-gamma function
- `Special.beta(a: double, b: double): double` — beta function
- `Special.logBeta(a: double, b: double): double` — log-beta function
- `Special.digamma(x: double): double` — digamma function ψ(x)
- `Special.polygamma(n: int, x: double): double` — polygamma function
- `Special.zeta(s: double): double` — Riemann zeta function
- `Special.dirichletEta(s: double): double` — Dirichlet eta function
- `Special.incompleteGamma(a: double, x: double): double` — lower incomplete gamma
- `Special.incompleteGammaUpper(a: double, x: double): double` — upper incomplete gamma

## Airy/Fresnel

- `Special.airyAi(x: double): double` — Airy function Ai(x)
- `Special.airyBi(x: double): double` — Airy function Bi(x)
- `Special.fresnelS(x: double): double` — Fresnel integral S(x)
- `Special.fresnelC(x: double): double` — Fresnel integral C(x)
- `Special.dawson(x: double): double` — Dawson function
- `Special.exponentialIntegral(x: double): double` — exponential integral E₁(x)

```titrate
let j0 = Special.besselJ(0, 2.5);     // Bessel J₀(2.5)
let p3 = Special.legendreP(3, 0.5);    // Legendre P₃(0.5)
let k = Special.ellipticK(0.5);        // complete elliptic K(0.5)
let g = Special.gamma(5.0);            // Γ(5) = 24
```

## Character types and wide-character utilities (C `<wctype>`/`<uchar>` parity, Phase 1-2)

This section documents the wide-character and multi-byte conversion utilities added for full C parity.

### char16_t / char32_t

`char16_t` and `char32_t` are 16-bit and 32-bit character types holding a single UTF-16 / UTF-32 code unit. They are represented as `u16` and `u32` integer aliases.

- `Char16` — wrapper for a 16-bit code unit (UTF-16)
- `Char32` — wrapper for a 32-bit code point (UTF-32)
- `Char16.init(value: u16)`, `Char16.value(): u16`
- `Char32.init(value: u32)`, `Char32.value(): u32`

### Multi-byte conversion

These functions convert between multi-byte (UTF-8) sequences and `char16_t` / `char32_t` code units, mirroring `mbrtoc16` / `mbrtoc32` / `c16rtomb` / `c32rtomb`.

- `mbrtoc16(s: string, state: MbState): (size_t, char16_t)` — convert a UTF-8 sequence to one `char16_t` (returns bytes consumed)
- `mbrtoc32(s: string, state: MbState): (size_t, char32_t)` — convert a UTF-8 sequence to one `char32_t`
- `c16rtomb(c: char16_t, state: MbState): string` — convert one `char16_t` to a UTF-8 multi-byte sequence
- `c32rtomb(c: char32_t, state: MbState): string` — convert one `char32_t` to a UTF-8 multi-byte sequence

```titrate
let s: string = "héllo";
let state = new MbState();
let (n, c16) = mbrtoc16(s, state);  // c16 holds the first UTF-16 unit
let back: string = c16rtomb(c16, new MbState());
```

### wctype_t / wctrans_t

- `wctype_t` — opaque handle to a wide-character classification category (e.g. `alpha`, `digit`, `space`)
- `wctrans_t` — opaque handle to a wide-character mapping (e.g. `tolower`, `toupper`)
- `wctype(name: string): wctype_t` — look up a classification category by name
- `iswctype(wc: char, desc: wctype_t): bool` — classify a wide character
- `wctrans(name: string): wctrans_t` — look up a mapping by name
- `towctrans(wc: char, desc: wctrans_t): char` — apply the mapping to a wide character

```titrate
let alphaDesc: wctype_t = wctype("alpha");
io::println(Boolean.toString(iswctype('a', alphaDesc)));  // true

let upperDesc: wctrans_t = wctrans("toupper");
io::println(Integer.toString(towctrans('a', upperDesc)));  // 'A'
```
