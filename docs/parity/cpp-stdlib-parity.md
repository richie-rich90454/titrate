# C++ Standard Library Parity Matrix

This document maps every C++17/20 standard library header to its Titrate equivalent(s). It is the deliverable for Task F.1 of the world-class-systems-grade-audit spec. Gap closure (Task F.2) is tracked separately; this matrix only documents the current state of `lib/tt/`.

## Status Legend

- ✅ **Full parity** — all major functions/classes have Titrate equivalents
- ⚠️ **Partial** — some major functions/classes are missing
- ❌ **Missing** — no Titrate equivalent exists

## Summary

| Metric | Count |
|--------|-------|
| Total headers enumerated | 97 |
| ✅ Full parity | 42 |
| ⚠️ Partial | 39 |
| ❌ Missing | 16 |

Breakdown by category is shown in each section below. Headers that appear in both a primary category and the C compatibility section (e.g. `<cmath>`, `<ctime>`, `<cerrno>`) are counted once in their primary category only.

## Containers

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<array>` | `tt::util::Array` | ✅ | Fixed-size array backed by `ArrayList`; `get`/`set`/`size`/`fill`/`equals`. |
| `<vector>` | `tt::util::ArrayList`, `tt::util::Vec` | ✅ | Dynamic array with full CRUD, `subList`, `ensureCapacity`, `trimToSize`, `addAll`, `removeAll`, `retainAll`, `sort`. |
| `<deque>` | `tt::util::Deque`, `tt::util::RingDeque` | ✅ | Doubly-ended queue with O(1) push/pop on both ends; `RingDeque` is the circular-buffer backend. |
| `<forward_list>` | `tt::util::ForwardList` | ✅ | Singly-linked list with O(1) `pushFront`/`popFront`/`front` and `insertAfter`. |
| `<list>` | `tt::util::LinkedList` | ✅ | Doubly-linked list with `addFirst`/`addLast`/`removeFirst`/`removeLast` and bidirectional traversal. |
| `<map>` | `tt::util::TreeMap` | ✅ | Red-black tree sorted map with `ceilingKey`/`floorKey`/`firstKey`/`lastKey`/`higherKey`/`lowerKey` and custom comparators. |
| `<set>` | `tt::util::TreeSet` | ✅ | Red-black tree sorted set; `ceiling`/`floor`/`first`/`last`/`higher`/`lower`. |
| `<unordered_map>` | `tt::util::HashMap` | ✅ | Hash map with `put`/`get`/`containsKey`/`keys`/`values`/`entries`/`computeIfAbsent`/`merge`/`getOrDefault`/`replace`/`putIfAbsent`. |
| `<unordered_set>` | `tt::util::HashSet` | ✅ | Hash set backed by `HashMap`. |
| `<stack>` | `tt::util::Stack` | ✅ | LIFO stack: `push`/`pop`/`peek`/`isEmpty`/`size`/`contains`/`toArray`. |
| `<queue>` | `tt::util::Queue`, `tt::util::PriorityQueue` | ✅ | FIFO `Queue` and binary-heap `PriorityQueue` with custom comparators. |
| `<span>` (C++20) | `tt::util::Span` | ⚠️ | `Span<T>` exists but is bound to `ArrayList<T>` rather than raw memory; no `as_bytes`/`subspan` over byte buffers, no fixed-extent `span<T, N>`. |

## Algorithms

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<algorithm>` | `tt::algorithms::Algorithms`, `tt::algo::*`, `tt::heapq::Heapq` | ⚠️ | Has `sort`/`stableSort`/`find`/`findIf`/`binarySearch`/`lowerBound`/`upperBound`/`count`/`countIf`/`transform`/`replace`/`replaceIf`/`remove`/`removeIf`/`unique`/`reverse`/`rotate`/`min`/`max`/`clamp`/`shuffle`. **Missing:** parallel execution policies (`seq`/`par`/`par_unseq`), `nth_element`, `partition_point`, `is_sorted_until`, `inplace_merge`, `stable_partition`, `sample`, `partial_sort`/`partial_sort_copy`. |
| `<numeric>` | `tt::math::Math` (`gcd`/`lcm`/`factorial`/`comb`), `tt::algorithms::Algorithms` (`transform`) | ⚠️ | **Missing:** `accumulate` with custom op (covered indirectly via `Functools.reduce`), `inner_product`, `adjacent_difference`, `partial_sum`, `inclusive_scan`, `exclusive_scan`, `transform_reduce`, `reduce` with execution policy, `midpoint`. |
| `<execution>` (C++17) | — | ❌ | No parallel execution policy types (`seq`/`par`/`par_unseq`/`unsequenced_policy`). |

