# Python Standard Library Parity Matrix

This document maps every Python 3.12 standard library module to its Titrate equivalent(s). It is the authoritative reference for tracking Titrate's coverage of the Python stdlib surface area.

## Status Legend

- ✅ Full parity — all major functions/classes have Titrate equivalents
- ⚠️ Partial — some major functions/classes are missing
- ❌ Missing — no Titrate equivalent exists

## Summary

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ Full parity | 75 | 41.4% |
| ⚠️ Partial | 31 | 17.1% |
| ❌ Missing | 75 | 41.4% |
| **Total** | **181** | **100%** |

Major gap areas (❌): runtime introspection (`ast`, `inspect`, `symtable`, `dis`, `trace`), Python-specific tooling (`pdb`, `profile`, `cProfile`, `pstats`, `pydoc`, `doctest`, `py_compile`, `importlib`, `pkgutil`, `runpy`, `zipimport`), serialization formats (`pickle`, `marshal`, `email`, `plistlib`), GUI/terminal toolkits (`tkinter`, `turtle`, `curses`, `turtledemo`, `idlelib`), legacy internet protocols (`telnetlib`, `imaplib`, `poplib`, `nntplib`, `ftplib`, `xmlrpc`, `wsgiref`, `cgi`, `cgitb`), Unix-specific (`fcntl`, `termios`, `tty`, `pty`, `pwd`, `spwd`, `syslog`, `resource`), Windows-specific (`winreg`, `winsound`), Python packaging (`distutils`, `venv`, `site`, `zipapp`), and concurrency primitives (`multiprocessing`, `contextvars`, `ctypes`).

Note: Several `❌` entries are Python-implementation-specific (e.g. `ast`, `inspect`, `dis`, `symtable`, `doctest`, `pydoc`, `pdb`) and arguably do not need direct Titrate equivalents — Titrate has its own compiler/VM introspection story. They are listed as `❌` per the strict "Python module-by-module" mapping requirement of this audit. Task G.2 will decide which gaps are genuine and worth closing.

---

