/*
 * titrate_native.h - C-ABI native runtime bridge for the Titrate LLVM backend.
 *
 * Declares every pub extern "C" function exported by the titrate_native
 * Rust crate:
 *   - 9 direct helpers in src/lib.rs (println, malloc/free, native_call, ...)
 *   - 358 per-function wrappers in src/wrappers.rs (titrate_<NativeName>)
 *   - 367 exported functions in total
 *
 * Hand-written. Update when wrappers.rs changes.
 * See scripts/gen-header.ps1 for the generator.
 */
#ifndef TITRATE_NATIVE_H
#define TITRATE_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Type tags for the TitrateValue tagged union                        */
/* ------------------------------------------------------------------ */

#define TV_VOID            0
#define TV_NULL            1
#define TV_BOOL            2
#define TV_BYTE            3
#define TV_SHORT           4
#define TV_INT             5
#define TV_LONG            6
#define TV_VAST            7
#define TV_UVAST           8
#define TV_FLOAT           9
#define TV_DOUBLE         10
#define TV_CHAR           11
#define TV_STRING         12
#define TV_ARRAY          13
#define TV_CLASS_INSTANCE 14
#define TV_RESULT_OK      15
#define TV_RESULT_ERR     16
#define TV_ENUM_INSTANCE  17
#define TV_TUPLE          18
#define TV_HALF           19
#define TV_QUAD           20
#define TV_HANDLE         21

/* ------------------------------------------------------------------ */
/* C-ABI value types                                                  */
/* ------------------------------------------------------------------ */

/* UTF-8 string: length + pointer (buffer is NOT NUL-terminated). */
typedef struct {
    int64_t  len;
    uint8_t *ptr;
} TitrateString;

/* Array of TitrateValue: length + heap-allocated data pointer. */
typedef struct {
    int64_t              len;
    struct TitrateValue *data;
} TitrateArray;

/* Opaque handle for complex values (class instances, file handles, etc.). */
typedef struct {
    int64_t id;
    int32_t type_tag;
} TitrateHandle;

/* 16-byte payload union. raw[16] fixes the size at 16 bytes regardless of
 * whether the compiler supports __int128 (MSVC does not). */
typedef union {
    int8_t          bool_val;
    int8_t          byte_val;
    int16_t         short_val;
    int32_t         int_val;
    int64_t         long_val;
#if defined(__SIZEOF_INT128__) || defined(__GNUC__) || defined(__clang__)
    __int128            vast_val;
    unsigned __int128   uvast_val;
#endif
    float           float_val;
    double          double_val;
    uint32_t        char_val;
    TitrateString   string;
    TitrateArray    array;
    TitrateHandle   handle;
    uint8_t         raw[16];
} TitratePayload;

/* The canonical C-ABI value: tag + 16-byte payload (24 bytes total). */
typedef struct TitrateValue {
    int32_t        tag;
    int32_t        _pad;
    TitratePayload payload;
} TitrateValue;

/* ------------------------------------------------------------------ */
/* Direct C-ABI helpers (from src/lib.rs)                              */
/* These have distinct, non-uniform signatures.                       */
/* ------------------------------------------------------------------ */

/* Print a UTF-8 string followed by a newline. */
void titrate_println(int64_t len, const uint8_t *ptr);

/* Concatenate two strings into a freshly allocated buffer. */
uint8_t *titrate_string_concat(int64_t a_len, const uint8_t *a_ptr,
                               int64_t b_len, const uint8_t *b_ptr,
                               int64_t *out_len);

/* Free a buffer returned by titrate_string_concat or titrate_malloc. */
void titrate_free(uint8_t *ptr);

/* Allocate size bytes of heap memory. */
uint8_t *titrate_malloc(int64_t size);

/* Primitive printers. */
void titrate_println_int(int64_t v);
void titrate_println_double(double v);
void titrate_println_bool(int32_t v);
void titrate_println_char(int32_t v);

/* Generic native dispatch bridge: invoke a VM native function by name. */
int32_t titrate_native_call(const uint8_t *name_ptr, int64_t name_len,
                            const uint8_t *args_ptr, int64_t args_count,
                            uint8_t *result_ptr, int64_t *result_cap);

/* ------------------------------------------------------------------ */
/* Per-function C-ABI wrappers (from src/wrappers.rs)                 */
/* Uniform signature:                                                 */
/*   TitrateValue titrate_<Name>(const TitrateValue* args,            */
/*                               size_t arg_count);                   */
/* 358 wrappers in wrappers.rs + 9 direct helpers in lib.rs = 367 total exports. */
/* (was incorrectly documented as "359 wrappers" in the previous revision) */
/* ------------------------------------------------------------------ */