## Memory

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<memory>` | `tt::copy::Copy` (`shallowCopy`/`deepCopy`), `tt::lang::WeakRef`, `tt::lang::Optional` | ⚠️ | Titrate is garbage-collected, so RAII smart pointers are unnecessary. **Missing:** `unique_ptr`/`shared_ptr`/`weak_ptr` analogs (only `WeakRef` exists, and it is registry-based rather than GC-integrated), `allocator`/`allocator_traits`, `pointer_traits`, `addressof`, `align`, `assume_aligned`. |
| `<memory_resource>` (C++17) | — | ❌ | No polymorphic memory resource (`pmr::polymorphic_allocator`, `synchronized_pool_resource`, `unsynchronized_pool_resource`, `monotonic_buffer_resource`). |
| `<scoped_allocator>` (C++11) | — | ❌ | No `scoped_allocator_adaptor`. |
| `<smart_ptr>` (Boost/TS) | `tt::lang::WeakRef` | ⚠️ | Library was a pre-C++17 TS; smart-pointer semantics merged into `<memory>` in C++17. Titrate only exposes `WeakRef`; no `unique_ptr`/`shared_ptr`/`intrusive_ptr` analogs. |

## Strings

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<string>` | `tt::lang::String`, `tt::lang::StringExt`, `tt::util::StringBuilder`, `tt::string::StringUtils` | ✅ | Static `String` module: `length`/`charAt`/`substring`/`indexOf`/`toUpperCase`/`toLowerCase`/`replace`/`split`/`repeat`/`reverse`/`toCharArray`/`trim`/`startsWith`/`endsWith`/`isEmpty`/`isBlank`. `StringBuilder` for efficient concatenation. |
| `<string_view>` (C++17) | `tt::lang::StringView` | ⚠️ | `StringView` exists with `slice`/`charAt`/`equals`/`startsWith`. **Missing:** non-owning view over arbitrary `char*` buffers (Titrate strings are GC-managed), `remove_suffix`/`remove_prefix`, `find`/`rfind`/`find_first_of`/`find_last_of`/`find_first_not_of`/`find_last_not_of`, `compare`, `substr`. |

## Iterators

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<iterator>` | `tt::lang::Iterator`, `tt::lang::Iterable`, `tt::util::ArrayListIterator`, `tt::util::Range` (`RangeIterator`) | ⚠️ | `Iterator` interface has `next`/`hasNext` plus default `map`/`filter`/`reduce`/`take`/`skip`/`zip`/`chain`/`enumerate`/`forEachRemaining`. **Missing:** iterator categories (`input_iterator_tag`/`forward_iterator_tag`/`bidirectional_iterator_tag`/`random_access_iterator_tag`/`contiguous_iterator_tag`), `reverse_iterator`, `back_inserter`/`front_inserter`/`inserter`, `make_move_iterator`, `distance`/`advance`/`next`/`prev` free functions, `istream_iterator`/`ostream_iterator`. |

## Ranges

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<ranges>` (C++20) | `tt::lang::Iterator` (lazy adaptors), `tt::util::Range`, `tt::itertools::Itertools`, `tt::itertools::ItertoolsSeq` | ⚠️ | Lazy iterator adaptors exist: `map`/`filter`/`take`/`skip`/`zip`/`chain`/`enumerate`. `Range` is a lazy integer sequence. **Missing:** `views::iota`, `views::keys`/`views::values`, `views::elements`, `views::adjacent`/`views::pairwise`, `views::join`/`views::join_with`, `views::split`/`views::lazy_split`, `views::stride`/`views::chunk`/`views::slide`, `views::common`, `views::reverse`, `views::single`/`views::empty`/`views::repeat`, `ranges::sort`/`ranges::copy`/etc. with projection, `ranges::to`, range concepts (`range`/`view`/`sized_range`/`common_range`/`bidirectional_range`/`random_access_range`/`contiguous_range`). |