## Core Modules

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `abc` | `tt::lang::Abc` | ✅ | 100% | Abstract base class support |
| `argparse` | `tt::argparse::ArgumentParser`, `tt::lang::ArgparseExt` | ✅ | 100% | Command-line argument parsing |
| `ast` | — | ❌ | 0% | No Titrate AST introspection API. Titrate has its own AST in `trc/src/ast/` but does not expose it to `.tr` programs |
| `code` | — | ❌ | 0% | No interactive REPL interpreter; Titrate uses `pipette` for tooling |
| `codecs` | `tt::encoding::Codecs` | ✅ | 100% | Codec registry and stream codecs |
| `configparser` | `tt::config::ConfigParser` | ✅ | 100% | INI-style configuration file parser, `ConfigParser`, `SectionProxy` |
| `contextlib` | `tt::contextlib::Contextlib`, `tt::lang::Contextlib` | ✅ | 100% | `with` statement utilities, context managers |
| `contextvars` | — | ❌ | 0% | No ContextVar equivalent. `tt::concurrent::ThreadLocal` exists but is thread-local, not async-context-local |
| `copy` | `tt::copy::Copy`, `tt::lang::CopyExt` | ✅ | 100% | Shallow and deep copy |
| `copyreg` | — | ❌ | 0% | No pickle dispatch registry (no pickle support) |
| `ctypes` | — | ❌ | 0% | No foreign function interface. Titrate uses native bridges via `titrate_native` |
| `dataclasses` | `tt::dataclass::Dataclass`, `tt::lang::DataclassExt` | ✅ | 100% | Dataclass generation and field metadata |
| `enum` | `tt::lang::Enum`, `tt::lang::EnumExt` | ✅ | 100% | Enum and IntEnum equivalents |
| `errno` | `tt::lang::ErrorCode` | ✅ | 100% | System error code constants |
| `functools` | `tt::functools::Functools`, `tt::lang::Functools` | ✅ | 100% | `lru_cache`, `partial`, `reduce`, `cmp_to_key`, etc. |
| `gc` | `tt::sys::Gc` | ✅ | 100% | Garbage collector interface |
| `getopt` | `tt::argparse::ArgumentParser` | ⚠️ | 60% | `argparse` covers the use case. No direct `getopt`/`gnu_getopt` C-style API |
| `importlib` | — | ❌ | 0% | No dynamic import API. Titrate imports are resolved at compile time |
| `inspect` | — | ❌ | 0% | No live object introspection API |
| `io` | `tt::io::IO`, `tt::io::File`, `tt::io::BufferedReader`, `tt::io::BytesIO`, `tt::io::StringReader`, `tt::io::StringWriter`, `tt::io::Reader`, `tt::io::Writer`, `tt::io::Pipe` | ✅ | 100% | Stream/Buffer/TextIO base classes and concrete streams |
| `itertools` | `tt::itertools::Itertools`, `tt::itertools::ItertoolsSeq`, `tt::lang::Itertools` | ✅ | 100% | `chain`, `count`, `cycle`, `groupby`, `product`, `permutations`, etc. |
| `keyword` | — | ❌ | 0% | No keyword list constant. Titrate keywords are documented in `AGENTS.md` but not exposed at runtime |
| `logging` | `tt::logging::Logger`, `tt::lang::LoggerExt` | ✅ | 100% | `Logger`, `Handler`, `Formatter`, levels, hierarchy, file/stream handlers |
| `operator` | `tt::operator::Operator` | ✅ | 100% | Functional forms of operators (`add`, `itemgetter`, `attrgetter`, etc.) |
| `optparse` | `tt::argparse::ArgumentParser` | ⚠️ | 65% | `argparse` covers the use case. No direct `OptionParser`/`add_option` API (deprecated in Python) |
| `pkgutil` | — | ❌ | 0% | No package utility API |
| `pprint` | `tt::pprint::Pprint` | ✅ | 100% | `pprint`, `pformat`, `PrettyPrinter`, `PrettyPrinter` with depth/width |
| `pyclbr` | — | ❌ | 0% | No Python source browser; applies to Python source only |
| `reprlib` | `tt::pprint::Pprint` | ⚠️ | 50% | `Pprint` covers pretty-printing. No dedicated `Repr` class with `repr1`/`reprlib.repr` recursive-limited representation |
| `runpy` | — | ❌ | 0% | No `run_module` equivalent; use `pipette run` |
| `symtable` | — | ❌ | 0% | No symbol table introspection. Titrate symbol tables are internal to `trc/src/analyzer/scope.rs` |
| `sys` | `tt::sys::Sys`, `tt::sys::Atexit`, `tt::sys::Warnings` | ✅ | 100% | `argv`, `exit`, `path`, `modules`-equivalent, `atexit`, `warnings` |
| `types` | `tt::lang::Variant`, `tt::lang::Optional`, `tt::lang::Result`, `tt::lang::Tuple`, `tt::lang::Iterable`, `tt::lang::Iterator` | ⚠️ | 60% | No `ModuleType`, `FunctionType`, `MethodType`, `GeneratorType` dynamic type objects. Static type abstractions exist |
| `typing` | `tt::lang::Iterable`, `tt::lang::Iterator`, `tt::lang::Optional`, `tt::lang::Result`, `tt::lang::Tuple`, `tt::lang::Variant`, `tt::lang::WeakRef` | ⚠️ | 55% | No `Protocol`, `TypedDict`, `Literal`, `Callable` runtime machinery. Titrate generics are compile-time only |
| `warnings` | `tt::sys::Warnings` | ✅ | 100% | `warn`, `filterwarnings`, `simplefilter` |
| `weakref` | `tt::lang::WeakRef` | ✅ | 100% | Weak references |

