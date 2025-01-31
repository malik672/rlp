pub mod header;
pub mod error;
pub mod encode;
pub mod decode;


#[inline(always)]
pub fn copy_from_slice<T>(dst: &mut [T], src: &[T]) 
where
    T: Copy,
{
    assert_eq!(dst.len(), src.len(), "source and destination slices must have equal lengths");
    unsafe {
        let mut src_ptr = src.as_ptr();
        let mut dst_ptr = dst.as_mut_ptr();
        let end_ptr = src_ptr.add(src.len());
        
        while src_ptr < end_ptr {
            *dst_ptr = *src_ptr;
            src_ptr = src_ptr.add(1);
            dst_ptr = dst_ptr.add(1);
        }
    }
}