## Functional

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<functional>` | `tt::functools::Functools`, `tt::operator::Operator`, `tt::lang::Abc` | ⚠️ | `Functools` provides `partial`/`reduce`/`cache`. `Operator` exposes `add`/`sub`/`mul`/`truediv`/`mod`/`neg`/`abs`/`eq`/`ne`/`lt`/`le`/`gt`/`ge`. Titrate has first-class function types `fn(Args): Ret` and arrow closures. **Missing:** `std::function` wrapper type (use `fn(...): ...` directly), `std::bind` with full placeholder support (`_1`/`_2`/`_N`), `std::ref`/`std::cref`/`std::reference_wrapper`, `std::hash`, `std::plus`/`minus`/`multiplies`/`divides`/`modulus`/`negate` functor classes (partially covered by `Operator`), `std::logical_and`/`logical_or`/`logical_not`, `std::mem_fn`, `std::invoke` (call syntax is native), `std::not1`/`not2`. |
| `<bind>` (pre-C++17 TS) | `tt::functools::Functools` (`Partial`) | ⚠️ | Library was a pre-standard TS; merged into `<functional>` in C++11. Titrate `Partial` covers basic partial application. **Missing:** placeholder-based binding (`_1`, `_2`, …), nested bind expressions, `is_bind_expression`/`is_placeholder` traits. |

## Concurrency

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<atomic>` | `tt::concurrent::Atomic` (`AtomicInt`/`AtomicLong`/`AtomicBool`/`AtomicReference`), `tt::concurrent::LockFreeQueue` | ✅ | `get`/`set`/`fetchAdd`/`fetchSub`/`fetchOr`/`fetchAnd`/`fetchXor`/`exchange`/`compareAswap`/`incrementAndGet`/`decrementAndGet`. `LockFreeQueue` for lock-free MPSC. |
| `<thread>` | `tt::concurrent::Thread`, `tt::concurrent::ThreadPoolExecutor`, `tt::concurrent::ThreadPoolExt`, `tt::concurrent::ThreadLocal`, `tt::concurrent::Async` | ⚠️ | `Thread.start`/`join`/`detach`/`getId` exist but execute synchronously due to `Rc<>` not being `Send`-safe in the VM. `ThreadPoolExecutor` provides real concurrency. `Async.async<T>` submits to a global pool. **Missing:** true OS thread parallelism for `Thread`, `thread::id` distinct type, `yield`/`hardware_concurrency` (covered indirectly by `Sys.cpuCount`), `sleep_for`/`sleep_until` (covered by `Time.sleep`), `jthread` (C++20). |
| `<mutex>` | `tt::concurrent::Mutex`, `tt::concurrent::LockGuard`, `tt::concurrent::RecursiveMutex`, `tt::concurrent::OnceFlag` | ✅ | `Mutex.lock`/`unlock`/`tryLock`/`tryLockFor`; `LockGuard` RAII wrapper; `RecursiveMutex`; `OnceFlag.callOnce`. |
| `<shared_mutex>` (C++14) | `tt::concurrent::SharedMutex` | ✅ | Reader-writer lock with `sharedLock`/`sharedUnlock`/`uniqueLock`/`uniqueUnlock`/`trySharedLock`/`tryUniqueLock`. |
| `<future>` | `tt::concurrent::Future`, `tt::concurrent::Promise`, `tt::concurrent::Async` | ✅ | `Future.isDone`/`get`/`cancel`/`isCancelled`; `Promise.complete`/`completeExceptionally`/`future`; `Async.async<T>` returns `Future<T>`. |
| `<condition_variable>` | `tt::concurrent::ConditionVariable` | ✅ | `wait`/`waitFor`/`notifyOne`/`notifyAll`. |
| `<latch>` (C++20) | `tt::concurrent::Latch` | ✅ | One-shot countdown: `countDown`/`wait`/`tryWait`/`count`. |
| `<barrier>` (C++20) | `tt::concurrent::Barrier` | ✅ | Reusable cyclic barrier: `wait`/`arrive`/`arriveAndWait`/`arriveAndDrop`. |
| `<semaphore>` (C++20) | `tt::concurrent::Semaphore` | ✅ | Counting semaphore: `acquire`/`release`/`tryAcquire`/`availablePermits`. |
| `<stop_token>` (C++20) | — | ❌ | No `stop_token`/`stop_source`/`stop_callback`/`jthread` cooperative cancellation. |

## Time

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<chrono>` | `tt::time::Time`, `tt::time::Duration`, `tt::time::Stopwatch`, `tt::time::DateTime`, `tt::time::ZoneInfo` | ⚠️ | `Time.now`/`sleep`/`micros`/`nanos`/`monotonic`/`perfCounter`/`epochSeconds`; `Duration` with `toMillis`/`toSeconds`/`toMinutes`/`toHours`/`toDays`/`toNanos`/`toMicros`/`plus`/`minus`; `Stopwatch` for elapsed-time measurement; `DateTime` calendar arithmetic; `ZoneInfo` time zones. **Missing:** distinct clock types (`system_clock`/`steady_clock`/`high_resolution_clock`/`utc_clock`/`tai_clock`/`gps_clock`/`file_clock`), `time_point`/`duration` template arithmetic with arbitrary `period` (only millisecond resolution stored), `clock_cast`, `is_clock`/`is_clock_v`, `tzdb`/`time_zone` full database, `leap_second`, calendar types `year`/`month`/`day`/`weekday`/`year_month_day`. |
| `<ctime>` | `tt::time::Time` (`now`/`sleep`), `tt::time::DateTime` | ✅ | Wall-clock time, sleep, formatted DateTime. |

## I/O

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<iostream>` | `tt::io::IO` | ✅ | `println`/`print`/`readLine`/`readAll`/`stderr`/`eprintln`/`eprint`/`input`. |
| `<fstream>` | `tt::io::File`, `tt::io::FileReader`, `tt::io::FileWriter`, `tt::io::BufferedReader`, `tt::io::AsyncFile`, `tt::io::Mmap`, `tt::io::FileLock`, `tt::io::FileWatcher`, `tt::io::Tempfile` | ✅ | Synchronous and asynchronous file I/O, buffered reading, memory mapping, file locking, recursive directory watching, temporary files. |
| `<sstream>` | `tt::io::StringReader`, `tt::io::StringWriter`, `tt::io::BytesIO`, `tt::io::Pipe` | ✅ | In-memory string/byte streams and pipe I/O. |
| `<syncstream>` (C++20) | — | ❌ | No `osyncstream` synchronized output stream buffer. |
| `<iomanip>` | `tt::io::Format` | ⚠️ | `Format.format` provides `printf`-style specifiers (`%d`/`%f`/`%s`/`%b`/`%x`/`%o` with width/precision/flags). **Missing:** stream manipulators (`std::setw`/`setprecision`/`setfill`/`hex`/`dec`/`oct`/`fixed`/`scientific`/`boolalpha`/`noboolalpha`/`showpoint`/`noshowpoint`/`setbase`/`put_money`/`get_money`/`put_time`/`get_time`/`quoted`). |
| `<ios>` | `tt::io::Reader`, `tt::io::Writer` | ⚠️ | `Reader` and `Writer` interfaces exist. **Missing:** `std::ios_base` full class hierarchy, stream state (`good`/`eof`/`fail`/`bad`), formatting flags (`fmtflags`/`iostate`/`openmode`/`seekdir`), `std::basic_ios`, `std::streambuf` integration, `std::ios`/`wios` typedefs. |
| `<streambuf>` | — | ❌ | No raw `std::basic_streambuf` abstraction. Buffered I/O is encapsulated in `BufferedReader`/`FileReader`. |