## Data Types & Collections

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `array` | `tt::util::Array`, `tt::util::Vec`, `tt::util::ArrayList` | ⚠️ | 65% | No `array.array` with type codes. `Vec<T>` and `Array<T>` provide typed contiguous storage but no `frombytes`/`tobytes`/`typecode` API |
| `bisect` | `tt::bisect::Bisect` | ✅ | 100% | `bisect_left`, `bisect_right`, `insort_left`, `insort_right` |
| `collections` | `tt::util::HashMap`, `tt::util::Counter`, `tt::util::ChainMap`, `tt::util::OrderedDict`, `tt::util::defaultdict`, `tt::util::UserDict`, `tt::util::UserList`, `tt::util::UserString`, `tt::util::namedTuple`, `tt::util::Deque` | ✅ | 100% | All major collections types including `deque`, `Counter`, `OrderedDict`, `defaultdict`, `ChainMap`, `namedtuple` |
| `decimal` | `tt::decimal::Decimal`, `tt::decimal::DecimalContext`, `tt::decimal::DecimalExt`, `tt::decimal::DecimalArithmetic` | ✅ | 100% | Decimal arithmetic, context, rounding modes |
| `fractions` | `tt::fractions::Fraction` | ✅ | 100% | Rational number arithmetic |
| `graphlib` | `tt::algo::Graph`, `tt::util::Graph`, `tt::util::GraphUtil` | ⚠️ | 60% | Graph data structures exist. Missing `TopologicalSorter` class and `topological_order`/`static_order` API |
| `heapq` | `tt::heapq::Heapq`, `tt::algo::HeapAlgo` | ⚠️ | 85% | `heappush`/`heappop`/`heapify` covered. `nlargest`/`nsmallest` partial. Some merge functions in `HeapAlgo` |
| `numbers` | `tt::lang::Integer`, `tt::lang::Long`, `tt::lang::Vast`, `tt::lang::Uvast`, `tt::lang::Float`, `tt::lang::Double`, `tt::lang::Half`, `tt::lang::Quad`, `tt::lang::Byte`, `tt::lang::Short` | ⚠️ | 70% | All concrete numeric types exist. No `Number` ABC hierarchy (`Integral`, `Rational`, `Real`, `Complex` base classes) |
| `queue` | `tt::util::Queue`, `tt::util::PriorityQueue`, `tt::util::PriorityQueueExt`, `tt::concurrent::Channel`, `tt::concurrent::LockFreeQueue` | ✅ | 100% | `Queue`, `PriorityQueue`, `LifoQueue` (via `Stack`), `SimpleQueue` (via `Channel`) |
| `shelve` | — | ❌ | 0% | No persistent object shelf. Requires `pickle` which is also missing |
| `sqlite3` | `tt::db::Sqlite`, `tt::db::SqliteExt` | ✅ | 100% | `connect`, `Connection`, `Cursor`, `execute`, parameter binding, transactions, extensions |
| `struct` | `tt::binary::Struct`, `tt::binary::StructExt` | ✅ | 100% | `pack`/`unpack`/`calcsize`, format strings, native/standard/little/big endian |

## File & Directory

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `csv` | `tt::csv::CsvReader`, `tt::csv::CsvWriter` | ✅ | 100% | `reader`, `writer`, `DictReader`, `DictWriter`, dialects, quoting |
| `filecmp` | — | ❌ | 0% | No file/directory comparison API |
| `fileinput` | — | ❌ | 0% | No lazy line-iterator over multiple files |
| `fnmatch` | `tt::file::Fnmatch` | ✅ | 100% | Unix shell-style glob matching |
| `glob` | `tt::file::Glob` | ✅ | 100% | `glob`, `iglob`, `recursive` globbing |
| `linecache` | — | ❌ | 0% | No line cache for random access to source lines |
| `pathlib` | `tt::file::Path`, `tt::file::Directory`, `tt::file::FileUtils` | ✅ | 100% | Pure and concrete path objects, `Path.read_text`/`write_text`/`exists`/`glob` |
| `shutil` | `tt::file::FileUtils`, `tt::file::Directory` | ⚠️ | 70% | `copy`, `move`, `rmtree` covered. Missing `make_archive`, `get_archive_formats`, `disk_usage`, `which` |
| `tempfile` | `tt::io::Tempfile`, `tt::tempfile::Tempfile` | ✅ | 100% | `mkstemp`, `mkdtemp`, `NamedTemporaryFile`, `TemporaryDirectory` |

