/// Convert an array of bits into an integer
pub fn int_list_to_num(int_list: &[u8]) -> u32 {
    let mut flags = 0;
    for flag in int_list {
        flags |= 1 << flag;
    }
    flags
}