## Numeric

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<cmath>` | `tt::math::Math`, `tt::math::MathAdvanced`, `tt::math::MathTrig` | ✅ | `Math`: `abs`/`fabs`/`floor`/`ceil`/`round`/`min`/`max`/`comb`/`factorial`/`erf`/`gamma`/`lgamma`/`gcd`/`lcm`/`random` + constants (`PI`/`E`/`tau`/`INF`/`NAN`). `MathAdvanced`: `sqrt`/`pow`/`exp`/`ln`/`log2`/`log10`/`cbrt`/`hypot`/`fma`/`log1p`/`expm1`. `MathTrig`: `sin`/`cos`/`tan`/`asin`/`acos`/`atan`/`atan2`/`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`. |
| `<complex>` | `tt::math::complex::Complex` | ✅ | `abs`/`arg`/`norm`/`conj`/`add`/`sub`/`mul`/`div`/`exp`/`log`/`pow`/`sqrt`/`sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`/`polar`. |
| `<valarray>` | `tt::math::ndarray::NDArray` (and `NDArrayMath`/`NDArrayReduce`/`NDArraySlice`) | ⚠️ | `NDArray<T>` is more general (N-dim) and covers `valarray`'s elementwise math, slicing, and reductions. **Missing:** direct `valarray`-style slice/`gslice` API, `indirect_array`/`mask_array`/`slice_array` proxy assignment, transcendental overloads for `valarray`. |
| `<random>` | `tt::random::Random`, `tt::random::ContinuousDist`, `tt::random::DiscreteDist`, `tt::random::Prng`, `tt::random::QuasiRandom`, `tt::random::Sampling` | ✅ | Xorshift128+ engine; uniform int/long/float/double; Box-Muller Gaussian; continuous (normal/exponential/gamma/beta/weibull/lognormal) and discrete (Bernoulli/binomial/Poisson/geometric) distributions; quasi-random (Sobol/Halton); sampling (reservoir/stratified/systematic). |
| `<ratio>` (C++11) | — | ❌ | No compile-time rational arithmetic (`std::ratio`/`ratio_add`/`ratio_subtract`/`ratio_multiply`/`ratio_divide`/`ratio_equal`/`ratio_less`/SI aliases). |
| `<numbers>` (C++20) | `tt::math::Math` (`PI`/`E`/`tau`) | ⚠️ | **Missing:** `inv_pi`/`inv_sqrtpi`/`ln2`/`ln10`/`log2e`/`log10e`/`sqrt2`/`sqrt3`/`inv_sqrt3`/`egamma`/`phi` as compile-time constants. |

## Type Support

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<type_traits>` (C++11) | — | ❌ | Titrate has no compile-time metaprogramming. The runtime `is` and `as` operators cover dynamic type checks. **Missing:** `is_integral`/`is_floating_point`/`is_arithmetic`/`is_pointer`/`is_reference`/`is_class`/`is_enum`/`is_union`/`is_same`/`is_base_of`/`is_convertible`/`is_constructible`/`is_assignable`/`is_trivially_*`/`add_const`/`remove_const`/`add_pointer`/`remove_pointer`/`decay`/`conditional`/`enable_if`/`void_t`, etc. |
| `<typeindex>` (C++11) | — | ❌ | No `std::type_index` for use as a hash key. `Variant._typeTag` provides a string-based analog. |
| `<typeinfo>` | `tt::lang` (`is`/`as` operators), `tt::lang::Variant` (`typeTag`/`hasTag`) | ⚠️ | Runtime type info via `is`/`as` and `Variant.typeTag()`. **Missing:** `std::type_info` class, `typeid` operator returning a comparable handle, `bad_cast`/`bad_typeid` exceptions. |
| `<any>` (C++17) | `tt::lang::Variant` | ✅ | `Variant` is the standard dynamic type; `Variant.get`/`getOrElse`/`hasTag`/`typeTag`. |
| `<optional>` (C++17) | `tt::lang::Optional`, `tt::lang::OptionalExt` | ✅ | `Optional.of`/`empty`/`isPresent`/`isEmpty`/`get`/`orElse`/`orElseGet`/`or`/`map`/`flatMap`/`filter`/`ifPresent`. Plus nullable `null` literal as a native alternative. |
| `<variant>` (C++17) | `tt::lang::Variant`, `tt::lang::VariantExt` | ⚠️ | `Variant` is dynamically typed (string-tagged). `VariantExt.visit`/`holdsAlternative`/`getVariant`/`getIf` exist. **Missing:** compile-time-checked `variant<Ts...>` with type-safe `visit` and valueless-by-exception state, `std::get<I>`/`std::get<T>`/`std::holds_alternative<T>` with compile-time type list, `std::monostate`. |
| `<tuple>` (C++11) | `tt::lang::Tuple`, `tt::lang::TupleExt`, `tt::util::Pair` | ✅ | `Tuple2`/`Tuple3`/`Tuple4` typed wrappers; `Pair<F,S>`; tuple destructuring via `let (a,b) = pair`. |
| `<utility>` | `tt::util::Pair`, `tt::util::Range`, `tt::copy::Copy` (`move` semantically via `shallowCopy`), `tt::operator::Operator`, `tt::lang::Integer`/`Long`/`Double` (`compare`/`max`/`min`) | ✅ | `Pair`/`makePair`/`swap`; `Range`; integer comparison utilities. **Missing:** `std::move`/`std::forward` (Titrate handles moves implicitly via GC), `std::swap` free function (covered by `Pair.swap` and collection-level swaps), `std::exchange`, `std::declval`, `std::in_place`/`std::piecewise_construct_t`. |
| `<format>` (C++20) | `tt::io::Format` | ⚠️ | `printf`-style `format(template, args)` with width/precision/flags. **Missing:** `std::format` format-string syntax (`{}`/`{0}`/`{:.2f}`/`{:>10}`), `std::format_to`/`std::format_to_n`/`std::formatted_size`, `std::format_error`, `std::formatter` specialization, compile-time format string checking, `std::vformat`/`std::basic_format_string`/`std::format_args`. |