TitrateValue titrate_AtomicBool_compareAndSwap(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicBool_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicBool_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicBool_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_compareAndSwap(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_exchange(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_fetchAdd(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_fetchAnd(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_fetchOr(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_fetchSub(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_fetchXor(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicInt_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_compareAndSwap(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_fetchAdd(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_fetchSub(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicLong_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicRef_compareAndSwap(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicRef_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicRef_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_AtomicRef_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Base64_decode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Base64_encode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_CondVar_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_CondVar_notifyAll(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_CondVar_notifyOne(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_CondVar_wait(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_CondVar_waitFor(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_copy(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_create(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_list(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_move(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_remove(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_removeTree(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dir_walk(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dns_lookup(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Dns_reverseLookup(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Double_parse(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Double_parseDouble(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Env_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Env_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Env_vars(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Err(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_copy(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_delete(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_flush(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_lastModified(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_open(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_readBytes(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_readFile(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_readLine(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_readLines(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_seek(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_setModified(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_size(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_tell(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_truncate(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_write(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_writeBytes(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_File_writeFile(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_exists(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_freeSpace(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_isDir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_isFile(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_size(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Fs_totalSpace(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Gc_collect(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Gzip_compress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Gzip_decompress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_blake2b(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_blake2s(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_crc32(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_md5(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha1(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha224(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha256(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha3_224(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha3_256(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha3_384(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha3_512(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha384(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_sha512(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_shake128(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hash_shake256(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_digest(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_hexDigest(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_reset(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hasher_update(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hex_decode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hex_encode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Hmac_compareDigest(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_delete(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_head(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_patch(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_post(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_put(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_setFollowRedirects(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Http_setTimeout(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Integer_parseOr(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Json_parse(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Json_stringify(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Long_parseLong(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_abs(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_absInt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_acos(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_asin(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_atan(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_atan2(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_cbrt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_ceil(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_cos(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_exp(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_floor(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_fma(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_getExponent(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_inf(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_ln(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_log10(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_log2(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_maxDouble(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_maxInt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_minDouble(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_minInt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_nan(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_negInf(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_nextDown(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_nextUp(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_pow(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_random(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_round(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_scalb(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_sin(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_sqrt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_tan(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Math_ulp(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_flush(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_get(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_open(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_set(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mmap_size(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mutex_lock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mutex_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mutex_tryLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Mutex_unlock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_accept(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_bind(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_connect(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_getLocalAddress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_getLocalPort(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_getRemoteAddress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_receive(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_send(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Net_setTimeout(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ok(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_OnceFlag_callOnce(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_OnceFlag_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_access(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_arch(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_chdir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_chmod(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_cpuCount(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_environ(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_environMap(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_family(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_getcwd(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_getenv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_getpid(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_getppid(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_hostName(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_kill(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_link(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_lstat(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_makedirs(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_name(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_readlink(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_release(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_removedirs(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_renames(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_replace(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_scandir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_setenv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_strerror(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_symlink(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_system(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_umask(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_uname(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_unsetenv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_urandom(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_userName(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_utime(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Os_version(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_parseInt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_basename(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_dirname(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_exists(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_extension(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_isDir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_isFile(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_isSymlink(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Path_join(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Process_args(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Process_id(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Random_nextLong(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Random_seed(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_RecursiveMutex_lock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_RecursiveMutex_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_RecursiveMutex_tryLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_RecursiveMutex_unlock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_find(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_findGroups(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_findWithFlags(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_fullMatch(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_groupCount(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_match(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_matchWithFlags(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_replace(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Regex_subN(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Semaphore_acquire(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Semaphore_availablePermits(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Semaphore_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Semaphore_release(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Semaphore_tryAcquire(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_sharedLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_sharedUnlock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_trySharedLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_tryUniqueLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_uniqueLock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_SharedMutex_uniqueUnlock(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Signal_raise(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Signal_register(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_accept(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_bind(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_connect(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_createConnection(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_createServer(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_getAddrInfo(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_getLocalAddress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_getLocalPort(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_getRemoteAddress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_getRemotePort(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_inetNtop(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_inetPton(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_listen(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_recv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_send(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setBroadcast(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setKeepAlive(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setLinger(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setNoDelay(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setReuseAddr(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Socket_setTimeout(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_backup(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_closeResult(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_columnCount(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_columnName(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_execute(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_executePrepared(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_getDouble(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_getInt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_getString(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_lastInsertId(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_nextRow(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_open(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sqlite_query(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_connect(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_contextClose(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_contextNew(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_getPeerCertHash(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_peerCertificate(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_recv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Ssl_send(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_charAt(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_endsWith(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_fromCharCode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_length(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_padLeft(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_padRight(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_replace(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_split(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_startsWith(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_toLowerCase(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_toUpperCase(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_trim(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_trimEnd(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_String_trimStart(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Subprocess_exec(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Subprocess_popenWrite(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Subprocess_run(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_args(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_env(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_exit(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_setEnv(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_setWorkingDir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_sleep(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Sys_workingDir(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Tempfile_create(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_currentId(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_detach(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_getId(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_join(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_sleep(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_spawn(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_spawnRunnable(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Thread_yield(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_dayOfWeek(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_dayOfYear(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_epochSeconds(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_format(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getDay(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getHour(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getMinute(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getMonth(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getSecond(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_getYear(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_millis(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_monotonic(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_nanos(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_now(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_perfCounter(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Time_sleep(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Titrate_version(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_toString(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_TypeName_of(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_bind(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_lastSenderHost(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_lastSenderPort(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_new(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_recvFrom(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_sendTo(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_UdpSocket_setTimeout(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Url_decode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Url_encode(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_entryCount(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_entryName(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_extractAll(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_open(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipFile_readEntry(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipWriter_addEntry(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipWriter_close(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_ZipWriter_open(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Zlib_compress(const TitrateValue *args, size_t arg_count);
TitrateValue titrate_Zlib_decompress(const TitrateValue *args, size_t arg_count);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* TITRATE_NATIVE_H */