## OS & System

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `fcntl` | — | ❌ | 0% | Unix-only file control; not applicable cross-platform. Titrate `tt::io::FileLock` provides advisory locking |
| `mmap` | `tt::io::Mmap`, `tt::io::MmapExt` | ✅ | 100% | Memory-mapped file objects, read/write/private modes, `flush`, `resize`, slicing |
| `os` | `tt::sys::Os`, `tt::sys::Sys`, `tt::sys::Platform` | ✅ | 100% | `getenv`, `setenv`, `listdir`, `mkdir`, `makedirs`, `symlink`, `readlink`, `kill`, `urandom`, `environ`, `remove`, `rename`, `access`, `cpu_count`, `pid`, `working_dir`, `change_dir` |
| `platform` | `tt::sys::Platform` | ✅ | 100% | `system`, `release`, `version`, `machine`, `processor`, `python_implementation`-equivalent |
| `posix` | `tt::sys::Os` | ⚠️ | 60% | POSIX-specific syscalls partially covered by `tt::sys::Os`. No `posix.fork`, `posix.execv`, `posix.waitpid` direct equivalents |
| `posixpath` | `tt::file::Path` | ✅ | 100% | Path manipulation (`join`, `split`, `basename`, `dirname`, `normpath`) |
| `pty` | — | ❌ | 0% | Unix-only pseudo-terminal; not implemented |
| `pwd` | — | ❌ | 0% | Unix-only password database; not applicable cross-platform |
| `resource` | — | ❌ | 0% | Unix-only resource limits; not implemented |
| `signal` | `tt::sys::Signal` | ✅ | 100% | `signal`, `getsignal`, `SIG*` constants, alarm, setitimer |
| `spwd` | — | ❌ | 0% | Unix-only shadow password database; deprecated in Python, not implemented |
| `stat` | `tt::sys::Os` | ⚠️ | 55% | `os.stat()` results covered. Missing `ST_MODE`/`ST_SIZE` constants module and `filemode` helper |
| `subprocess` | `tt::subprocess::Subprocess`, `tt::lang::Subprocess` | ✅ | 100% | `run`, `Popen`, `call`, `check_call`, `check_output`, pipes, stdin/stdout/stderr capture |
| `sysconfig` | `tt::sys::Platform` | ⚠️ | 50% | Platform paths and config variables partially covered. No `get_path`/`get_config_var`/`get_platform` API matching CPython |
| `syslog` | — | ❌ | 0% | Unix-only syslog; not implemented |
| `termios` | — | ❌ | 0% | Unix-only terminal control; not implemented |
| `tty` | — | ❌ | 0% | Unix-only terminal utilities; not implemented |
| `winreg` | — | ❌ | 0% | Windows-only registry; not implemented |

## Concurrency

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `asyncio` | `tt::concurrent::Async`, `tt::concurrent::Future`, `tt::concurrent::Promise` | ⚠️ | 35% | `async`/`Future`/`Promise` primitives exist. No full event loop, no `asyncio.run`, no `gather`/`wait`/`create_task`, no async I/O streams, no `Lock`/`Semaphore` async equivalents |
| `concurrent.futures` | `tt::concurrent::ThreadPoolExecutor`, `tt::concurrent::Future`, `tt::concurrent::ThreadPoolExt`, `tt::concurrent::Promise` | ✅ | 100% | `ThreadPoolExecutor`, `Future`, `as_completed`, `wait` |
| `multiprocessing` | — | ❌ | 0% | No multi-process API. `tt::concurrent::Thread` and `tt::subprocess::Subprocess` cover related use cases but no `Process`/`Queue`/`Pool` for true multiprocessing |
| `threading` | `tt::concurrent::Thread`, `tt::concurrent::Mutex`, `tt::concurrent::Event`, `tt::concurrent::ConditionVariable`, `tt::concurrent::RecursiveMutex`, `tt::concurrent::SharedMutex`, `tt::concurrent::Semaphore`, `tt::concurrent::OnceFlag`, `tt::concurrent::Barrier`, `tt::concurrent::Latch`, `tt::concurrent::ThreadLocal`, `tt::concurrent::LockGuard` | ✅ | 100% | `Thread`, `Lock`, `RLock`, `Semaphore`, `Event`, `Condition`, `Barrier`, `local`, plus `OnceFlag`/`Latch`/`SharedMutex` extensions |