## Error Handling

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<exception>` | `tt::lang::Traceback` (`Frame`/`format`/`extract`), language `throw`/`try`/`catch`/`?` operator | ✅ | Built-in `throw`/`try`/`catch (e: string)`; error propagation `?`; `Traceback.Frame` captures function/file/line. |
| `<stdexcept>` | `throw "string"` | ⚠️ | Exceptions are string-typed (`throw "IndexOutOfBounds: …"`). **Missing:** exception class hierarchy (`std::exception`/`logic_error`/`runtime_error`/`domain_error`/`invalid_argument`/`length_error`/`out_of_range`/`range_error`/`overflow_error`/`underflow_error`), `what()` virtual method, nested exceptions (`std::nested_exception`/`throw_with_nested`/`rethrow_if_nested`). |
| `<system_error>` (C++11) | `tt::lang::ErrorCode`, `tt::lang::DataFile` (`lang/error_codes.json`) | ✅ | `ErrorCode(value, category, message)`; loaded error code tables; `equals`/`toString`. |
| `<cerrno>` | `tt::sys::Signal` (loaded from `sys/signals.json`) | ⚠️ | **Missing:** `errno` macro and standard `E*` constants (`EAGAIN`/`EINVAL`/`ENOMEM`/`EACCES`/…), `strerror`/`perror`. Only signal numbers are loaded. |

## Localization

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<locale>` | `tt::i18n::Locale`, `tt::lang::DataFile` (`locale/cldr.json`), `tt::text::Unicodedata` | ⚠️ | `Locale` with `decimalPoint`/`thousandsSep`/`currencySymbol`/`dateFormat`/`timeFormat` loaded from CLDR; Unicode database for character properties. **Missing:** full `std::locale` facet system (`std::ctype`/`num_put`/`num_get`/`time_put`/`time_get`/`money_put`/`money_get`/`messages`/`collate`/`codecvt` facets), `std::locale::global`/`classic`/`combine`, `std::use_facet`. |
| `<codecvt>` (C++11, deprecated C++17, removed C++20) | `tt::encoding::Codecs`, `tt::encoding::Base64`/`Hex`/`Url` | ⚠️ | `Codecs.encode`/`decode` support utf-8/ascii/latin-1 plus aliases loaded from data. **Missing:** `std::codecvt_utf8`/`codecvt_utf16`/`codecvt_utf8_utf16`/`wstring_convert`/`wbuffer_convert` (header itself removed in C++20; functionality partially covered by `Codecs`). |

