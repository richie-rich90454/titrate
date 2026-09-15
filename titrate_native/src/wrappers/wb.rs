use super::native_wrapper;
use crate::TitrateValue;

#[no_mangle]
pub unsafe extern "C" fn titrate_String_charAt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_charAt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_md5(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_md5", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha1(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha1", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha384(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha384", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha512(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha512", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_384(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_384", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_512(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_512", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_blake2b(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_blake2b", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_blake2s(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_blake2s", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_crc32(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_crc32", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha224(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha224", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_224(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_224", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_shake128(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_shake128", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_shake256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_shake256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_update(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_update", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_digest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_digest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_hexDigest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_hexDigest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_reset(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_reset", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hmac_compareDigest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hmac_compareDigest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Base64_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Base64_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Base64_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Base64_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hex_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hex_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hex_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hex_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Url_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Url_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Url_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Url_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_nextUp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_nextUp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_nextDown(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_nextDown", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_ulp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_ulp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_getExponent(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_getExponent", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_scalb(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_scalb", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_random(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_random", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_negInf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_negInf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_fma(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_fma", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_groupCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_groupCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findGroups(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findGroups", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findWithFlags(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findWithFlags", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_matchWithFlags(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_matchWithFlags", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_walk(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_walk", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_copy(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_copy", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_move(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_move", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_dayOfWeek(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_dayOfWeek", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_dayOfYear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_dayOfYear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_monotonic(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_monotonic", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_perfCounter(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_perfCounter", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_epochSeconds(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_epochSeconds", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_nanos(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_nanos", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_millis(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_millis", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_fullMatch(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_fullMatch", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_subN(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_subN", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_parseDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_parseDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_parse(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_parse", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Long_parseLong(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Long_parseLong", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_TypeName_of(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("TypeName_of", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_run(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_run", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_exec(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_exec", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_popenWrite(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_popenWrite", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_runFull(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_runFull", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_runWithInput(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_runWithInput", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_runWithTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_runWithTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Tempfile_create(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Tempfile_create", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_spawn(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_spawn", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_spawnRunnable(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_spawnRunnable", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_join(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_join", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_sleep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_sleep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_yield(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_yield", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_getId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_getId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_currentId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_currentId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_detach(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_detach", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_lock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_lock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_unlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_unlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_tryLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_tryLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_lock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_lock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_unlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_unlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_tryLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_tryLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_sharedLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_sharedLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_sharedUnlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_sharedUnlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_uniqueLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_uniqueLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_uniqueUnlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_uniqueUnlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_trySharedLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_trySharedLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_tryUniqueLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_tryUniqueLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_OnceFlag_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("OnceFlag_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_OnceFlag_callOnce(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("OnceFlag_callOnce", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_wait(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_wait", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_waitFor(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_waitFor", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_notifyOne(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_notifyOne", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_notifyAll(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_notifyAll", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_acquire(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_acquire", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_release(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_release", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_tryAcquire(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_tryAcquire", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_availablePermits(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_availablePermits", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchAdd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchAdd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchSub(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchSub", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchOr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchOr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchAnd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchAnd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchXor(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchXor", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_exchange(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_exchange", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_fetchAdd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_fetchAdd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_fetchSub(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_fetchSub", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_connect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_connect", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_bind(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_bind", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_listen(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_listen", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_accept(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_accept", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_recv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_recv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_close", args, arg_count) }
}