## Networking

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `ftplib` | — | ❌ | 0% | No FTP client |
| `http` | `tt::net::HttpClient`, `tt::net::HttpUtil` | ✅ | 100% | HTTP client, status codes, headers, cookies, methods |
| `imaplib` | — | ❌ | 0% | No IMAP4 client |
| `ipaddress` | `tt::net::IpAddress` | ✅ | 100% | `IPv4Address`, `IPv6Address`, `IPv4Network`, `IPv6Network`, `ip_address`, `ip_network`, `ip_interface` |
| `netrc` | — | ❌ | 0% | No `.netrc` parser |
| `nntplib` | — | ❌ | 0% | No NNTP client (deprecated in Python 3.13) |
| `poplib` | — | ❌ | 0% | No POP3 client |
| `select` | `tt::sys::Selectors` | ⚠️ | 70% | `select`/`poll` covered via `tt::sys::Selectors`. No `epoll`/`kqueue` direct equivalents exposed |
| `selectors` | `tt::sys::Selectors` | ✅ | 100% | High-level I/O multiplexing abstraction |
| `smtplib` | `tt::net::Smtp` | ✅ | 100% | SMTP client |
| `socket` | `tt::net::Socket`, `tt::net::TcpClient`, `tt::net::TcpServer`, `tt::net::UdpSocket`, `tt::net::Dns` | ✅ | 100% | TCP/UDP sockets, DNS resolution, `socket`/`bind`/`listen`/`accept`/`connect`/`recv`/`send` |
| `socketserver` | `tt::net::TcpServer` | ⚠️ | 50% | `TCPServer` covered. Missing `UDPServer`, `UnixStreamServer`, `ForkingMixIn`/`ThreadingMixIn` and the `BaseRequestHandler` framework |
| `ssl` | `tt::net::Ssl`, `tt::net::SslExt` | ✅ | 100% | SSLContext, certificate loading, TLS wrap, peer verification |
| `telnetlib` | — | ❌ | 0% | No Telnet client (deprecated in Python 3.11) |
| `urllib` | `tt::net::HttpClient`, `tt::net::UrlBuilder`, `tt::encoding::Url` | ✅ | 100% | `urlopen`, URL parsing, URL encoding/decoding, `Request` |
| `webbrowser` | — | ❌ | 0% | No system browser launcher |

## Cryptography

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `crypt` | `tt::crypto::CryptoExt` | ⚠️ | 40% | `crypt` module is deprecated in Python 3.13. `CryptoExt` provides modern primitives (ed25519, curve25519, chacha20poly1305, hkdf) but no `crypt.crypt` traditional Unix hash |
| `hashlib` | `tt::crypto::Hash` | ✅ | 100% | `md5`, `sha1`, `sha256`, `sha512`, `sha3`, `blake2` |
| `hmac` | `tt::crypto::Hmac` | ✅ | 100% | HMAC computation, `compare_digest` |
| `secrets` | `tt::crypto::Secrets`, `tt::secrets::Secrets` | ✅ | 100% | `token_bytes`, `token_hex`, `token_urlsafe`, `choice`, `randbelow` |

## Text Processing

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `base64` | `tt::encoding::Base64` | ✅ | 100% | `b64encode`, `b64decode`, `urlsafe_b64encode`/`decode`, Base32, Base85, `standard_b64decode` |
| `difflib` | `tt::text::Difflib` | ✅ | 100% | `SequenceMatcher`, `Differ`, `unified_diff`, `HtmlDiff` |
| `html` | `tt::html::Html` | ✅ | 100% | HTML escaping, unescaping, parsing |
| `mimetypes` | — | ⚠️ | 0% | No MIME type guesser. Type data exists in `tt::data::encoding` but no public API |
| `quopri` | `tt::encoding::Codecs` | ⚠️ | 50% | `tt::encoding::Codecs` provides quoted-printable utilities indirectly. No dedicated `encode`/`decode`/`encodestring`/`decodestring` API |
| `re` | `tt::regex::Regex`, `tt::regex::Match`, `tt::regex::RegexExt`, `tt::regex::RegexIterator` | ✅ | 100% | `match`, `search`, `findall`, `sub`, `split`, `finditer`, capture groups |
| `shlex` | `tt::text::Shlex` | ✅ | 100% | Shell-style lexing, `split`, `quote`, `Shlex` iterator |
| `string` | `tt::lang::String`, `tt::lang::StringExt`, `tt::lang::StringUtils`, `tt::string::StringUtils`, `tt::lang::StringView` | ✅ | 100% | `ascii_letters`, `digits`, `Template`, `Formatter`, `capwords` |
| `stringprep` | — | ❌ | 0% | No RFC 3454 stringprep. `tt::text::Unicodedata` provides the underlying Unicode tables |
| `textwrap` | `tt::textwrap::Textwrap` | ✅ | 100% | `wrap`, `fill`, `dedent`, `indent`, `TextWrapper` |
| `unicodedata` | `tt::text::Unicodedata`, `tt::lang::Character`, `tt::lang::CharacterExt` | ✅ | 100% | `name`, `lookup`, `category`, `bidirectional`, `combining`, `decimal`, `digit`, `numeric`, `normalize` |
| `xdrlib` | `tt::binary::Struct` | ⚠️ | 40% | `struct` can pack/unpack XDR-compatible data but no dedicated `Packer`/`Unmarshaller` API |