## Filesystem

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<filesystem>` (C++17, boost-origin) | `tt::file::Path`, `tt::file::Directory`, `tt::file::FileUtils`, `tt::file::Glob`, `tt::file::Fnmatch` | ✅ | `Path.join`/`basename`/`dirname`/`extension`/`exists`/`isFile`/`isDir`/`absolutePath`/`canonicalPath`/`resolve`/`parent`/`relativePath`; `Directory.list`/`walk`/`walkWithDepth`/`walkWithMaxDepth`/`walkWithPrune`/`create`/`remove`; `FileUtils` copy/move/touch; `Glob`/`Fnmatch` pattern matching. |

## Regex

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<regex>` | `tt::regex::Regex`, `tt::regex::Match`, `tt::regex::RegexIterator`, `tt::regex::RegexExt` | ✅ | `Regex.match`/`find`/`findAll`/`matches`/`split`/`replace`/`replaceFirst`; flags `IGNORECASE`/`MULTILINE`/`DOTALL`/`VERBOSE`; capture groups via `Match`; `RegexIterator` for streaming matches. |

## Concepts

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<concepts>` (C++20) | — | ❌ | Titrate has no concepts language feature. Generic functions use unchecked type parameters. **Missing:** `same_as`/`derived_from`/`convertible_to`/`common_with`/`integral`/`signed_integral`/`unsigned_integral`/`floating_point`/`assignable_from`/`swappable`/`destructible`/`constructible_from`/`default_initializable`/`move_constructible`/`copy_constructible`/`regular`/`semiregular`/`equality_comparable`/`totally_ordered`/`movable`/`copyable`, iterator concepts (`input_iterator`/`forward_iterator`/`bidirectional_iterator`/`random_access_iterator`/`contiguous_iterator`), range concepts (see `<ranges>`). |

## Coroutines

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<coroutine>` (C++20) | — | ❌ | Titrate has no coroutine language feature. `tt::concurrent::Async.async<T>` provides thread-pool-based asynchronous execution; `tt::concurrent::Channel` provides CSP-style message passing. **Missing:** `std::coroutine_handle`/`std::coroutine_traits`/`std::suspend_always`/`std::suspend_never`, `co_await`/`co_yield`/`co_return` keywords, promise-type customization. |

## Charconv

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<charconv>` (C++17) | `tt::lang::Integer`, `tt::lang::IntegerExt`, `tt::lang::Long`, `tt::lang::LongExt`, `tt::lang::Double`, `tt::lang::DoubleExt` | ⚠️ | `Integer.parseInt`/`toString`/`parseOr`/`parseIntWithRadix`/`parseUnsignedInt`/`toUnsignedString`/`toHexString`/`toBinaryString`/`toOctalString`/`bitCount`/`rotateLeft`/`rotateRight`/`highestOneBit`/`lowestOneBit`/`signum`/`clampInt`; `Double.parseDouble`/`toString`/`isNaN`/`isInfinite`/`isFinite`/`doubleToLongBits`/`longBitsToDouble`/`toHexString`. **Missing:** `std::to_chars`/`std::from_chars` with `chars_format` (`scientific`/`fixed`/`hex`/`general`) and `chars_result` (`ptr`/`ec`), zero-allocation round-trip guarantees, `std::chars_format`. |

## Bit

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<bit>` (C++20) | `tt::math::Bit` | ✅ | `popcount`/`countlZero`/`countrZero`/`rotl`/`rotr`/`hasSingleBit`/`bitWidth`/`bitFloor`/`bitCeil`. **Missing:** `byteswap` (C++23), `countl_one`/`countr_one` (minor). |

## Source Location

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<source_location>` (C++20) | — | ❌ | No `std::source_location` (`file_name`/`line`/`column`/`function_name`). `tt::lang::Traceback.Frame` captures function/file/line at runtime, but there is no current-call-site intrinsic. |

## Compare

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<compare>` (C++20) | built-in `<`/`<=`/`>`/`>=`/`==`/`!=`, `tt::lang::Integer.compare`, `tt::lang::Long.compare` | ⚠️ | Comparison operators are native. **Missing:** three-way comparison `operator<=>` (spaceship), `std::partial_ordering`/`std::weak_ordering`/`std::strong_ordering` category types, `std::is_eq`/`is_neq`/`is_lt`/`is_lteq`/`is_gt`/`is_gteq`, `std::common_comparison_category`. |

