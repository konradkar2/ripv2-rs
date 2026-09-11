use std::io;

pub fn ifc_nametoindex(if_name: &str) -> io::Result<u32> {
    if if_name.len() >= (libc::IFNAMSIZ - 1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid interface name: {}", if_name),
        ));
    }

    let mut buffer: Vec<libc::c_char> = vec![0 as libc::c_char; libc::IFNAMSIZ];

    for (dst, src) in buffer.iter_mut().zip(if_name.as_bytes()) {
        *dst = *src as libc::c_char;
    }

    let if_index = unsafe { libc::if_nametoindex(buffer.as_ptr()) };
    if if_index == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(if_index)
}