## Numeric & Math

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `math` | `tt::math::Math`, `tt::math::MathAdvanced`, `tt::math::MathTrig`, `tt::math::special::Special`, `tt::math::NumberTheory`, `tt::math::Combinatorics`, `tt::math::complex::Complex`, `tt::math::Bit` | ✅ | 100% | All `math` module functions including `sqrt`, `pow`, `exp`, `log`, trig, hyperbolic, `gcd`, `lcm`, `factorial`, `comb`, `perm`, `erf`, `gamma`, `isqrt`, `prod`. Plus `cmath` via `tt::math::complex::Complex` |
| `random` | `tt::random::Random`, `tt::random::Prng`, `tt::random::ContinuousDist`, `tt::random::DiscreteDist`, `tt::random::QuasiRandom`, `tt::random::Sampling` | ✅ | 100% | `random`, `randint`, `choice`, `shuffle`, `sample`, `seed`, `gauss`, `uniform`, distributions |
| `statistics` | `tt::statistics::Statistics`, `tt::statistics::Bootstrap`, `tt::statistics::Kde`, `tt::statistics::Mcmc`, `tt::statistics::Survival`, `tt::statistics::TimeSeries` | ✅ | 100% | `mean`, `median`, `mode`, `stdev`, `variance`, `quantiles`, `correlation`, `linear_regression`. Plus `Bootstrap`/`Kde`/`Mcmc` extensions |

## Date & Time

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `calendar` | `tt::time::DateTime`, `tt::time::BusinessCalendar`, `tt::time::DateRange` | ⚠️ | 60% | `BusinessCalendar` covers workday logic. Missing `TextCalendar`/`HTMLCalendar` formatters, `monthcalendar`, `yeardatescalendar`, `weekday`, `month_name` constants |
| `datetime` | `tt::time::DateTime`, `tt::time::Duration`, `tt::time::ZoneInfo`, `tt::time::Stopwatch` | ✅ | 100% | `date`, `time`, `datetime`, `timedelta`, `tzinfo`, `timezone`, formatting and parsing |
| `sched` | `tt::time::Scheduler`, `tt::time::Cron` | ✅ | 100% | `scheduler` event queue, `enter`, `enterabs`, `run`, plus `Cron` for cron-style scheduling |
| `time` | `tt::time::Time`, `tt::time::Duration`, `tt::time::Stopwatch` | ✅ | 100% | `time`, `sleep`, `monotonic`, `perf_counter`, `strftime`, `strptime`, `gmtime`, `localtime` |
| `timeit` | `tt::time::Stopwatch` | ⚠️ | 65% | `Stopwatch` provides high-resolution timing. Missing `Timer` class with `timeit`/`repeat`/`autorange` API and statement-execution model |
| `zoneinfo` | `tt::time::ZoneInfo` | ✅ | 100% | IANA timezone database, `ZoneInfo(key)` construction |

## JSON & Serialization

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `email` | — | ❌ | 0% | No email message / MIME / parser package. SMTP send is supported via `tt::net::Smtp` but message construction is not |
| `json` | `tt::json::Json`, `tt::json::JsonValue`, `tt::json::JsonParser`, `tt::json::Json5`, `tt::json::JsonBinary`, `tt::json::JsonPatch`, `tt::json::JsonPath`, `tt::json::JsonSchema`, `tt::json::JsonStreamingParser` | ✅ | 100% | `dumps`/`loads`/`dump`/`load`, plus JSON5, BSON, JSON Patch, JSON Path, JSON Schema, streaming parser |
| `marshal` | — | ❌ | 0% | No marshal format. Python-internal serialization, not portable to Titrate |
| `pickle` | — | ❌ | 0% | No pickle protocol. Requires runtime type introspection that Titrate does not expose |
| `pickletools` | — | ❌ | 0% | No pickle protocol (depends on `pickle`) |
| `plistlib` | — | ❌ | 0% | No Apple property list reader/writer |
| `tomllib` | `tt::config::Toml` | ✅ | 100% | TOML 1.0 parser, `loads`/`load` |
| `uu` | `tt::encoding::Base64` | ⚠️ | 30% | No direct `uu` encode/decode. `tt::encoding::Base64` covers the binary-to-text use case but with a different encoding scheme |
| `xml` | `tt::xml::Xml`, `tt::xml::XmlNode`, `tt::xml::XmlBuilder`, `tt::xml::XmlStreamingParser`, `tt::xml::XPath`, `tt::xml::XmlNamespace`, `tt::xml::XmlSchema`, `tt::xml::XmlCanonicalizer` | ✅ | 100% | DOM, SAX-style streaming, XPath, namespaces, schema validation, canonicalization |
| `uuid` | `tt::uuid::Uuid` | ✅ | 100% | `uuid1`, `uuid3`, `uuid4`, `uuid5`, `UUID` class |