## Version

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<version>` (C++20) | — | ❌ | No language/library version macros (`__cpp_lib_*`). Titrate does not expose feature-test macros. |

## C Compatibility Headers

Headers suffixed with their C `<name>.h` counterpart. `<cmath>`, `<ctime>`, and `<cerrno>` are documented above in their primary categories and listed here for completeness.

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<cassert>` | `tt::assert::Assert` | ✅ | `assertTrue`/`assertFalse`/`assertEqual`/`assertNotEqual`/`assertNull`/`assertNotNull`/`assertThrows` (replaces `assert` macro). |
| `<cctype>` | `tt::lang::Character` | ✅ | `isDigit`/`isLetter`/`isWhitespace`/`isUpperCase`/`isLowerCase`/`toUpperCase`/`toLowerCase`/`getNumericValue`/`isAlphabetic`/`isISOControl`. |
| `<cerrno>` | `tt::sys::Signal` (loaded from `sys/signals.json`) | ⚠️ | See Error Handling. **Missing:** `errno` macro and standard `E*` constants. |
| `<cfloat>` | `tt::lang::NumericLimits`, `tt::lang::Double` (`MAX_VALUE`/`MIN_VALUE`/`MIN_NORMAL`/`EPSILON`) | ✅ | Float limit constants loaded from `lang/numeric_limits.json`. |
| `<ciso646>` (deprecated C++17, removed C++20) | (no-op) | ✅ | C++ defines `and`/`or`/`not`/`xor`/`bitand`/`bitor`/`compl` as keywords natively; header is a no-op in C++17 and removed in C++20. |
| `<climits>` | `tt::lang::NumericLimits`, `tt::lang::Integer` (`MAX_VALUE`/`MIN_VALUE`), `tt::lang::Long` (`MAX_VALUE`/`MIN_VALUE`) | ✅ | Integer limit constants. |
| `<clocale>` | `tt::i18n::Locale` | ⚠️ | Basic locale support (`decimalPoint`/`thousandsSep`/`currencySymbol`/`dateFormat`/`timeFormat`). **Missing:** `setlocale`/`LC_*` category macros, `lconv` struct, `localeconv`. |
| `<cmath>` | `tt::math::Math`, `tt::math::MathAdvanced`, `tt::math::MathTrig` | ✅ | See Numeric. |
| `<csetjmp>` | — | ❌ | No `setjmp`/`longjmp`. Titrate uses structured exception handling (`throw`/`try`/`catch`) for non-local control flow. |
| `<csignal>` | `tt::sys::Signal` | ⚠️ | Standard signal numbers loaded from data (`SIGHUP`/`SIGINT`/`SIGTERM`/`SIGKILL`/…). **Missing:** `signal()` handler installation, `raise()`, `sig_atomic_t`. |
| `<cstdarg>` | — | ❌ | No varargs (`va_list`/`va_start`/`va_arg`/`va_end`). Titrate uses `ArrayList<Variant>` or `Variant[]` for variadic functions. |
| `<cstddef>` | language primitives (`size`/`ptrdiff` types implicit) | ⚠️ | `size` is a primitive type for sizes; `null` is the null pointer literal. **Missing:** `offsetof` macro, `max_align_t`. |
| `<cstdio>` | `tt::io::IO` (`println`/`print`/`readLine`/`readAll`), `tt::io::File`, `tt::io::Format` | ✅ | Higher-level I/O replaces `printf`/`scanf`/`fopen`/`fclose`/`fread`/`fwrite`/`fgets`/`fputs`/`fprintf`/`fscanf`. |
| `<cstdlib>` | `tt::sys::Sys` (`exit`/`env`/`setEnv`/`args`), `tt::lang::Integer` (`parseInt`/`toString`/`MAX_VALUE`/`MIN_VALUE`), `tt::random::Random` (`init`/`nextInt`/`nextLong`/`nextDouble`), `tt::math::Math` (`abs`/`min`/`max`) | ⚠️ | **Missing:** `malloc`/`calloc`/`realloc`/`free` (GC manages memory), `qsort`/`bsearch` (covered by `Algorithms.sort`/`Algorithms.binarySearch`), `atof`/`atoi`/`atol`/`strtol`/`strtoul`/`strtod` (covered by `Integer.parse`/`Double.parse`), `abort`/`atexit` (covered by `tt::sys::Atexit`), `getenv` (covered by `Sys.env`). |
| `<cstring>` | `tt::lang::String` (`length`/`charAt`/`substring`/`indexOf`/`concat`/`equals`/`compareTo`), `tt::util::StringBuilder` | ✅ | String operations replace `strlen`/`strcpy`/`strcat`/`strcmp`/`strncmp`/`strchr`/`strrchr`/`strstr`/`strtok`/`memcpy`/`memmove`/`memset`/`memcmp`. |
| `<ctime>` | `tt::time::Time` (`now`/`sleep`/`millis`), `tt::time::DateTime` | ✅ | See Time. |
| `<cuchar>` (C++11) | `tt::lang::Character`, `tt::text::Unicodedata` | ⚠️ | Titrate `string` is Unicode (UTF-8 backed by VM); `Character` provides char classification. **Missing:** `char16_t`/`char32_t` distinct types, `mbrtoc16`/`mbrtoc32`/`c16rtomb`/`c32rtomb` conversion functions. |
| `<cwchar>` | `tt::lang::String` (Unicode-aware), `tt::lang::Character` | ⚠️ | Titrate `string` handles wide characters natively. **Missing:** `wchar_t` distinct type, `wcslen`/`wcscpy`/`wcscat`/`wcscmp`/`wcsncmp`/`wcschr`/`wcsrchr`/`wcsstr`/`wcstok`/`wprintf`/`wscanf`/`fgetwc`/`fputwc`/`getwc`/`putwc`/`getwchar`/`putwchar`/`ungetwc`/`wcstof`/`wcstol`/`wcstoul`. |
| `<cwctype>` | `tt::lang::Character` | ⚠️ | `iswalnum`/`iswalpha`/`iswcntrl`/`iswdigit`/`iswgraph`/`iswlower`/`iswprint`/`iswpunct`/`iswspace`/`iswupper`/`iswxdigit`/`towlower`/`towupper` partially covered. **Missing:** `wctype_t`/`wctrans_t` types, `wctype`/`wctrans` runtime category lookup, full Unicode category coverage. |

