use super::native_wrapper;
use crate::TitrateValue;

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setNoDelay(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setNoDelay", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_bind(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_bind", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_sendTo(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_sendTo", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_recvFrom(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_recvFrom", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_lastSenderHost(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_lastSenderHost", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_lastSenderPort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_lastSenderPort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getAddrInfo(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getAddrInfo", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_inetPton(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_inetPton", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_inetNtop(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_inetNtop", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_createConnection(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_createConnection", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_createServer(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_createServer", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getLocalAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getLocalAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getRemoteAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getRemoteAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getLocalPort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getLocalPort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getRemotePort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getRemotePort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setReuseAddr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setReuseAddr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setBroadcast(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setBroadcast", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setKeepAlive(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setKeepAlive", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setLinger(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setLinger", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_contextNew(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_contextNew", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_connect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_connect", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_recv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_recv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_peerCertificate(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_peerCertificate", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_contextClose(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_contextClose", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_getPeerCertHash(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_getPeerCertHash", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_execute(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_execute", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_query(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_query", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_lastInsertId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_lastInsertId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_nextRow(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_nextRow", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_columnCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_columnCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_columnName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_columnName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_closeResult(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_closeResult", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_executePrepared(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_executePrepared", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_backup(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_backup", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_flush(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_flush", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Signal_register(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Signal_register", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Signal_raise(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Signal_raise", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zlib_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zlib_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zlib_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zlib_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gzip_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gzip_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gzip_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gzip_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_entryCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_entryCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_entryName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_entryName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_readEntry(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_readEntry", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_extractAll(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_extractAll", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_addEntry(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_addEntry", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_cpuCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_cpuCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_userName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_userName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_hostName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_hostName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_urandom(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_urandom", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_chmod(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_chmod", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_makedirs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_makedirs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_symlink(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_symlink", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_readlink(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_readlink", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_kill(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_kill", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_killSignal(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_killSignal", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_environ(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_environ", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_umask(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_umask", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_scandir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_scandir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_environMap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_environMap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getpid(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getpid", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getcwd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getcwd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_chdir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_chdir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_setenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_setenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_unsetenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_unsetenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_system(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_system", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_uname(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_uname", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getppid(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getppid", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_strerror(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_strerror", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_removedirs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_removedirs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_renames(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_renames", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_replace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_replace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_link(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_link", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_utime(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_utime", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_lstat(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_lstat", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_access(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_access", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_release(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_release", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_version(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_version", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Titrate_version(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Titrate_version", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gc_collect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gc_collect", args, arg_count) }
}

// ---------------------------------------------------------------------------
// ArrayList native wrappers (for LLVM backend)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_add(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_add", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_remove(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_remove", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_removeAt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_removeAt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_contains(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_contains", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_indexOf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_indexOf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_isEmpty(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_isEmpty", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_clear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_clear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_toString", args, arg_count) }
}

// ---------------------------------------------------------------------------
// HashMap native wrappers (for LLVM backend)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_put(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_put", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_containsKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_containsKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_containsValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_containsValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_remove(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_remove", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_keys(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_keys", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_values(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_values", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_isEmpty(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_isEmpty", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_clear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_clear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Integer_parseInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Integer_parseInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_indexOf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_indexOf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_substring(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_substring", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Boolean_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Boolean_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Integer_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Integer_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Long_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Long_toString", args, arg_count) }
}