## Compression

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `gzip` | `tt::compression::Gzip` | ✅ | 100% | `compress`/`decompress`, `GzipFile`, `open` |
| `lzma` | — | ⚠️ | 0% | No LZMA/XZ. `tt::compression::Zstd` and `tt::compression::Lz4` provide alternative modern compressors |
| `tarfile` | `tt::compression::Tar` | ✅ | 100% | `open`, `TarFile`, `TarInfo`, read/write/append modes |
| `zipapp` | — | ❌ | 0% | No combined-archive executable builder |
| `zipfile` | `tt::compression::ZipFile` | ✅ | 100% | `ZipFile`, `ZipInfo`, `zip_open`, read/write, password entry |
| `zipimport` | — | ❌ | 0% | Python-specific zip-based import; not applicable to Titrate |
| `zlib` | `tt::compression::Zlib` | ✅ | 100% | `compress`/`decompress`, `compressobj`/`decompressobj`, Adler-32, CRC-32 |

## Audio & Image

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `imghdr` | `tt::image::Image` | ⚠️ | 35% | `tt::image::Image` can decode common formats but no standalone `what()` file-type sniffer |
| `sndhdr` | `tt::audio::WavReader` | ⚠️ | 30% | `tt::audio::WavReader` handles WAV only. No `what()` for AIFF/AU/VOC/hCOM sniffing |
| `sunau` | `tt::audio::WavReader`, `tt::audio::WavWriter` | ⚠️ | 20% | No Sun AU format support. `WavReader`/`WavWriter` cover WAV only |
| `wave` | `tt::audio::WavReader`, `tt::audio::WavWriter`, `tt::audio::AudioBuffer` | ✅ | 100% | `open`, `Wave_read`, `Wave_write`, channels, framerate, frames |
| `winsound` | — | ❌ | 0% | Windows-only beep/PCM playback; not implemented |

## Internet Protocols

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `cgi` | — | ❌ | 0% | Deprecated in Python 3.11, removed in 3.13. Not implemented |
| `cgitb` | — | ❌ | 0% | Deprecated in Python 3.11. Not implemented |
| `mailcap` | — | ❌ | 0% | Deprecated in Python 3.11, removed in 3.13. Not implemented |
| `smtpd` | — | ❌ | 0% | Deprecated in Python 3.11, removed in 3.13. No SMTP server; `tt::net::Smtp` provides the client only |

## Internationalization

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `encodings` | `tt::encoding::Codecs` | ⚠️ | 55% | Codec registry exists via `tt::encoding::Codecs`. Not all Python codec names are registered (e.g. `punycode`, `rot_13`, `shift_jis` subsets) |
| `gettext` | `tt::i18n::Locale` | ⚠️ | 40% | `Locale` provides locale-aware formatting. No GNU `gettext` `.mo` catalog reader, `gettext`/`ngettext`/`bindtextdomain` API |
| `locale` | `tt::i18n::Locale` | ✅ | 100% | `setlocale`, `localeconv`, `LC_*` categories, `format_string`, `strxfrm` |