### Deprecated/Removed C Compatibility Headers

These were deprecated in C++17 and removed in C++20 but are listed for completeness.

| C++ Header | Titrate Module(s) | Status | Notes |
|------------|-------------------|--------|-------|
| `<ccomplex>` (deprecated C++17, removed C++20) | `tt::math::complex::Complex` | ✅ | Header was a no-op including `<complex>`; covered by `tt::math::complex::Complex`. |
| `<cstdalign>` (deprecated C++17, removed C++20) | (no-op) | ✅ | Header was a no-op (`alignof`/`alignas` are native keywords); Titrate does not expose alignment control. |
| `<cstdbool>` (deprecated C++17, removed C++20) | (no-op) | ✅ | Header was a no-op (`bool`/`true`/`false` are native keywords). |
| `<ctgmath>` (deprecated C++17, removed C++20) | `tt::math::Math`, `tt::math::MathAdvanced`, `tt::math::MathTrig`, `tt::math::complex::Complex` | ✅ | Header was a no-op including `<complex>` and `<cmath>`; covered by the math modules above. |

---

## Major Gaps

The following 16 ❌ entries represent the most significant parity gaps:

1. **`<execution>`** — No parallel execution policy framework. Affects `<algorithm>` parallel overloads.
2. **`<memory_resource>`** — No polymorphic memory resource system (pmr).
3. **`<scoped_allocator>`** — No scoped allocator adaptor.
4. **`<stop_token>`** — No cooperative thread cancellation (C++20 `jthread`).
5. **`<syncstream>`** — No synchronized output stream (C++20 `osyncstream`).
6. **`<streambuf>`** — No raw stream buffer abstraction.
7. **`<ratio>`** — No compile-time rational arithmetic.
8. **`<type_traits>`** — No compile-time type introspection; fundamental gap for metaprogramming.
9. **`<typeindex>`** — No `type_index` for use as a hash key.
10. **`<concepts>`** — No C++20 concepts language feature; fundamental gap for constrained generics.
11. **`<coroutine>`** — No C++20 coroutines; `Async` is a thread-pool workaround.
12. **`<source_location>`** — No call-site intrinsic; runtime `Traceback.Frame` is the only fallback.
13. **`<version>`** — No feature-test macros.
14. **`<csetjmp>`** — No `setjmp`/`longjmp`; exceptions are the only non-local control flow.
15. **`<cstdarg>`** — No C-style varargs; `Variant[]` is the alternative.

The 33 ⚠️ entries mostly fall into these themes:

- **Compile-time vs. runtime** — Titrate lacks `constexpr` evaluation, template metaprogramming, and concepts, so `<type_traits>`/`<concepts>`/`<ratio>` style compile-time machinery has no equivalent.
- **Smart pointers / allocators** — Titrate is garbage-collected, so RAII smart-pointer and allocator APIs (`<memory>`/`<memory_resource>`/`<scoped_allocator>`/`<smart_ptr>`) are partially or fully absent.
- **Stream buffer hierarchy** — Titrate exposes high-level `Reader`/`Writer` interfaces but not the full `std::ios_base`/`std::basic_streambuf`/`std::basic_iostream` class hierarchy (`<ios>`/`<streambuf>`/`<syncstream>`).
- **Format strings** — Titrate uses `printf`-style formatting rather than `std::format` and `std::iomanip` manipulators (`<format>`/`<iomanip>`).
- **Three-way comparison** — No `operator<=>` or ordering category types (`<compare>`).
- **Locale facets** — `tt::i18n::Locale` covers basic formatting but not the full `std::locale` facet system (`<locale>`).
- **C low-level APIs** — `errno`, `setjmp`/`longjmp`, varargs, and signal handler installation are missing or stubbed (`<cerrno>`/`<csetjmp>`/`<csignal>`/`<cstdarg>`).

These gaps are intentionally documented here; closure is tracked by Task F.2 of the world-class-systems-grade-audit spec.
