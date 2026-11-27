use super::native_wrapper;
use crate::TitrateValue;

#[no_mangle]
pub unsafe extern "C" fn titrate_StructExt_pack(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("StructExt_pack", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_StructExt_unpack(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("StructExt_unpack", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_StructExt_packInto(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("StructExt_packInto", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_StructExt_unpackFrom(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("StructExt_unpackFrom", args, arg_count) }
}