## Development Tools

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `dis` | — | ❌ | 0% | No bytecode disassembler exposed to programs. Titrate bytecode is internal to `trc` |
| `doctest` | — | ❌ | 0% | No docstring example runner. Titrate uses `tt::assay::Assay` for testing |
| `py_compile` | — | ❌ | 0% | Python-specific `.pyc` compiler; not applicable. Titrate uses `pipette build` |
| `pydoc` | — | ❌ | 0% | No HTML/repl doc generator. Titrate uses VitePress docs in `docs/` |
| `test` | — | ❌ | 0% | Python's internal regression test suite. Titrate's regression suite lives in `stdlib_test/`/`mega_test*` and is not a public module |
| `token` | — | ❌ | 0% | No Python token constants. Titrate tokens are defined in `trc/src/lexer.rs` but not exposed |
| `tokenize` | `tt::nlp::Tokenizer`, `tt::lang::Tokenizer` (NLP) | ⚠️ | 25% | NLP tokenizer exists for natural language. No Titrate source-code tokenizer API for programs |
| `traceback` | `tt::lang::Traceback` | ✅ | 100% | `print_tb`, `format_tb`, `print_exception`, `format_exception`, `extract_tb`, `format_stack` |
| `unittest` | `tt::assay::Assay`, `tt::assay::TestRunner`, `tt::assert::Assert`, `tt::lang::AssayExt`, `tt::lang::AssertExt` | ⚠️ | 65% | `Assay` provides `assertEqual`/`assertTrue`/`expect`/test discovery. Different API than `unittest.TestCase`. No `mock`/`patch` subpackage |

## Debugging & Profiling

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `cProfile` | — | ❌ | 0% | No deterministic profiler. Titrate uses `pipette bench` for benchmarking |
| `faulthandler` | — | ❌ | 0% | No fault handler for VM crashes. Titrate crashes surface as Rust panics in `trc` |
| `pdb` | — | ❌ | 0% | No source-level debugger for `.tr` programs |
| `profile` | — | ❌ | 0% | No pure-Python profiler |
| `pstats` | — | ❌ | 0% | No profiler statistics viewer (depends on `profile`/`cProfile`) |
| `trace` | — | ❌ | 0% | No statement-level tracer |
| `tracemalloc` | — | ❌ | 0% | No memory allocation tracer |

## Miscellaneous

| Python Module | Titrate Module(s) | Status | Coverage % | Notes |
|---------------|-------------------|--------|------------|-------|
| `cmd` | — | ❌ | 0% | No line-oriented command interpreter framework |
| `curses` | — | ❌ | 0% | No curses terminal UI library |
| `distutils` | — | ❌ | 0% | Deprecated in Python 3.10, removed in 3.12. Titrate uses `pipette` for builds and `Titrate.toml` for project config |
| `getpass` | — | ❌ | 0% | No secure terminal password prompt. `tt::sys::Os::urandom` exists but no `getpass`/`getuser` |
| `idlelib` | — | ❌ | 0% | Python IDLE editor; not applicable to Titrate |
| `mailbox` | — | ❌ | 0% | No mailbox (mbox/Maildir/MH) reader/writer |
| `readline` | — | ❌ | 0% | No GNU readline bindings |
| `rlcompleter` | — | ❌ | 0% | No readline completion (depends on `readline`) |
| `site` | — | ❌ | 0% | No site-packages initialization hook; Titrate uses module paths in `Titrate.toml` |
| `tabnanny` | — | ❌ | 0% | No indentation checker. `pipette lint` covers static analysis |
| `tkinter` | — | ❌ | 0% | No Tcl/Tk GUI bindings |
| `turtle` | — | ❌ | 0% | No turtle graphics |
| `turtledemo` | — | ❌ | 0% | No turtle demo suite (depends on `turtle`) |
| `venv` | — | ❌ | 0% | No virtual environment creator. Titrate projects are self-contained via `Titrate.toml` |
| `wsgiref` | — | ❌ | 0% | No WSGI reference server/gateway (Python web standard) |
| `xmlrpc` | — | ❌ | 0% | No XML-RPC client/server |

---

## Methodology

1. Enumerated every Python 3.12 standard library module from the official Python 3.12 documentation index (181 top-level modules).
2. For each module, searched the Titrate codebase at `lib/tt/` using `Glob` and `Grep` to identify any `.tr` file whose name or public API corresponds to the Python module's surface area.
3. Assessed parity by comparing the documented Python 3.12 API surface against the public `public fn`/`public class` declarations in the corresponding Titrate module(s).
4. Assigned ✅ when all major functions/classes have direct Titrate equivalents, ⚠️ when some are missing, and ❌ when no Titrate module exists.
5. For each ⚠️ and ❌ entry, recorded the specific gap in the Notes column.

This matrix is the baseline for Task G.2 (Close Python stdlib parity gaps). Updates to the matrix after G.2 should change ⚠️/❌ entries to ✅ as gaps are closed.
