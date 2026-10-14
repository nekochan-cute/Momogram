pub(crate) fn read_i32_at(data: &[u8], pos: &mut usize) -> Result<i32, String> {
    let bytes: [u8; 4] = take(data, pos, 4)?
        .try_into()
        .map_err(|_| "Invalid int32 width in backup file".to_owned())?;
    Ok(i32::from_le_bytes(bytes))
}

pub(crate) fn take<'a>(data: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let end = (*pos)
        .checked_add(len)
        .ok_or_else(|| "Backup length overflow".to_owned())?;
    if end > data.len() {
        return Err("Unexpected end of backup file".to_owned());
    }
    let out = &data[*pos..end];
    *pos = end;
    Ok(out)
}
